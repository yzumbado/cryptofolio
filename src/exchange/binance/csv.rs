/// Generic Binance CSV export parser.
///
/// Supports all seven Binance export formats, auto-detected from column headers.
/// Accepts raw CSV files or ZIP archives (Binance wraps exports in a single-file ZIP).
///
/// Every row becomes a normalized `BinanceCsvRow` which the import command converts
/// to a `Transaction` and writes to the immutable ledger.
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;
use std::str::FromStr;

use crate::core::transaction::TransactionType;
use crate::error::{CryptofolioError, Result};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum BinanceExportFormat {
    /// `User ID, Time, Account, Operation, Coin, Change, Remark`
    TransactionHistory,
    /// `Time, Coin, Network, Amount, Fee, Address, TXID, Status`
    WithdrawHistory,
    /// `Time, Coin, Network, Amount, Address, TXID, Status`
    DepositHistory,
    /// `Time, Pair, Side, Price, Executed, Amount, Fee`
    SpotTradeHistory,
    /// `Time, OrderNo, Pair, Type, Side, Order Price, Order Amount, Time, Executed, Average Price, Trading total, Status`
    SpotOrderHistory,
    /// `Time, OrderNo, Type, Direction, Base Asset, Quote Asset, AvgTrading Price, Filled, Total, Status, Slippage, Network Fee`
    AlphaOrderHistory,
}

/// Normalized row produced by any parser variant.
#[derive(Debug, Clone)]
pub struct BinanceCsvRow {
    pub timestamp: DateTime<Utc>,
    pub tx_type: TransactionType,

    pub from_asset: Option<String>,
    pub from_quantity: Option<Decimal>,

    pub to_asset: Option<String>,
    pub to_quantity: Option<Decimal>,

    pub fee: Option<Decimal>,
    pub fee_asset: Option<String>,

    /// Execution price per unit of the traded asset, denominated in `price_asset`.
    /// Present for trade/order rows — the cost-basis input P&L depends on.
    pub price: Option<Decimal>,
    /// Currency the `price` is quoted in (the quote asset of the pair, e.g. USDC).
    pub price_asset: Option<String>,

    /// On-chain tx hash — present for Withdraw and Deposit history rows.
    pub tx_hash: Option<String>,
    /// Destination/source address — present for Withdraw and Deposit history rows.
    pub address: Option<String>,
    /// On-chain network as reported by Binance (e.g. "ETH", "BSC", "TRX").
    /// Present for Withdraw and Deposit history rows; the authoritative chain hint.
    pub network: Option<String>,

    /// Unique key for DB deduplication (`UNIQUE(external_id)` constraint).
    pub external_id: String,

    pub source: &'static str,
    pub trust_level: &'static str,
    pub notes: Option<String>,
}

/// A row that could not be imported, with the reason why. The ledger must never
/// lose financial records silently, so every malformed or unrecognised row is
/// surfaced here rather than dropped.
#[derive(Debug, Clone)]
pub struct SkippedRow {
    /// 1-based line number in the CSV (header counted), best-effort.
    pub line: usize,
    pub reason: String,
}

/// Result of parsing an export: the importable rows plus an explicit account of
/// everything that was skipped and why. Callers MUST surface `skipped`.
#[derive(Debug, Clone, Default)]
pub struct ParseReport {
    pub rows: Vec<BinanceCsvRow>,
    pub skipped: Vec<SkippedRow>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Parse a file (`.csv` or `.zip`) and return all normalised rows.
///
/// Format is auto-detected from column headers. Completed/failed rows from
/// Withdraw and Deposit history are both returned; callers should filter on
/// `notes` or add a `status` field if needed.
pub fn parse_file(path: &Path) -> Result<ParseReport> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "zip" => parse_zip(path),
        "csv" => {
            let content = std::fs::read(path)?;
            parse_bytes(&content)
        }
        other => Err(CryptofolioError::Config(format!(
            "Unsupported file extension '.{}'. Pass a .csv or .zip Binance export.",
            other
        ))),
    }
}

fn parse_zip(path: &Path) -> Result<ParseReport> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CryptofolioError::Config(format!("Cannot open ZIP: {}", e)))?;

    let mut report = ParseReport::default();
    let mut saw_csv = false;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CryptofolioError::Config(format!("ZIP read error: {}", e)))?;

        let name = entry.name().to_lowercase();
        if !name.ends_with(".csv") {
            continue;
        }
        saw_csv = true;

        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        let sub = parse_bytes(&buf)?;
        report.rows.extend(sub.rows);
        report.skipped.extend(sub.skipped);
    }

    if !saw_csv {
        return Err(CryptofolioError::Config(
            "ZIP contains no CSV files with recognisable Binance headers.".into(),
        ));
    }

    Ok(report)
}

fn parse_bytes(bytes: &[u8]) -> Result<ParseReport> {
    // Strip UTF-8 BOM (Binance exports include \xEF\xBB\xBF)
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(bytes));

    let headers = reader
        .headers()
        .map_err(CryptofolioError::Csv)?
        .clone()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let format = detect_format(&headers).ok_or_else(|| {
        CryptofolioError::Config(format!(
            "Unrecognised Binance CSV headers: {:?}. \
             Supported exports: Transaction History, Withdraw History, Deposit History, \
             Spot Trade History, Spot Order History, Alpha Order History.",
            &headers[..headers.len().min(6)]
        ))
    })?;

    let mut report = ParseReport::default();
    for (idx, result) in reader.records().enumerate() {
        let line = idx + 2; // +1 for header row, +1 for 1-based numbering
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                report.skipped.push(SkippedRow {
                    line,
                    reason: format!("malformed CSV record: {}", e),
                });
                continue;
            }
        };
        match parse_record(&record, &format) {
            Ok(Some(row)) => report.rows.push(row),
            // Intentionally filtered (e.g. non-Completed withdrawal/deposit status).
            Ok(None) => {}
            // Malformed or unrecognised — never dropped silently; surfaced with a reason.
            Err(e) => report.skipped.push(SkippedRow {
                line,
                reason: e.to_string(),
            }),
        }
    }

    if matches!(format, BinanceExportFormat::TransactionHistory) {
        pair_tx_history_trades(&mut report.rows);
    }
    disambiguate_external_ids(&mut report.rows);
    Ok(report)
}

