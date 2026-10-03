//! `cryptofolio mining-pnl` — the DePIN mining P&L statement.
//!
//! Books mined tokens as revenue (zero-cost income, valued at current FMV) and
//! the `MINER-*` hardware as a depreciating capital asset, then prints the
//! operating profit and capital-recovery view described in
//! `docs/MINING_ASSET_ACCOUNTING.md`.
//!
//! Depreciation parameters are read from the ledger: each rig has a `correction`
//! transaction whose `notes` begin `DEPRECIATION <MINER-SYMBOL>: {json}` with
//! `{cost_usd, useful_life_months, in_service_date, [salvage_usd]}`. The latest
//! such row for a rig wins (append-only revisions).

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::cli::output::format_usd;
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::core::mining::{depreciation_at, mining_pnl, DepreciationParams};
use crate::db::{AccountRepository, HoldingRepository};
use crate::error::Result;
use rust_decimal::Decimal;

/// Earned-token symbols treated as mining revenue (zero cost basis).
const EARNED_TOKENS: &[&str] = &["GEOD", "WINGS"];

#[derive(Serialize)]
struct MiningPnlOutput {
    revenue_usd: String,
    depreciation_usd: String,
    opex_usd: String,
    operating_profit_usd: String,
    hardware_cost_usd: String,
    net_book_value_usd: String,
    capital_recovered_percent: String,
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

pub async fn handle_mining_pnl_command(pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;
    let holding_repo = HoldingRepository::new(pool);
    let _account_repo = AccountRepository::new(pool);
    let as_of: DateTime<Utc> = Utc::now();

    // --- Revenue: earned tokens at current FMV (zero cost basis) ---
    let all_holdings = holding_repo.list_all().await?;
    let price_map =
        crate::core::pricing::build_price_map(&all_holdings, &config, use_testnet).await;

    let mut revenue = Decimal::ZERO;
    let mut token_lines: Vec<(String, Decimal, Decimal)> = Vec::new();
    for h in &all_holdings {
        let up = h.asset.to_uppercase();
        if EARNED_TOKENS.contains(&up.as_str()) {
            let price = price_map.get(&up).copied().unwrap_or(Decimal::ZERO);
            let value = h.quantity * price;
            revenue += value;
            token_lines.push((h.asset.clone(), h.quantity, value));
        }
    }

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
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("DePIN Mining Operation — P&L (cumulative)\n");
    println!(
        "  Revenue (tokens mined, FMV at $0 cost):   {}",
        format_usd(pnl.revenue_usd)
    );
    for (asset, qty, value) in &token_lines {
        println!("    {asset:6} {qty:>14.4} = {}", format_usd(*value));
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
