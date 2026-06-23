use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::SqlitePool;
use std::path::Path;

use crate::cli::output::{error, info, success, warning};
use crate::cli::GlobalOptions;
use crate::core::transaction::{Transaction, TransactionType};
use crate::db::{AccountRepository, AddressRegistry, DiscoveryQueue, TransactionRepository};
use crate::error::{CryptofolioError, Result};
use crate::exchange::binance::csv::{parse_file, BinanceCsvRow};

/// Map a Binance "Network" code (e.g. "ETH", "BSC", "TRX") to our canonical chain
/// name. This is the authoritative source — the export tells us the exact chain,
/// so we never guess when it's present. Unknown codes are passed through lowercased
/// rather than discarded, preserving what Binance reported.
fn network_to_chain(network: &str) -> String {
    match network.to_uppercase().as_str() {
        "BTC" | "BITCOIN" => "bitcoin".to_string(),
        "ETH" | "ERC20" | "ETHEREUM" => "ethereum".to_string(),
        "BSC" | "BEP20" | "BEP2" => "bsc".to_string(),
        "TRX" | "TRC20" | "TRON" => "tron".to_string(),
        "SOL" | "SOLANA" => "solana".to_string(),
        "ADA" | "CARDANO" => "cardano".to_string(),
        "MATIC" | "POLYGON" | "POL" => "polygon".to_string(),
        "ARBITRUM" | "ARB" | "ARBITRUMONE" => "arbitrum".to_string(),
        "OPTIMISM" | "OP" => "optimism".to_string(),
        "AVAXC" | "AVAX" | "AVALANCHE" => "avalanche".to_string(),
        "DOT" | "POLKADOT" => "polkadot".to_string(),
        "ATOM" | "COSMOS" => "cosmos".to_string(),
        "TAO" | "BITTENSOR" => "bittensor".to_string(),
        other => other.to_lowercase(),
    }
}

/// Last-resort chain guess from the coin ticker, used ONLY when the export carries
/// no Network column (e.g. Transaction History rows). Prefer `network_to_chain`.
fn infer_chain(coin: &str) -> &'static str {
    match coin {
        "BTC" => "bitcoin",
        "ETH" | "USDT" | "USDC" | "BNB" | "LINK" | "UNI" | "AAVE" | "MKR" | "COMP" => "ethereum",
        "SOL" => "solana",
        "ADA" => "cardano",
        "DOT" => "polkadot",
        "MATIC" | "POL" => "polygon",
        "AVAX" => "avalanche",
        "ATOM" => "cosmos",
        "TAO" => "bittensor",
        _ => "unknown",
    }
}

/// USD-equivalent quote assets — when a trade is priced in one of these, the price
/// is (within stablecoin tolerance) the USD price, so it populates price_usd.
fn is_usd_equivalent(asset: &str) -> bool {
    matches!(
        asset.to_uppercase().as_str(),
        "USD" | "USDT" | "USDC" | "BUSD" | "DAI" | "FDUSD" | "TUSD" | "USDP"
    )
}