/// Make `external_id`s unique within a file. Transaction-History rows have no
/// source ID, so their id is a content hash of (time, coin, change) — which
/// legitimately collides when, e.g., two identical interest payments land in the
/// same second. Left alone, the `UNIQUE(external_id)` constraint would silently
/// drop one (data loss). Here the first occurrence keeps its id and each later
/// duplicate gets a stable `#N` suffix, so distinct rows all survive. Re-importing
/// the same file yields the same suffixes, so dedup stays idempotent.
fn disambiguate_external_ids(rows: &mut [BinanceCsvRow]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for row in rows.iter_mut() {
        let count = seen.entry(row.external_id.clone()).or_insert(0);
        if *count > 0 {
            row.external_id = format!("{}#{}", row.external_id, count);
        }
        *count += 1;
    }
}

/// Pair Transaction-History trade legs into single priced buy/sell rows.
///
/// The Transaction History export records a spot trade as TWO rows at the same
/// timestamp: the asset leg (`Transaction Buy` +COIN / `Transaction Sold` −COIN)
/// and the quote leg (`Transaction Spend` −USDT / `Transaction Revenue` +USDT).
/// Parsed row-by-row, the asset leg has NO price (cost basis unknown) and the
/// quote leg is a stray transfer. This post-pass matches them by timestamp and
/// rewrites each trade into one row carrying the real execution price
/// (quote_amount / coin_qty), dropping the now-redundant quote leg.
///
/// Without this, importing Transaction History gives priceless buys (zero cost
/// basis) and orphaned sells (no proceeds) — FIFO then produces garbage.
fn pair_tx_history_trades(rows: &mut Vec<BinanceCsvRow>) {
    use std::collections::HashMap;
    const STABLES: [&str; 5] = ["USDT", "USDC", "USD", "BUSD", "FDUSD"];
    let is_stable = |a: &str| STABLES.contains(&a);

    // Group row indices by exact timestamp.
    let mut by_ts: HashMap<DateTime<Utc>, Vec<usize>> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        by_ts.entry(r.timestamp).or_default().push(i);
    }

    let mut drop_idx: Vec<usize> = Vec::new();

    for (_ts, idxs) in by_ts.iter() {
        // Within this timestamp, sum the quote legs and collect the coin legs.
        // A coin BUY leg: tx_type==Buy, to_asset is a non-stable coin, price None.
        // Its quote: tx_type==TransferOut (Transaction Spend), from_asset stable.
        // A coin SELL leg: tx_type==Sell, from_asset non-stable; quote TransferIn.
        let mut quote_spend = Decimal::ZERO; // stable leaving (funds a buy)
        let mut quote_revenue = Decimal::ZERO; // stable arriving (from a sell)
        let mut quote_asset: Option<String> = None;
        let mut spend_idx: Vec<usize> = Vec::new();
        let mut revenue_idx: Vec<usize> = Vec::new();
        let mut buy_legs: Vec<usize> = Vec::new();
        let mut sell_legs: Vec<usize> = Vec::new();

        for &i in idxs {
            let r = &rows[i];
            let note = r.notes.as_deref().unwrap_or("");
            match r.tx_type {
                TransactionType::Buy
                    if r.price.is_none()
                        && r.to_asset
                            .as_deref()
                            .map(|a| !is_stable(a))
                            .unwrap_or(false)
                        && note.starts_with("Transaction Buy") =>
                {
                    buy_legs.push(i);
                }
                TransactionType::Sell
                    if r.price.is_none()
                        && r.from_asset
                            .as_deref()
                            .map(|a| !is_stable(a))
                            .unwrap_or(false) =>
                {
                    sell_legs.push(i);
                }
                TransactionType::TransferOut
                    if r.from_asset.as_deref().map(is_stable).unwrap_or(false)
                        && note.starts_with("Transaction Spend") =>
                {
                    quote_spend += r.from_quantity.unwrap_or(Decimal::ZERO);
                    quote_asset = r.from_asset.clone();
                    spend_idx.push(i);
                }
                TransactionType::TransferIn
                    if r.to_asset.as_deref().map(is_stable).unwrap_or(false)
                        && note.starts_with("Transaction Revenue") =>
                {
                    quote_revenue += r.to_quantity.unwrap_or(Decimal::ZERO);
                    quote_asset = r.to_asset.clone();
                    revenue_idx.push(i);
                }
                _ => {}
            }
        }

        // Price BUY legs from the spend total (distributed by qty share).
        let buy_qty: Decimal = buy_legs.iter().filter_map(|&i| rows[i].to_quantity).sum();
        if !buy_legs.is_empty() && quote_spend > Decimal::ZERO && buy_qty > Decimal::ZERO {
            for &i in &buy_legs {
                let q = rows[i].to_quantity.unwrap_or(Decimal::ZERO);
                if q > Decimal::ZERO {
                    let cost = quote_spend * (q / buy_qty);
                    rows[i].price = Some(cost / q);
                    rows[i].price_asset = quote_asset.clone();
                    rows[i].from_asset = quote_asset.clone();
                    rows[i].from_quantity = Some(cost);
                }
            }
            drop_idx.extend(&spend_idx); // quote leg folded into the buy
        }

        // Price SELL legs from the revenue total.
        let sell_qty: Decimal = sell_legs
            .iter()
            .filter_map(|&i| rows[i].from_quantity)
            .sum();
        if !sell_legs.is_empty() && quote_revenue > Decimal::ZERO && sell_qty > Decimal::ZERO {
            for &i in &sell_legs {
                let q = rows[i].from_quantity.unwrap_or(Decimal::ZERO);
                if q > Decimal::ZERO {
                    let proceeds = quote_revenue * (q / sell_qty);
                    rows[i].price = Some(proceeds / q);
                    rows[i].price_asset = quote_asset.clone();
                    rows[i].to_asset = quote_asset.clone();
                    rows[i].to_quantity = Some(proceeds);
                }
            }
            drop_idx.extend(&revenue_idx); // quote leg folded into the sell
        }

        // --- Binance Convert pairing ---
        // A Convert is two Swap legs at the same timestamp: a coin leg and a
        // stable leg. coin-IN + stable-OUT = a BUY of coin with stable; coin-OUT
        // + stable-IN = a SELL. Rewrite the coin leg into a priced buy/sell and
        // drop the stable leg, so converts contribute cost basis / proceeds just
        // like spot trades (previously they were untyped Swaps with no price).
        let mut conv_stable_out = Decimal::ZERO; // stable leaving -> funds a buy
        let mut conv_stable_in = Decimal::ZERO; // stable arriving -> from a sell
        let mut conv_stable_asset: Option<String> = None;
        let mut conv_stable_idx: Vec<usize> = Vec::new();
        let mut conv_coin_buy: Vec<usize> = Vec::new(); // Swap, to_asset = coin
        let mut conv_coin_sell: Vec<usize> = Vec::new(); // Swap, from_asset = coin
        for &i in idxs {
            let r = &rows[i];
            if !matches!(r.tx_type, TransactionType::Swap) {
                continue;
            }
            let note = r.notes.as_deref().unwrap_or("");
            if !note.starts_with("Binance Convert") {
                continue;
            }
            if let Some(a) = r.to_asset.as_deref() {
                if is_stable(a) {
                    conv_stable_in += r.to_quantity.unwrap_or(Decimal::ZERO);
                    conv_stable_asset = r.to_asset.clone();
                    conv_stable_idx.push(i);
                } else {
                    conv_coin_buy.push(i);
                }
            } else if let Some(a) = r.from_asset.as_deref() {
                if is_stable(a) {
                    conv_stable_out += r.from_quantity.unwrap_or(Decimal::ZERO);
                    conv_stable_asset = r.from_asset.clone();
                    conv_stable_idx.push(i);
                } else {
                    conv_coin_sell.push(i);
                }
            }
        }
        // Convert BUY: coin in, stable out -> priced buy.
        let cbq: Decimal = conv_coin_buy
            .iter()
            .filter_map(|&i| rows[i].to_quantity)
            .sum();
        if !conv_coin_buy.is_empty() && conv_stable_out > Decimal::ZERO && cbq > Decimal::ZERO {
            for &i in &conv_coin_buy {
                let q = rows[i].to_quantity.unwrap_or(Decimal::ZERO);
                if q > Decimal::ZERO {
                    let cost = conv_stable_out * (q / cbq);
                    rows[i].tx_type = TransactionType::Buy;
                    rows[i].price = Some(cost / q);
                    rows[i].price_asset = conv_stable_asset.clone();
                    rows[i].from_asset = conv_stable_asset.clone();
                    rows[i].from_quantity = Some(cost);
                }
            }
            drop_idx.extend(&conv_stable_idx);
        }
        // Convert SELL: coin out, stable in -> priced sell.
        let csq: Decimal = conv_coin_sell
            .iter()
            .filter_map(|&i| rows[i].from_quantity)
            .sum();
        if !conv_coin_sell.is_empty() && conv_stable_in > Decimal::ZERO && csq > Decimal::ZERO {
            for &i in &conv_coin_sell {
                let q = rows[i].from_quantity.unwrap_or(Decimal::ZERO);
                if q > Decimal::ZERO {
                    let proceeds = conv_stable_in * (q / csq);
                    rows[i].tx_type = TransactionType::Sell;
                    rows[i].price = Some(proceeds / q);
                    rows[i].price_asset = conv_stable_asset.clone();
                    rows[i].to_asset = conv_stable_asset.clone();
                    rows[i].to_quantity = Some(proceeds);
                }
            }
            drop_idx.extend(&conv_stable_idx);
        }
    }

    // Drop the folded quote legs (highest index first to keep indices valid).
    drop_idx.sort_unstable();
    drop_idx.dedup();
    for &i in drop_idx.iter().rev() {
        rows.remove(i);
    }
}

