//! `cryptofolio mining-pnl` — the DePIN mining P&L statement.
//!
//! Books mined tokens as revenue at $0 cost basis and the `MINER-*` hardware as
//! a depreciating capital asset, then prints the operating profit and
//! capital-recovery view described in `docs/MINING_ASSET_ACCOUNTING.md`.
//!
//! Revenue is taken from the **dated reward stream** whenever the ledger has it:
//! the sync books each on-chain DePIN reward as a `receive` row with a
//! `MINING REWARD: …` note, and those dates/quantities are chain truth. When no
//! dated rows exist the command falls back to valuing currently-held earned
//! tokens at current FMV (the historical approximation).
//!
//! # Price approximation (explicit, not silent)
//!
//! The model values each reward at its FMV on the receipt date. This codebase
//! has no historical price-at-date pipeline, so a dated reward is valued at the
//! token's CURRENT price. Dates and quantities are exact; the per-unit price is
//! the approximation. `--json` reports `revenue_basis` so consumers can tell
//! which one was used.
//!
//! Depreciation parameters are read from the ledger: each rig has a `correction`
//! transaction whose `notes` begin `DEPRECIATION <MINER-SYMBOL>: {json}` with
//! `{cost_usd, useful_life_months, in_service_date, [salvage_usd]}`. The latest
//! such row for a rig wins (append-only revisions).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::cli::output::format_usd;
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::core::holdings::Holding;
use crate::core::mining::{
    dated_reward_revenue, depreciation_at, mining_pnl, select_revenue, DepreciationParams,
    RevenueBasis, RewardEvent,
};
use crate::db::{AccountRepository, HoldingRepository};
use crate::error::{CryptofolioError, Result};
use rust_decimal::Decimal;

/// Earned-token symbols treated as mining revenue (zero cost basis).
const EARNED_TOKENS: &[&str] = &["GEOD", "WINGS"];

/// Note prefix the sync writes on dated on-chain reward income rows.
const MINING_REWARD_NOTE_PREFIX: &str = "MINING REWARD";

#[derive(Serialize)]
struct MiningPnlOutput {
    revenue_usd: String,
    depreciation_usd: String,
    opex_usd: String,
    operating_profit_usd: String,
    hardware_cost_usd: String,
    net_book_value_usd: String,
    capital_recovered_percent: String,
    // --- Added fields (existing shape kept for backward compatibility) ---
    /// `"dated_rewards"` when the dated ledger stream was used, else
    /// `"current_fmv"` for the fallback approximation.
    revenue_basis: String,
    /// Number of dated reward events behind `revenue_usd` (0 in fallback mode).
    dated_reward_count: usize,
    /// USD total of the dated reward stream (0 in fallback mode).
    dated_rewards_usd: String,
}