pub async fn handle_import_binance_command(
    file: String,
    account: String,
    dry_run: bool,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let path = Path::new(&file);
    if !path.exists() {
        return Err(CryptofolioError::Config(format!(
            "File not found: {}",
            file
        )));
    }

    let account_repo = AccountRepository::new(pool);
    let tx_repo = TransactionRepository::new(pool);
    let addr_registry = AddressRegistry::new(pool);
    let disc_queue = DiscoveryQueue::new(pool);

    let acc = account_repo
        .get_account(&account)
        .await?
        .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

    if !opts.quiet {
        info(&format!("Parsing Binance export '{}'...", file));
    }

    let report = parse_file(path)?;

    // Surface parse-level skips loudly — never lose financial records silently.
    if !report.skipped.is_empty() {
        warning(&format!(
            "{} row(s) could not be parsed and were skipped:",
            report.skipped.len()
        ));
        for s in &report.skipped {
            warning(&format!("  line {}: {}", s.line, s.reason));
        }
    }

    if report.rows.is_empty() {
        warning("No importable rows found in file.");
        return Ok(());
    }

    if !opts.quiet {
        info(&format!(
            "Found {} importable rows — importing into '{}'",
            report.rows.len(),
            account
        ));
    }

    if dry_run {
        println!(
            "Dry run — {} rows would be imported, {} skipped during parsing. Not writing anything.",
            report.rows.len(),
            report.skipped.len()
        );
        return Ok(());
    }

    let progress = if !opts.quiet {
        let pb = ProgressBar::new(report.rows.len() as u64);
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-");
        pb.set_style(style);
        Some(pb)
    } else {
        None
    };

    let mut imported = 0u64;
    let mut dup_skipped = 0u64;
    let mut error_skipped = 0u64;
    let mut addresses_discovered = 0u64;

    for row in &report.rows {
        match import_row(row, &acc.id, &tx_repo, &addr_registry, &disc_queue).await {
            Ok((_tx_id, addr_count)) => {
                imported += 1;
                addresses_discovered += addr_count;
            }
            Err(CryptofolioError::Database(e))
                if e.to_string().contains("UNIQUE constraint failed") =>
            {
                dup_skipped += 1;
            }
            Err(e) => {
                if !opts.quiet {
                    error(&format!("Skipped row ({}): {}", row.external_id, e));
                }
                error_skipped += 1;
            }
        }

        if let Some(ref pb) = progress {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    let pending = disc_queue.pending_count().await.unwrap_or(0);
    let parse_skipped = report.skipped.len();

    success(&format!(
        "Imported {} transactions ({} duplicates, {} parse-skipped, {} write-errors)",
        imported, dup_skipped, parse_skipped, error_skipped
    ));

    if addresses_discovered > 0 {
        println!(
            "  {} new addresses added to discovery queue ({} pending sync)",
            addresses_discovered, pending
        );
    }

    Ok(())
}

async fn import_row(
    row: &BinanceCsvRow,
    account_id: &str,
    tx_repo: &TransactionRepository<'_>,
    addr_registry: &AddressRegistry<'_>,
    disc_queue: &DiscoveryQueue<'_>,
) -> Result<(i64, u64)> {
    let tx = row_to_transaction(row, account_id);
    let tx_id = tx_repo.insert(&tx).await?;

    let mut addr_count = 0u64;

    // Register any address we found and enqueue it for future sync.
    if let Some(ref address) = row.address {
        if !address.is_empty() {
            // Prefer the export's Network column; fall back to a coin-based guess
            // only when it's absent.
            let chain = match &row.network {
                Some(net) => network_to_chain(net),
                None => {
                    let coin = row
                        .from_asset
                        .as_deref()
                        .or(row.to_asset.as_deref())
                        .unwrap_or("BTC");
                    infer_chain(coin).to_string()
                }
            };

            // On-chain withdraw destination → external (not mine), deposit source → unknown
            let classification = match row.tx_type {
                TransactionType::TransferOut => "external",
                TransactionType::TransferIn => "unknown",
                _ => "unknown",
            };

            addr_registry
                .upsert(address, &chain, classification, None, None)
                .await?;

            disc_queue.enqueue(address, &chain, Some(tx_id)).await?;
            addr_count += 1;
        }
    }

    Ok((tx_id, addr_count))
}

fn row_to_transaction(row: &BinanceCsvRow, account_id: &str) -> Transaction {
    // Determine account sides based on tx type
    let (from_account_id, to_account_id) = match row.tx_type {
        TransactionType::Buy
        | TransactionType::TransferIn
        | TransactionType::Receive
        | TransactionType::Earn
        | TransactionType::Airdrop
        | TransactionType::Unstake => (None, Some(account_id.to_string())),

        TransactionType::Sell
        | TransactionType::TransferOut
        | TransactionType::Fee
        | TransactionType::Stake => (Some(account_id.to_string()), None),

        TransactionType::Swap | TransactionType::TransferInternal | TransactionType::Correction => {
            (Some(account_id.to_string()), Some(account_id.to_string()))
        }
    };

    // Preserve the captured execution price. If it's quoted in a USD-equivalent
    // stablecoin/fiat, populate price_usd directly; otherwise keep the native
    // quote in price_currency/price_amount so no cost-basis data is lost.
    let (price_usd, price_currency, price_amount) = match (&row.price, &row.price_asset) {
        (Some(price), Some(asset)) if is_usd_equivalent(asset) => (Some(*price), None, None),
        (Some(price), Some(asset)) => (None, Some(asset.clone()), Some(*price)),
        _ => (None, None, None),
    };

    Transaction {
        id: 0,
        tx_type: row.tx_type,
        from_account_id,
        from_asset: row.from_asset.clone(),
        from_quantity: row.from_quantity,
        to_account_id,
        to_asset: row.to_asset.clone(),
        to_quantity: row.to_quantity,
        price_usd,
        price_currency,
        price_amount,
        exchange_rate: None,
        exchange_rate_pair: None,
        fee: row.fee,
        fee_asset: row.fee_asset.clone(),
        tx_hash: row.tx_hash.clone(),
        external_id: Some(row.external_id.clone()),
        source: row.source.to_string(),
        trust_level: row.trust_level.to_string(),
        notes: row.notes.clone(),
        timestamp: row.timestamp,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::binance::csv::BinanceCsvRow;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn trade_row(price: &str, price_asset: &str) -> BinanceCsvRow {
        BinanceCsvRow {
            timestamp: Utc::now(),
            tx_type: TransactionType::Buy,
            from_asset: Some(price_asset.to_string()),
            from_quantity: Some(Decimal::from_str("100").unwrap()),
            to_asset: Some("SOL".to_string()),
            to_quantity: Some(Decimal::from_str("1.35").unwrap()),
            fee: None,
            fee_asset: None,
            price: Some(Decimal::from_str(price).unwrap()),
            price_asset: Some(price_asset.to_string()),
            tx_hash: None,
            address: None,
            network: None,
            external_id: "binance-trade-test".to_string(),
            source: "binance_csv",
            trust_level: "exchange_verified",
            notes: None,
        }
    }

    #[test]
    fn test_price_in_usd_equivalent_populates_price_usd() {
        let tx = row_to_transaction(&trade_row("74.12", "USDC"), "acc");
        assert_eq!(tx.price_usd, Some(Decimal::from_str("74.12").unwrap()));
        assert_eq!(tx.price_currency, None);
        assert_eq!(tx.price_amount, None);
    }

    #[test]
    fn test_price_in_non_usd_preserved_natively() {
        // A trade priced in BTC keeps the native quote — no data is lost.
        let tx = row_to_transaction(&trade_row("0.0021", "BTC"), "acc");
        assert_eq!(tx.price_usd, None);
        assert_eq!(tx.price_currency.as_deref(), Some("BTC"));
        assert_eq!(tx.price_amount, Some(Decimal::from_str("0.0021").unwrap()));
    }

    #[test]
    fn test_network_to_chain_uses_export_value() {
        assert_eq!(network_to_chain("ETH"), "ethereum");
        assert_eq!(network_to_chain("TRC20"), "tron");
        assert_eq!(network_to_chain("BSC"), "bsc");
        // Unknown codes are preserved (lowercased), not discarded or guessed.
        assert_eq!(network_to_chain("NEWCHAIN"), "newchain");
    }
}