// ---------------------------------------------------------------------------
// Format detection
// ---------------------------------------------------------------------------

pub fn detect_format(headers: &[String]) -> Option<BinanceExportFormat> {
    let h: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();

    if h.contains(&"Operation") && h.contains(&"Coin") && h.contains(&"Change") {
        return Some(BinanceExportFormat::TransactionHistory);
    }
    if h.contains(&"TXID") && h.contains(&"Fee") && h.contains(&"Address") {
        return Some(BinanceExportFormat::WithdrawHistory);
    }
    if h.contains(&"TXID") && h.contains(&"Address") && !h.contains(&"Fee") {
        return Some(BinanceExportFormat::DepositHistory);
    }
    if h.contains(&"Pair") && h.contains(&"Side") && h.contains(&"Executed") {
        return Some(BinanceExportFormat::SpotTradeHistory);
    }
    if h.contains(&"OrderNo") && h.contains(&"Direction") {
        return Some(BinanceExportFormat::AlphaOrderHistory);
    }
    if h.contains(&"OrderNo") && h.contains(&"Side") {
        return Some(BinanceExportFormat::SpotOrderHistory);
    }

    None
}

// ---------------------------------------------------------------------------
// Per-format parsers
// ---------------------------------------------------------------------------

fn parse_record(
    record: &csv::StringRecord,
    format: &BinanceExportFormat,
) -> Result<Option<BinanceCsvRow>> {
    match format {
        BinanceExportFormat::TransactionHistory => parse_tx_history(record),
        BinanceExportFormat::WithdrawHistory => parse_withdraw(record),
        BinanceExportFormat::DepositHistory => parse_deposit(record),
        BinanceExportFormat::SpotTradeHistory => parse_spot_trade(record),
        BinanceExportFormat::SpotOrderHistory => parse_spot_order(record),
        BinanceExportFormat::AlphaOrderHistory => parse_alpha_order(record),
    }
}