/// Parse a `DEPRECIATION <symbol>: {json}` note into (symbol, params).
fn parse_depreciation_note(note: &str) -> Option<(String, DepreciationParams)> {
    let rest = note.strip_prefix("DEPRECIATION ")?;
    let (symbol, json) = rest.split_once(": ")?;
    let v: serde_json::Value = serde_json::from_str(json.trim()).ok()?;

    let cost_usd =
        Decimal::from_str_exact(v.get("cost_usd")?.to_string().trim_matches('"')).ok()?;
    let useful_life_months = v.get("useful_life_months")?.as_u64()? as u32;
    let in_service_str = v.get("in_service_date")?.as_str()?;
    let in_service_date = NaiveDate::parse_from_str(in_service_str, "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)?
        .and_utc();
    let salvage_usd = v
        .get("salvage_usd")
        .and_then(|s| Decimal::from_str_exact(s.to_string().trim_matches('"')).ok())
        .unwrap_or(Decimal::ZERO);

    Some((
        symbol.trim().to_string(),
        DepreciationParams {
            cost_usd,
            useful_life_months,
            in_service_date,
            salvage_usd,
        },
    ))
}

/// Read the dated mining-reward income rows the sync wrote to the ledger.
///
/// Returns `(date, uppercase asset, quantity)` for the earned-token symbols
/// only. A malformed timestamp or quantity is an error (`DateParse` /
/// `DecimalParse`), never a silent `Utc::now()` or zero.
async fn load_dated_reward_rows(
    pool: &SqlitePool,
) -> Result<Vec<(DateTime<Utc>, String, Decimal)>> {
    let pattern = format!("{}%", MINING_REWARD_NOTE_PREFIX);
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT timestamp, to_asset, to_quantity
         FROM transactions
         WHERE tx_type = 'receive' AND notes LIKE ?
         ORDER BY timestamp ASC, id ASC",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let mut dated = Vec::new();
    for (timestamp, asset, quantity) in rows {
        let asset = asset.to_uppercase();
        if !EARNED_TOKENS.contains(&asset.as_str()) {
            continue;
        }
        let quantity = quantity.ok_or_else(|| {
            CryptofolioError::InvalidAmount(format!("mining reward for {asset} has no quantity"))
        })?;
        let quantity = Decimal::from_str_exact(&quantity)?;
        let date = DateTime::parse_from_rfc3339(&timestamp)?.with_timezone(&Utc);
        dated.push((date, asset, quantity));
    }
    Ok(dated)
}

/// Value each dated reward at the price supplied by `price_map`.
///
/// APPROXIMATION: the price is today's price (no historical pipeline exists), so
/// only the per-unit FMV is approximate. Dates and quantities are exact.
fn reward_events_from_rows(
    rows: Vec<(DateTime<Utc>, String, Decimal)>,
    price_map: &HashMap<String, Decimal>,
) -> Vec<RewardEvent> {
    rows.into_iter()
        .map(|(date, asset, quantity)| {
            let price = price_map.get(&asset).copied().unwrap_or(Decimal::ZERO);
            RewardEvent {
                date,
                asset,
                quantity,
                fmv_usd: quantity * price,
            }
        })
        .collect()
}

/// A zero-quantity placeholder holding, used only to make `build_price_map`
/// price a symbol the wallet may no longer hold (e.g. an earned token that was
/// fully sold after being received). The price pipeline keys off symbols, so a
/// placeholder is enough to get a price for it.
fn synthetic_holding(asset: &str, as_of: DateTime<Utc>) -> Holding {
    Holding {
        id: 0,
        account_id: String::new(),
        asset: asset.to_string(),
        quantity: Decimal::ZERO,
        avg_cost_basis: None,
        cost_basis_currency: None,
        avg_cost_basis_base: None,
        updated_at: as_of,
    }
}

pub async fn handle_mining_pnl_command(pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;
    let holding_repo = HoldingRepository::new(pool);
    let _account_repo = AccountRepository::new(pool);
    let as_of: DateTime<Utc> = Utc::now();

    let all_holdings = holding_repo.list_all().await?;

    // --- Revenue source 1: the dated on-chain reward stream (if present) ---
    let dated_rows = load_dated_reward_rows(pool).await?;

    // The price pipeline is driven by holdings, so add a placeholder for any
    // reward asset no longer held — otherwise its price would be missing and the
    // dated revenue silently zeroed.
    let mut price_holdings = all_holdings.clone();
    let mut priced: HashSet<String> = all_holdings
        .iter()
        .map(|h| h.asset.to_uppercase())
        .collect();
    for (_, asset, _) in &dated_rows {
        if priced.insert(asset.clone()) {
            price_holdings.push(synthetic_holding(asset, as_of));
        }
    }

    let price_map =
        crate::core::pricing::build_price_map(&price_holdings, &config, use_testnet).await;
    let reward_events = reward_events_from_rows(dated_rows, &price_map);

    // --- Revenue source 2: earned tokens held today at current FMV ---
    let mut current_fmv = Decimal::ZERO;
    let mut token_lines: Vec<(String, Decimal, Decimal)> = Vec::new();
    for h in &all_holdings {
        let up = h.asset.to_uppercase();
        if EARNED_TOKENS.contains(&up.as_str()) {
            let price = price_map.get(&up).copied().unwrap_or(Decimal::ZERO);
            let value = h.quantity * price;
            current_fmv += value;
            token_lines.push((h.asset.clone(), h.quantity, value));
        }
    }

    // Dated stream wins when present; otherwise the current-FMV approximation.
    let (revenue, revenue_basis) = select_revenue(&reward_events, current_fmv);

    // Aggregate the dated events per asset for the human breakdown.
    let mut dated_totals: HashMap<String, (Decimal, Decimal)> = HashMap::new();
    for event in &reward_events {
        let entry = dated_totals
            .entry(event.asset.clone())
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += event.quantity;
        entry.1 += event.fmv_usd;
    }
    let mut dated_lines: Vec<(String, Decimal, Decimal)> = dated_totals
        .into_iter()
        .map(|(asset, (quantity, value))| (asset, quantity, value))
        .collect();
    dated_lines.sort_by(|a, b| a.0.cmp(&b.0));

    // --- Depreciation: latest DEPRECIATION note per rig ---
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT notes FROM transactions WHERE notes LIKE 'DEPRECIATION %' ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Latest row per symbol wins (append-only revisions).
    let mut params_by_symbol: std::collections::HashMap<String, DepreciationParams> =
        std::collections::HashMap::new();
    for note in rows {
        if let Some((sym, params)) = parse_depreciation_note(&note) {
            params_by_symbol.insert(sym, params);
        }
    }

    let mut depreciation = Decimal::ZERO;
    let mut hardware_cost = Decimal::ZERO;
    let mut rig_lines: Vec<(String, Decimal, Decimal, u32)> = Vec::new();
    for (sym, params) in &params_by_symbol {
        let state = depreciation_at(params, as_of);
        depreciation += state.accumulated;
        hardware_cost += params.cost_usd;
        rig_lines.push((
            sym.clone(),
            params.cost_usd,
            state.net_book_value,
            state.months_in_service,
        ));
    }

    // Opex is $0 by design (miners run in owned homes — no incremental cost).
    let opex = Decimal::ZERO;
    let pnl = mining_pnl(revenue, depreciation, hardware_cost, opex);

    if opts.json {
        let out = MiningPnlOutput {
            revenue_usd: pnl.revenue_usd.round_dp(2).to_string(),
            depreciation_usd: pnl.depreciation_usd.round_dp(2).to_string(),
            opex_usd: pnl.opex_usd.round_dp(2).to_string(),
            operating_profit_usd: pnl.operating_profit_usd.round_dp(2).to_string(),
            hardware_cost_usd: pnl.hardware_cost_usd.round_dp(2).to_string(),
            net_book_value_usd: pnl.net_book_value_usd.round_dp(2).to_string(),
            capital_recovered_percent: (pnl.capital_recovered_fraction * Decimal::from(100))
                .round_dp(1)
                .to_string(),
            revenue_basis: revenue_basis.as_str().to_string(),
            dated_reward_count: reward_events.len(),
            dated_rewards_usd: dated_reward_revenue(&reward_events).round_dp(2).to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("DePIN Mining Operation — P&L (cumulative)\n");
    println!(
        "  Revenue (tokens mined, FMV at $0 cost):   {}",
        format_usd(pnl.revenue_usd)
    );
    match revenue_basis {
        RevenueBasis::DatedRewards => {
            println!(
                "    basis: dated on-chain rewards ({} events, valued at current price)",
                reward_events.len()
            );
            for (asset, qty, value) in &dated_lines {
                println!("    {asset:6} {qty:>14.4} = {}", format_usd(*value));
            }
        }
        RevenueBasis::CurrentFmv => {
            println!("    basis: current FMV of held tokens (no dated reward stream in ledger)");
            for (asset, qty, value) in &token_lines {
                println!("    {asset:6} {qty:>14.4} = {}", format_usd(*value));
            }
        }
    }
    println!(
        "  Less: hardware depreciation:              -{}",
        format_usd(pnl.depreciation_usd)
    );
    println!(
        "  Less: electricity/internet (owned homes): -{}",
        format_usd(pnl.opex_usd)
    );
    println!("  {}", "-".repeat(52));
    println!(
        "  Operating profit/(loss):                   {}",
        format_usd(pnl.operating_profit_usd)
    );
    println!();
    println!("  Memo — capital recovery:");
    println!(
        "    Hardware at cost:          {}",
        format_usd(pnl.hardware_cost_usd)
    );
    for (sym, cost, nbv, months) in &rig_lines {
        println!(
            "      {sym:20} cost {} -> NBV {} ({months}mo)",
            format_usd(*cost),
            format_usd(*nbv)
        );
    }
    println!(
        "    Accumulated depreciation:  -{}",
        format_usd(pnl.depreciation_usd)
    );
    println!(
        "    Net book value:             {}",
        format_usd(pnl.net_book_value_usd)
    );
    println!(
        "    Tokens earned to date:      {}",
        format_usd(pnl.revenue_usd)
    );
    println!(
        "    Capital recovered:          {:.1}%",
        pnl.capital_recovered_fraction * Decimal::from(100)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_pool;

    const ACCOUNT_ID: &str = "test-wallet";

    async fn setup_db() -> SqlitePool {
        let pool = init_memory_pool().await.expect("in-memory pool");
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test-cat', 'Test')")
            .execute(&pool)
            .await
            .expect("insert category");
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) \
             VALUES (?, 'test-cat', 'Miner Wallet', 'wallet')",
        )
        .bind(ACCOUNT_ID)
        .execute(&pool)
        .await
        .expect("insert account");
        pool
    }

    async fn insert_reward_row(pool: &SqlitePool, timestamp: &str, asset: &str, qty: &str) {
        sqlx::query(
            "INSERT INTO transactions
             (tx_type, to_account_id, to_asset, to_quantity,
              external_id, source, trust_level, notes, timestamp)
             VALUES ('receive', ?, ?, ?, ?, 'helius', 'chain_verified', 'MINING REWARD: solana-x', ?)",
        )
        .bind(ACCOUNT_ID)
        .bind(asset)
        .bind(qty)
        .bind(format!("solana-{timestamp}"))
        .bind(timestamp)
        .execute(pool)
        .await
        .expect("insert reward row");
    }

    #[tokio::test]
    async fn dated_rewards_take_precedence_over_current_fmv() {
        let pool = setup_db().await;
        insert_reward_row(&pool, "2024-10-01T12:00:00+00:00", "GEOD", "10").await;
        insert_reward_row(&pool, "2024-10-02T12:00:00+00:00", "GEOD", "10").await;
        insert_reward_row(&pool, "2024-10-02T13:00:00+00:00", "WINGS", "4").await;
        // A non-earned token receive must be ignored even with the note.
        insert_reward_row(&pool, "2024-10-03T12:00:00+00:00", "BTC", "1").await;
        // A plain receive without the mining note must be ignored.
        sqlx::query(
            "INSERT INTO transactions (tx_type, to_account_id, to_asset, to_quantity, timestamp)
             VALUES ('receive', ?, 'GEOD', '99', '2024-10-04T12:00:00+00:00')",
        )
        .bind(ACCOUNT_ID)
        .execute(&pool)
        .await
        .expect("insert plain receive");

        let rows = load_dated_reward_rows(&pool).await.expect("dated rows");
        assert_eq!(rows.len(), 3);

        let mut price_map = HashMap::new();
        price_map.insert("GEOD".to_string(), Decimal::from_str_exact("0.05").unwrap());
        price_map.insert(
            "WINGS".to_string(),
            Decimal::from_str_exact("1.00").unwrap(),
        );

        let events = reward_events_from_rows(rows, &price_map);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].asset, "GEOD");
        assert_eq!(events[0].quantity, Decimal::from_str_exact("10").unwrap());
        assert_eq!(events[0].fmv_usd, Decimal::from_str_exact("0.50").unwrap());
        assert_eq!(
            events[0].date,
            DateTime::parse_from_rfc3339("2024-10-01T12:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc)
        );

        // Dated revenue wins over the (much larger) current-FMV fallback.
        let (revenue, basis) = select_revenue(&events, Decimal::from_str_exact("9999").unwrap());
        assert_eq!(basis, RevenueBasis::DatedRewards);
        // 20 × 0.05 + 4 × 1.00 = 5.00
        assert_eq!(revenue, Decimal::from_str_exact("5.00").unwrap());
    }

    #[tokio::test]
    async fn falls_back_to_current_fmv_without_dated_rows() {
        let pool = setup_db().await;

        let rows = load_dated_reward_rows(&pool).await.expect("dated rows");
        assert!(rows.is_empty());

        let events = reward_events_from_rows(rows, &HashMap::new());
        assert!(events.is_empty());

        let (revenue, basis) = select_revenue(&events, Decimal::from_str_exact("671.59").unwrap());
        assert_eq!(basis, RevenueBasis::CurrentFmv);
        assert_eq!(revenue, Decimal::from_str_exact("671.59").unwrap());
    }

    #[tokio::test]
    async fn malformed_reward_timestamp_is_an_error() {
        let pool = setup_db().await;
        insert_reward_row(&pool, "not-a-date", "GEOD", "10").await;

        let err = load_dated_reward_rows(&pool)
            .await
            .expect_err("malformed timestamp must be DateParse, never Utc::now()");
        assert!(
            matches!(err, CryptofolioError::DateParse(_)),
            "expected DateParse, got {err:?}"
        );
    }

    #[test]
    fn mining_pnl_output_json_is_backward_compatible() {
        let out = MiningPnlOutput {
            revenue_usd: "5.00".to_string(),
            depreciation_usd: "1.00".to_string(),
            opex_usd: "0".to_string(),
            operating_profit_usd: "4.00".to_string(),
            hardware_cost_usd: "100.00".to_string(),
            net_book_value_usd: "99.00".to_string(),
            capital_recovered_percent: "5.0".to_string(),
            revenue_basis: "dated_rewards".to_string(),
            dated_reward_count: 3,
            dated_rewards_usd: "5.00".to_string(),
        };
        let json = serde_json::to_string(&out).expect("serialize");
        // Original fields keep their names and decimal-as-string shape.
        assert!(json.contains("\"revenue_usd\":\"5.00\""));
        assert!(json.contains("\"capital_recovered_percent\":\"5.0\""));
        // New fields are additive.
        assert!(json.contains("\"revenue_basis\":\"dated_rewards\""));
        assert!(json.contains("\"dated_reward_count\":3"));
        assert!(json.contains("\"dated_rewards_usd\":\"5.00\""));
    }
}