/// `User ID, Time, Account, Operation, Coin, Change, Remark`
fn parse_tx_history(r: &csv::StringRecord) -> Result<Option<BinanceCsvRow>> {
    let time_str = field(r, 1)?;
    let operation = field(r, 3)?;
    let coin = field(r, 4)?.to_uppercase();
    let change_str = field(r, 5)?;
    let remark = r.get(6).unwrap_or("").to_string();

    let timestamp = parse_binance_time(time_str)?;
    let change = parse_decimal_clean(change_str)?;

    let (tx_type, from_asset, from_qty, to_asset, to_qty) =
        classify_tx_history_row(operation, &coin, change)?;

    // Stable dedup key — no row ID in this export, so use content hash
    let external_id = format!(
        "binance-txhist-{}-{}-{}",
        time_str.replace(' ', "T"),
        coin,
        change_str.replace('-', "n")
    );

    let notes = if remark.is_empty() {
        Some(operation.to_string())
    } else {
        Some(format!("{}: {}", operation, remark))
    };

    Ok(Some(BinanceCsvRow {
        timestamp,
        tx_type,
        from_asset,
        from_quantity: from_qty,
        to_asset,
        to_quantity: to_qty,
        fee: None,
        fee_asset: None,
        price: None,
        price_asset: None,
        tx_hash: None,
        address: None,
        network: None,
        external_id,
        source: "binance_csv",
        trust_level: "exchange_verified",
        notes,
    }))
}

/// `Time, Coin, Network, Amount, Fee, Address, TXID, Status`
fn parse_withdraw(r: &csv::StringRecord) -> Result<Option<BinanceCsvRow>> {
    let time_str = field(r, 0)?;
    let coin = field(r, 1)?.to_uppercase();
    let network = field(r, 2)?.to_string();
    let amount_str = field(r, 3)?;
    let fee_str = field(r, 4)?;
    let address = field(r, 5)?.to_string();
    let txid = field(r, 6)?.to_string();
    let status = field(r, 7)?;

    if !status.eq_ignore_ascii_case("Completed") {
        return Ok(None);
    }

    let timestamp = parse_binance_time(time_str)?;
    let amount = parse_decimal_clean(amount_str)?;
    let fee = parse_decimal_clean(fee_str).ok();

    let external_id = if txid.is_empty() {
        format!(
            "binance-withdraw-{}-{}-{}",
            time_str.replace(' ', "T"),
            coin,
            amount_str
        )
    } else {
        format!("binance-withdraw-{}", txid)
    };

    let tx_hash = if txid.is_empty() {
        None
    } else {
        Some(txid.clone())
    };
    let address_opt = if address.is_empty() {
        None
    } else {
        Some(address.clone())
    };

    Ok(Some(BinanceCsvRow {
        timestamp,
        tx_type: TransactionType::TransferOut,
        from_asset: Some(coin.clone()),
        from_quantity: Some(amount),
        to_asset: None,
        to_quantity: None,
        fee,
        fee_asset: fee.map(|_| coin),
        price: None,
        price_asset: None,
        tx_hash,
        address: address_opt,
        network: if network.is_empty() {
            None
        } else {
            Some(network)
        },
        external_id,
        source: "binance_csv",
        trust_level: "exchange_verified",
        notes: Some(format!(
            "Binance withdrawal to {}",
            &address[..address.len().min(20)]
        )),
    }))
}

/// `Time, Coin, Network, Amount, Address, TXID, Status`
fn parse_deposit(r: &csv::StringRecord) -> Result<Option<BinanceCsvRow>> {
    let time_str = field(r, 0)?;
    let coin = field(r, 1)?.to_uppercase();
    let network = field(r, 2)?.to_string();
    let amount_str = field(r, 3)?;
    let address = field(r, 4)?.to_string();
    let txid = field(r, 5)?.to_string();
    let status = field(r, 6)?;

    if !status.eq_ignore_ascii_case("Completed") {
        return Ok(None);
    }

    let timestamp = parse_binance_time(time_str)?;
    let amount = parse_decimal_clean(amount_str)?;

    let external_id = if txid.is_empty() {
        format!(
            "binance-deposit-{}-{}-{}",
            time_str.replace(' ', "T"),
            coin,
            amount_str
        )
    } else {
        format!("binance-deposit-{}", txid)
    };

    let tx_hash = if txid.is_empty() {
        None
    } else {
        Some(txid.clone())
    };
    let address_opt = if address.is_empty() {
        None
    } else {
        Some(address.clone())
    };

    Ok(Some(BinanceCsvRow {
        timestamp,
        tx_type: TransactionType::TransferIn,
        from_asset: None,
        from_quantity: None,
        to_asset: Some(coin),
        to_quantity: Some(amount),
        fee: None,
        fee_asset: None,
        price: None,
        price_asset: None,
        tx_hash,
        address: address_opt,
        network: if network.is_empty() {
            None
        } else {
            Some(network)
        },
        external_id,
        source: "binance_csv",
        trust_level: "exchange_verified",
        notes: None,
    }))
}

/// `Time, Pair, Side, Price, Executed, Amount, Fee`
/// Executed: "9.434SOL"  Amount: "699.43676USDC"  Fee: "0.00081438BNB"
fn parse_spot_trade(r: &csv::StringRecord) -> Result<Option<BinanceCsvRow>> {
    let time_str = field(r, 0)?;
    let pair = field(r, 1)?;
    let side = field(r, 2)?;
    let price_raw = field(r, 3)?;
    let executed_raw = field(r, 4)?;
    let amount_raw = field(r, 5)?;
    let fee_raw = r.get(6).unwrap_or("");

    let timestamp = parse_binance_time(time_str)?;

    let (exec_qty, exec_asset) = split_qty_asset(executed_raw)?;
    let (amount_qty, amount_asset) = split_qty_asset(amount_raw)?;
    let fee_parsed = split_qty_asset(fee_raw).ok();

    // The Price column is quoted in the quote asset (the `Amount` side of the pair).
    // It may be a bare number ("74.12") or carry the asset suffix ("74.12USDC").
    let price = split_qty_asset(price_raw)
        .map(|(qty, _)| qty)
        .or_else(|_| parse_decimal_clean(price_raw))
        .ok();
    let price_asset = Some(amount_asset.clone());

    let (from_asset, from_qty, to_asset, to_qty) = if side.eq_ignore_ascii_case("BUY") {
        (amount_asset, amount_qty, exec_asset, exec_qty)
    } else {
        (exec_asset, exec_qty, amount_asset, amount_qty)
    };

    let external_id = format!(
        "binance-trade-{}-{}-{}",
        time_str.replace(' ', "T"),
        pair,
        executed_raw.replace('.', "p")
    );

    let tx_type = if side.eq_ignore_ascii_case("BUY") {
        TransactionType::Buy
    } else {
        TransactionType::Sell
    };

    Ok(Some(BinanceCsvRow {
        timestamp,
        tx_type,
        from_asset: Some(from_asset),
        from_quantity: Some(from_qty),
        to_asset: Some(to_asset),
        to_quantity: Some(to_qty),
        fee: fee_parsed.as_ref().map(|(qty, _)| *qty),
        fee_asset: fee_parsed.map(|(_, asset)| asset),
        price,
        price_asset,
        tx_hash: None,
        address: None,
        network: None,
        external_id,
        source: "binance_csv",
        trust_level: "exchange_verified",
        notes: Some(format!("{} {}", side, pair)),
    }))
}

/// `Time, OrderNo, Pair, Type¹, Side, Order Price, Order Amount, Time, Executed², Average Price, Trading total³, Status`
fn parse_spot_order(r: &csv::StringRecord) -> Result<Option<BinanceCsvRow>> {
    let time_str = field(r, 0)?;
    let order_no = field(r, 1)?;
    let pair = field(r, 2)?;
    let side = field(r, 4)?;
    let executed_raw = field(r, 8)?;
    let avg_price_raw = field(r, 9)?;
    let total_raw = field(r, 10)?;
    let status = field(r, 11)?;

    if !status.eq_ignore_ascii_case("Filled") {
        return Ok(None);
    }

    let timestamp = parse_binance_time(time_str)?;
    let (exec_qty, exec_asset) = split_qty_asset(executed_raw)?;
    let (total_qty, total_asset) = split_qty_asset(total_raw)?;

    // Average Price is quoted in the quote asset (the `Trading total` side).
    let price = split_qty_asset(avg_price_raw)
        .map(|(qty, _)| qty)
        .or_else(|_| parse_decimal_clean(avg_price_raw))
        .ok();
    let price_asset = Some(total_asset.clone());

    let (from_asset, from_qty, to_asset, to_qty) = if side.eq_ignore_ascii_case("BUY") {
        (total_asset, total_qty, exec_asset, exec_qty)
    } else {
        (exec_asset, exec_qty, total_asset, total_qty)
    };

    let tx_type = if side.eq_ignore_ascii_case("BUY") {
        TransactionType::Buy
    } else {
        TransactionType::Sell
    };

    Ok(Some(BinanceCsvRow {
        timestamp,
        tx_type,
        from_asset: Some(from_asset),
        from_quantity: Some(from_qty),
        to_asset: Some(to_asset),
        to_quantity: Some(to_qty),
        fee: None,
        fee_asset: None,
        price,
        price_asset,
        tx_hash: None,
        address: None,
        network: None,
        external_id: format!("binance-order-{}", order_no),
        source: "binance_csv",
        trust_level: "exchange_verified",
        notes: Some(format!("{} {} (order {})", side, pair, order_no)),
    }))
}

/// `Time, OrderNo, Type, Direction, Base Asset, Quote Asset, AvgTrading Price, Filled, Total, Status, ...`
/// Filled: `"74,651.45 NIGHT"`  Total: `"4,299.97255125 USDT"`
fn parse_alpha_order(r: &csv::StringRecord) -> Result<Option<BinanceCsvRow>> {
    let time_str = field(r, 0)?;
    let order_no = field(r, 1)?;
    let direction = field(r, 3)?;
    let base_asset = field(r, 4)?.to_uppercase();
    let quote_asset = field(r, 5)?.to_uppercase();
    let avg_price_raw = field(r, 6)?;
    let filled_raw = field(r, 7)?;
    let total_raw = field(r, 8)?;
    let status = field(r, 9)?;

    if !status.eq_ignore_ascii_case("FILLED") {
        return Ok(None);
    }

    let timestamp = parse_binance_time(time_str)?;

    // AvgTrading Price is quoted in the quote asset; amounts use comma thousands.
    let price = parse_decimal_clean(&avg_price_raw.replace(',', "")).ok();
    let price_asset = Some(quote_asset.clone());

    // Alpha amounts use comma-separated thousands: "74,651.45 NIGHT"
    let filled_qty = parse_decimal_clean(
        filled_raw
            .replace(',', "")
            .split_whitespace()
            .next()
            .unwrap_or(""),
    )?;
    let total_qty = parse_decimal_clean(
        total_raw
            .replace(',', "")
            .split_whitespace()
            .next()
            .unwrap_or(""),
    )?;

    let (from_asset, from_qty, to_asset, to_qty) = if direction.eq_ignore_ascii_case("BUY") {
        (
            quote_asset.clone(),
            total_qty,
            base_asset.clone(),
            filled_qty,
        )
    } else {
        (
            base_asset.clone(),
            filled_qty,
            quote_asset.clone(),
            total_qty,
        )
    };

    let tx_type = if direction.eq_ignore_ascii_case("BUY") {
        TransactionType::Buy
    } else {
        TransactionType::Sell
    };

    Ok(Some(BinanceCsvRow {
        timestamp,
        tx_type,
        from_asset: Some(from_asset),
        from_quantity: Some(from_qty),
        to_asset: Some(to_asset),
        to_quantity: Some(to_qty),
        fee: None,
        fee_asset: None,
        price,
        price_asset,
        tx_hash: None,
        address: None,
        network: None,
        external_id: format!("binance-alpha-{}", order_no),
        source: "binance_csv",
        trust_level: "exchange_verified",
        notes: Some(format!(
            "Alpha {} {}/{} (order {})",
            direction, base_asset, quote_asset, order_no
        )),
    }))
}

// ---------------------------------------------------------------------------
// Operation → TransactionType mapping (Transaction History)
// ---------------------------------------------------------------------------

fn classify_tx_history_row(
    operation: &str,
    coin: &str,
    change: Decimal,
) -> Result<(
    TransactionType,
    Option<String>,
    Option<Decimal>,
    Option<String>,
    Option<Decimal>,
)> {
    let positive = change >= Decimal::ZERO;
    let qty = change.abs();
    let asset = Some(coin.to_string());

    // Fail closed: an unrecognised operation is NEVER guessed into a transaction
    // type (the old code defaulted unknowns to `Earn`, silently inventing income).
    // It becomes an error so the row is surfaced as skipped for human review.
    let tx_type = map_operation(operation).ok_or_else(|| {
        CryptofolioError::Config(format!(
            "unrecognised Binance operation '{}' — review and add a mapping before importing",
            operation
        ))
    })?;

    // Direction: positive Change = money arriving (to_account), negative = leaving (from_account)
    let (from_asset, from_qty, to_asset, to_qty) = if positive {
        (None, None, asset, Some(qty))
    } else {
        (asset, Some(qty), None, None)
    };

    Ok((tx_type, from_asset, from_qty, to_asset, to_qty))
}

/// Map a Binance Transaction-History operation string to a transaction type.
/// Returns `None` for unrecognised operations — callers must fail closed rather
/// than guess, so an unknown op is never silently classified as income.
fn map_operation(op: &str) -> Option<TransactionType> {
    use TransactionType::*;
    Some(match op {
        "Transaction Buy" | "Buy Crypto With Fiat" | "Buy" | "Instant Order Settlement" => Buy,
        "Transaction Sold" => Sell,
        "Transaction Spend" | "Merchant Acquiring" => TransferOut,
        "Transaction Revenue" => TransferIn,
        "Transaction Fee" | "Fee" | "Alpha Token - Payment" => Fee,
        "Withdraw" | "Send" => TransferOut,
        "Deposit" => TransferIn,
        "Simple Earn Flexible Subscription"
        | "Simple Earn Locked Subscription"
        | "Alpha 2.0 - Asset Freeze"
        | "Asset Freeze" => Stake,
        "Simple Earn Flexible Redemption"
        | "Simple Earn Locked Redemption"
        | "Alpha 2.0 - Refund"
        | "Alpha Token - Refund" => Unstake,
        "Simple Earn Flexible Interest"
        | "Simple Earn Locked Rewards"
        | "Strategy Trading Fee Rebate"
        | "Futures Referral Rebate"
        | "Cash Voucher" => Earn,
        "HODLer Airdrops Distribution"
        | "Earn - Airdrop Distribution"
        | "Launchpool Airdrop - System Distribution"
        | "Crypto Box" => Airdrop,
        "Binance Convert" => Swap,
        "Transfer Between Spot and Strategy Account"
        | "Transfer Between Spot and Strategy"
        | "Transfer Funds to Spot"
        | "Transfer Funds to Funding Wallet"
        | "Transfer Between Alpha And Spot"
        | "Transfer Between Alpha and Spot"
        | "Vega - Funds Transfer"
        | "Funds Transfer Request - Vega"
        | "Transfer"
        | "Transfer Funds to Spot Account" => TransferInternal,
        "Realized Profit and Loss" => Correction,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse Binance 2-digit-year timestamp: "25-11-28 13:29:41"
fn parse_binance_time(s: &str) -> Result<DateTime<Utc>> {
    // Binance Transaction History emits a four-digit year, e.g.
    // "2025-11-28 13:29:41". (%y expects two digits and rejected everything.)
    // Accept a date-only form too, defaulting to midnight.
    let parsed = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .or_else(|| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
        });
    parsed
        .map(|dt| Utc.from_utc_datetime(&dt))
        .ok_or_else(|| CryptofolioError::Config(format!("Cannot parse Binance timestamp: '{}'", s)))
}

/// Strip commas and parse a Decimal.
fn parse_decimal_clean(s: &str) -> Result<Decimal> {
    let clean = s.trim().replace(',', "");
    Decimal::from_str(&clean).map_err(|_| CryptofolioError::InvalidAmount(s.to_string()))
}

/// Split "9.434SOL" → (9.434, "SOL").
fn split_qty_asset(s: &str) -> Result<(Decimal, String)> {
    let s = s.trim();
    // Find where digits end and the asset ticker begins
    let split = s
        .find(|c: char| c.is_alphabetic())
        .ok_or_else(|| CryptofolioError::InvalidAmount(format!("No asset in '{}'", s)))?;
    let qty = parse_decimal_clean(&s[..split])?;
    let asset = s[split..].trim().to_uppercase();
    Ok((qty, asset))
}

fn field(r: &csv::StringRecord, idx: usize) -> Result<&str> {
    r.get(idx)
        .map(|s| s.trim())
        .ok_or_else(|| CryptofolioError::Config(format!("Missing column {}", idx)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_tx_history_trade_pairing_prices_buy_and_sell() {
        // A buy and a sell, each as a Transaction History coin leg + quote leg at
        // the same timestamp. After pairing, the coin rows must carry the real
        // execution price and the quote legs must be dropped.
        //   BUY : +0.01 BTC spent 650 USDT -> price 65000
        //   SELL: -0.01 BTC revenue 690 USDT -> price 69000
        let csv = "\u{feff}User ID,Time,Account,Operation,Coin,Change,Remark\n\
            1,2026-02-01 10:00:00,Spot,Transaction Buy,BTC,0.01,\n\
            1,2026-02-01 10:00:00,Spot,Transaction Spend,USDT,-650,\n\
            1,2026-03-01 12:00:00,Spot,Transaction Sold,BTC,-0.01,\n\
            1,2026-03-01 12:00:00,Spot,Transaction Revenue,USDT,690,\n";
        let report = parse_bytes(csv.as_bytes()).expect("parse");
        // Quote legs folded away: only the 2 priced trade rows remain.
        let trades: Vec<_> = report
            .rows
            .iter()
            .filter(|r| matches!(r.tx_type, TransactionType::Buy | TransactionType::Sell))
            .collect();
        assert_eq!(trades.len(), 2, "expected one buy + one sell");
        let buy = trades
            .iter()
            .find(|r| matches!(r.tx_type, TransactionType::Buy))
            .unwrap();
        assert_eq!(buy.to_asset.as_deref(), Some("BTC"));
        assert_eq!(buy.price, Some(Decimal::from(65000)));
        let sell = trades
            .iter()
            .find(|r| matches!(r.tx_type, TransactionType::Sell))
            .unwrap();
        assert_eq!(sell.from_asset.as_deref(), Some("BTC"));
        assert_eq!(sell.price, Some(Decimal::from(69000)));
        // No stray stable-quote transfer rows left behind.
        let stray = report.rows.iter().any(|r| {
            matches!(
                r.tx_type,
                TransactionType::TransferOut | TransactionType::TransferIn
            ) && r
                .notes
                .as_deref()
                .map(|n| n.contains("Transaction"))
                .unwrap_or(false)
        });
        assert!(!stray, "quote legs should have been folded into the trades");
    }

    #[test]
    fn test_binance_convert_pairing_prices_buy_and_sell() {
        // Convert = two Swap legs at one timestamp. coin-in + stable-out => BUY;
        // coin-out + stable-in => SELL. Both must become priced trades.
        let csv = "\u{feff}User ID,Time,Account,Operation,Coin,Change,Remark\n\
            1,2025-11-28 13:54:14,Spot,Binance Convert,BTC,0.0109949,\n\
            1,2025-11-28 13:54:14,Spot,Binance Convert,USDC,-1008,\n\
            1,2025-12-14 06:08:11,Spot,Binance Convert,BTC,-0.0045259,\n\
            1,2025-12-14 06:08:11,Spot,Binance Convert,USDT,401.25,\n";
        let report = parse_bytes(csv.as_bytes()).expect("parse");
        let buy = report
            .rows
            .iter()
            .find(|r| {
                matches!(r.tx_type, TransactionType::Buy) && r.to_asset.as_deref() == Some("BTC")
            })
            .expect("convert buy");
        // 1008 / 0.0109949 ~ 91678.87
        assert!(
            buy.price.unwrap() > Decimal::from(91000) && buy.price.unwrap() < Decimal::from(92000)
        );
        let sell = report
            .rows
            .iter()
            .find(|r| {
                matches!(r.tx_type, TransactionType::Sell) && r.from_asset.as_deref() == Some("BTC")
            })
            .expect("convert sell");
        // 401.25 / 0.0045259 ~ 88667
        assert!(
            sell.price.unwrap() > Decimal::from(88000)
                && sell.price.unwrap() < Decimal::from(89000)
        );
        // No leftover Swap rows for the paired converts.
        let swaps = report
            .rows
            .iter()
            .filter(|r| matches!(r.tx_type, TransactionType::Swap))
            .count();
        assert_eq!(
            swaps, 0,
            "convert legs should be rewritten, not left as Swap"
        );
    }

    #[test]
    fn test_detect_format_tx_history() {
        let headers: Vec<String> = [
            "User ID",
            "Time",
            "Account",
            "Operation",
            "Coin",
            "Change",
            "Remark",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            detect_format(&headers),
            Some(BinanceExportFormat::TransactionHistory)
        );
    }

    #[test]
    fn test_detect_format_withdraw() {
        let headers: Vec<String> = [
            "Time", "Coin", "Network", "Amount", "Fee", "Address", "TXID", "Status",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            detect_format(&headers),
            Some(BinanceExportFormat::WithdrawHistory)
        );
    }

    #[test]
    fn test_detect_format_deposit() {
        let headers: Vec<String> = [
            "Time", "Coin", "Network", "Amount", "Address", "TXID", "Status",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            detect_format(&headers),
            Some(BinanceExportFormat::DepositHistory)
        );
    }

    #[test]
    fn test_detect_format_spot_trade() {
        let headers: Vec<String> = ["Time", "Pair", "Side", "Price", "Executed", "Amount", "Fee"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            detect_format(&headers),
            Some(BinanceExportFormat::SpotTradeHistory)
        );
    }

    #[test]
    fn test_split_qty_asset() {
        let (qty, asset) = split_qty_asset("9.434SOL").unwrap();
        assert_eq!(asset, "SOL");
        assert_eq!(qty.to_string(), "9.434");
    }

    #[test]
    fn test_split_qty_asset_usdc() {
        let (_qty, asset) = split_qty_asset("699.43676USDC").unwrap();
        assert_eq!(asset, "USDC");
    }

    #[test]
    fn test_parse_binance_time() {
        // Real Binance exports use a four-digit year.
        let dt = parse_binance_time("2025-11-28 13:29:41").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 28);
    }

    #[test]
    fn test_map_operation_coverage() {
        // All known operations should map to something other than panicking
        let ops = [
            "Transaction Buy",
            "Transaction Sold",
            "Transaction Spend",
            "Transaction Revenue",
            "Transaction Fee",
            "Withdraw",
            "Deposit",
            "Simple Earn Flexible Subscription",
            "Simple Earn Flexible Redemption",
            "Simple Earn Flexible Interest",
            "Simple Earn Locked Rewards",
            "HODLer Airdrops Distribution",
            "Earn - Airdrop Distribution",
            "Binance Convert",
            "Transfer Between Spot and Strategy Account",
            "Strategy Trading Fee Rebate",
            "Realized Profit and Loss",
        ];
        for op in ops {
            assert!(
                map_operation(op).is_some(),
                "known operation '{}' should map to a transaction type",
                op
            );
        }
        // And an unknown op maps to None (so callers fail closed).
        assert!(map_operation("Definitely Not A Real Op").is_none());
    }

    #[test]
    fn test_parse_tx_history_csv() {
        let csv = "User ID,Time,Account,Operation,Coin,Change,Remark\n\
                   1234567,2025-11-28 13:29:41,Spot,Deposit,USD,2000.72,\n\
                   1234567,2025-11-28 13:37:32,Spot,Simple Earn Flexible Interest,USDC,1.23,\n";

        let rows = parse_bytes(csv.as_bytes()).unwrap().rows;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tx_type, TransactionType::TransferIn);
        assert_eq!(rows[0].to_asset.as_deref(), Some("USD"));
        assert_eq!(rows[1].tx_type, TransactionType::Earn);
        assert_eq!(rows[1].to_asset.as_deref(), Some("USDC"));
    }

    #[test]
    fn test_parse_withdraw_csv_skips_pending() {
        let csv = "Time,Coin,Network,Amount,Fee,Address,TXID,Status\n\
                   26-06-02 21:39:30,TAO,TAO,9.9997068,0.0003,5GQs7Dx,0xabc123,Completed\n\
                   26-06-01 10:00:00,ETH,ETH,1.0,0.001,0xaabbcc,0xdef456,Pending\n";

        let rows = parse_bytes(csv.as_bytes()).unwrap().rows;
        assert_eq!(rows.len(), 1); // Pending skipped
        assert_eq!(rows[0].tx_hash.as_deref(), Some("0xabc123"));
        assert_eq!(rows[0].address.as_deref(), Some("5GQs7Dx"));
        assert_eq!(rows[0].tx_type, TransactionType::TransferOut);
        // Network is captured from the export, not guessed.
        assert_eq!(rows[0].network.as_deref(), Some("TAO"));
    }

    #[test]
    fn test_parse_spot_trade_captures_price() {
        // Time, Pair, Side, Price, Executed, Amount, Fee
        let csv = "Time,Pair,Side,Price,Executed,Amount,Fee\n\
                   25-11-28 13:29:41,SOLUSDC,BUY,74.12,9.434SOL,699.43676USDC,0.00081438BNB\n";
        let rows = parse_bytes(csv.as_bytes()).unwrap().rows;
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.tx_type, TransactionType::Buy);
        // Price column (field 3) is captured, quoted in the quote asset.
        assert_eq!(row.price, Some(Decimal::from_str("74.12").unwrap()));
        assert_eq!(row.price_asset.as_deref(), Some("USDC"));
        // Sanity: buy of SOL paid in USDC.
        assert_eq!(row.to_asset.as_deref(), Some("SOL"));
        assert_eq!(row.from_asset.as_deref(), Some("USDC"));
    }

    #[test]
    fn test_external_id_uniqueness() {
        // Two rows with same time but different coin must have different external_ids
        let csv = "User ID,Time,Account,Operation,Coin,Change,Remark\n\
                   1,2025-11-28 13:29:41,Spot,Deposit,USD,100.00,\n\
                   1,2025-11-28 13:29:41,Spot,Deposit,USDT,100.00,\n";
        let rows = parse_bytes(csv.as_bytes()).unwrap().rows;
        assert_ne!(rows[0].external_id, rows[1].external_id);
    }

    #[test]
    fn test_identical_rows_get_distinct_external_ids() {
        // Two legitimately distinct rows sharing (time, coin, change) must BOTH
        // survive — the dedup key disambiguates them instead of collapsing one.
        let csv = "User ID,Time,Account,Operation,Coin,Change,Remark\n\
                   1,2025-11-28 13:29:41,Spot,Simple Earn Flexible Interest,USDC,1.23,\n\
                   1,2025-11-28 13:29:41,Spot,Simple Earn Flexible Interest,USDC,1.23,\n";
        let rows = parse_bytes(csv.as_bytes()).unwrap().rows;
        assert_eq!(rows.len(), 2);
        assert_ne!(rows[0].external_id, rows[1].external_id);

        // Idempotent: re-parsing the same file yields the same ids (no new dups).
        let rows2 = parse_bytes(csv.as_bytes()).unwrap().rows;
        assert_eq!(rows[0].external_id, rows2[0].external_id);
        assert_eq!(rows[1].external_id, rows2[1].external_id);
    }

    #[test]
    fn test_unknown_operation_fails_closed_not_earn() {
        // An unrecognised operation must NOT be silently imported as income.
        let csv = "User ID,Time,Account,Operation,Coin,Change,Remark\n\
                   1,2025-11-28 13:29:41,Spot,Deposit,USDT,100.00,\n\
                   1,2025-11-28 13:30:00,Spot,Some Brand New Op,XYZ,5.00,\n";
        let report = parse_bytes(csv.as_bytes()).unwrap();

        // The known deposit imports; the unknown op is skipped, not invented as Earn.
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].tx_type, TransactionType::TransferIn);
        assert!(report
            .rows
            .iter()
            .all(|r| r.tx_type != TransactionType::Earn));

        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].reason.contains("Some Brand New Op"));
    }

    #[test]
    fn test_malformed_row_is_reported_not_dropped() {
        // A row with an unparseable amount is surfaced as skipped, not silently lost.
        let csv = "User ID,Time,Account,Operation,Coin,Change,Remark\n\
                   1,2025-11-28 13:29:41,Spot,Deposit,USDT,100.00,\n\
                   1,2025-11-28 13:30:00,Spot,Deposit,USDT,not_a_number,\n";
        let report = parse_bytes(csv.as_bytes()).unwrap();

        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.skipped.len(), 1);
        // Line number points at the offending record (header is line 1).
        assert_eq!(report.skipped[0].line, 3);
    }

    #[test]
    fn test_parse_binance_time_four_digit_year() {
        // Regression: real Binance exports use a four-digit year. The parser
        // previously used %y (two digits) and rejected every row.
        let t = parse_binance_time("2025-11-28 13:29:41").expect("4-digit year parses");
        assert_eq!(t.to_rfc3339(), "2025-11-28T13:29:41+00:00");
        // Date-only form defaults to midnight.
        let d = parse_binance_time("2026-04-09").expect("date-only parses");
        assert_eq!(d.to_rfc3339(), "2026-04-09T00:00:00+00:00");
        // Genuinely bad input still errors.
        assert!(parse_binance_time("not-a-date").is_err());
    }
}
