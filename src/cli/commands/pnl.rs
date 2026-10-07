use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::cli::output::{
    format_pnl, format_quantity, format_usd, info, print_header, print_row, success, warning,
};
use crate::cli::{GlobalOptions, PnlCommands};
use crate::config::AppConfig;
use crate::core::pnl::{CostBasisMethod, PnLCalculator};
use crate::db::{
    AccountRepository, HoldingRepository, RealizedPnLRepository, TransactionRepository,
};
use crate::error::Result;
use crate::exchange::{BinanceClient, Exchange};

#[derive(Serialize)]
struct PnLSummaryOutput {
    total_realized: String,
    total_unrealized: String,
    net_pnl: String,
    account: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

#[derive(Serialize)]
struct RealizedPnLOutput {
    id: i64,
    date: String,
    asset: String,
    account: String,
    quantity: String,
    cost_basis: String,
    proceeds: String,
    gain_loss: String,
    holding_period_days: Option<i64>,
}

#[derive(Serialize)]
struct UnrealizedPnLOutput {
    asset: String,
    account: String,
    quantity: String,
    avg_cost_basis: String,
    current_price: String,
    current_value: String,
    unrealized_pnl: String,
}

#[derive(Serialize)]
struct AssetPnlOutput {
    asset: String,
    account: Option<String>,
    realized_transactions: usize,
    total_realized: String,
    total_unrealized: String,
    net_pnl: String,
}

pub async fn handle_pnl_command(
    command: PnlCommands,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    match command {
        PnlCommands::Summary { account, from, to } => {
            handle_summary(account, from, to, pool, opts).await
        }
        PnlCommands::Realized {
            account,
            asset,
            from,
            to,
            limit,
        } => handle_realized(account, asset, from, to, limit, pool, opts).await,
        PnlCommands::Unrealized { account, asset } => {
            handle_unrealized(account, asset, pool, opts).await
        }
        PnlCommands::ByAsset { asset, account } => {
            handle_by_asset(asset, account, pool, opts).await
        }
        PnlCommands::Backfill { yes, account } => handle_backfill(yes, account, pool, opts).await,
    }
}

async fn handle_summary(
    account: Option<String>,
    from: Option<String>,
    to: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let pnl_repo = RealizedPnLRepository::new(pool);

    // Parse dates
    let from_date = from
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", s)))
        .transpose()
        .map_err(|_| crate::error::CryptofolioError::InvalidInput("Invalid from date".to_string()))?
        .map(|dt| dt.with_timezone(&Utc));

    let to_date = to
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", s)))
        .transpose()
        .map_err(|_| crate::error::CryptofolioError::InvalidInput("Invalid to date".to_string()))?
        .map(|dt| dt.with_timezone(&Utc));

    // Get realized P&L
    let total_realized = pnl_repo
        .get_summary(account.as_deref(), from_date, to_date)
        .await?;

    // Get unrealized P&L
    let total_unrealized = calculate_total_unrealized(account.as_deref(), None, pool, opts).await?;

    let net_pnl = total_realized + total_unrealized;

    if opts.json {
        let output = PnLSummaryOutput {
            total_realized: total_realized.to_string(),
            total_unrealized: total_unrealized.to_string(),
            net_pnl: net_pnl.to_string(),
            account: account.clone(),
            from: from.clone(),
            to: to.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("\n=== P&L Summary ===\n");
        if let Some(ref acc) = account {
            println!("Account: {}", acc);
        }
        if let Some(ref f) = from {
            println!("From: {}", f);
        }
        if let Some(ref t) = to {
            println!("To: {}", t);
        }
        if account.is_some() || from.is_some() || to.is_some() {
            println!();
        }

        println!("Realized P&L:   {}", format_pnl(total_realized, true));
        println!("Unrealized P&L: {}", format_pnl(total_unrealized, true));
        println!("─────────────────────────────");
        println!("Net P&L:        {}", format_pnl(net_pnl, true));
    }

    Ok(())
}

async fn handle_realized(
    account: Option<String>,
    asset: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: i64,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let pnl_repo = RealizedPnLRepository::new(pool);

    // Parse dates
    let from_date = from
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", s)))
        .transpose()
        .map_err(|_| crate::error::CryptofolioError::InvalidInput("Invalid from date".to_string()))?
        .map(|dt| dt.with_timezone(&Utc));

    let to_date = to
        .as_ref()
        .map(|s| DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", s)))
        .transpose()
        .map_err(|_| crate::error::CryptofolioError::InvalidInput("Invalid to date".to_string()))?
        .map(|dt| dt.with_timezone(&Utc));

    // Get realized P&L records
    let pnls = pnl_repo
        .list_filtered(
            account.as_deref(),
            asset.as_deref(),
            from_date,
            to_date,
            Some(limit),
        )
        .await?;

    if pnls.is_empty() {
        if opts.json {
            println!("[]");
        } else {
            info("No realized P&L records found.");
        }
        return Ok(());
    }

    if opts.json {
        let output: Vec<RealizedPnLOutput> = pnls
            .iter()
            .map(|p| RealizedPnLOutput {
                id: p.id,
                date: p.disposal_date.format("%Y-%m-%d %H:%M:%S").to_string(),
                asset: p.asset.clone(),
                account: p.account_id.clone(),
                quantity: p.quantity.to_string(),
                cost_basis: p.cost_basis.to_string(),
                proceeds: p.proceeds.to_string(),
                gain_loss: p.realized_gain.to_string(),
                holding_period_days: p.holding_period_days,
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("\n=== Realized P&L ===\n");

        print_header(&[
            ("Date", 12),
            ("Asset", 8),
            ("Account", 15),
            ("Quantity", 14),
            ("Cost Basis", 12),
            ("Proceeds", 12),
            ("Gain/Loss", 14),
            ("Days", 6),
        ]);

        for p in &pnls {
            let date_str = p.disposal_date.format("%Y-%m-%d").to_string();
            let qty_str = format_quantity(p.quantity);
            let cost_str = format_usd(p.cost_basis);
            let proceeds_str = format_usd(p.proceeds);
            let pnl_str = format_pnl(p.realized_gain, true);
            let days_str = p
                .holding_period_days
                .map(|d| d.to_string())
                .unwrap_or_else(|| "-".to_string());

            print_row(&[
                (&date_str, 12),
                (&p.asset, 8),
                (&p.account_id, 15),
                (&qty_str, 14),
                (&cost_str, 12),
                (&proceeds_str, 12),
                (&pnl_str, 14),
                (&days_str, 6),
            ]);
        }

        println!();
        let total_gain: Decimal = pnls.iter().map(|p| p.realized_gain).sum();
        println!("Total Realized P&L: {}", format_pnl(total_gain, true));
    }

    Ok(())
}

async fn handle_unrealized(
    account: Option<String>,
    asset: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;
    let holding_repo = HoldingRepository::new(pool);
    let pnl_calc = PnLCalculator::new(pool);

    // Get holdings
    let holdings = if let Some(ref acc) = account {
        holding_repo.list_by_account(acc).await?
    } else {
        holding_repo.list_all().await?
    };

    let filtered_holdings: Vec<_> = if let Some(ref ast) = asset {
        holdings
            .into_iter()
            .filter(|h| h.asset.eq_ignore_ascii_case(ast))
            .collect()
    } else {
        holdings
    };

    if filtered_holdings.is_empty() {
        info("No holdings found.");
        return Ok(());
    }

    // Fetch current prices
    let unique_assets: Vec<String> = filtered_holdings
        .iter()
        .map(|h| h.asset.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let client = BinanceClient::new(use_testnet, None, None);

    let mut prices = std::collections::HashMap::new();
    for asset in &unique_assets {
        if let Ok(price_data) = client.get_price(asset).await {
            prices.insert(asset.clone(), price_data.price);
        }
    }

    let mut unrealized_entries = Vec::new();
    let mut total_unrealized = Decimal::ZERO;

    for holding in &filtered_holdings {
        if let Some(&current_price) = prices.get(&holding.asset) {
            let unrealized_pnl = pnl_calc
                .calculate_unrealized_pnl(&holding.account_id, &holding.asset, current_price)
                .await
                .unwrap_or(Decimal::ZERO);

            let current_value = holding.quantity * current_price;
            let avg_cost = holding.avg_cost_basis.unwrap_or(Decimal::ZERO);

            total_unrealized += unrealized_pnl;

            unrealized_entries.push(UnrealizedPnLOutput {
                asset: holding.asset.clone(),
                account: holding.account_id.clone(),
                quantity: holding.quantity.to_string(),
                avg_cost_basis: avg_cost.to_string(),
                current_price: current_price.to_string(),
                current_value: current_value.to_string(),
                unrealized_pnl: unrealized_pnl.to_string(),
            });
        }
    }

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&unrealized_entries)?);
    } else {
        println!("\n=== Unrealized P&L ===\n");

        print_header(&[
            ("Asset", 8),
            ("Account", 15),
            ("Quantity", 14),
            ("Avg Cost", 12),
            ("Current Price", 14),
            ("Current Value", 14),
            ("Unrealized P&L", 16),
        ]);

        for entry in &unrealized_entries {
            let qty = Decimal::from_str_exact(&entry.quantity).unwrap_or(Decimal::ZERO);
            let avg_cost = Decimal::from_str_exact(&entry.avg_cost_basis).unwrap_or(Decimal::ZERO);
            let current_price =
                Decimal::from_str_exact(&entry.current_price).unwrap_or(Decimal::ZERO);
            let current_value =
                Decimal::from_str_exact(&entry.current_value).unwrap_or(Decimal::ZERO);
            let unrealized =
                Decimal::from_str_exact(&entry.unrealized_pnl).unwrap_or(Decimal::ZERO);

            let qty_str = format_quantity(qty);
            let avg_cost_str = format_usd(avg_cost);
            let current_price_str = format_usd(current_price);
            let current_value_str = format_usd(current_value);
            let unrealized_str = format_pnl(unrealized, true);

            print_row(&[
                (&entry.asset, 8),
                (&entry.account, 15),
                (&qty_str, 14),
                (&avg_cost_str, 12),
                (&current_price_str, 14),
                (&current_value_str, 14),
                (&unrealized_str, 16),
            ]);
        }

        println!();
        println!(
            "Total Unrealized P&L: {}",
            format_pnl(total_unrealized, true)
        );
    }

    Ok(())
}

async fn handle_by_asset(
    asset: String,
    account: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let pnl_repo = RealizedPnLRepository::new(pool);

    // Get realized P&L for this asset
    let realized_pnls = if let Some(ref acc) = account {
        pnl_repo
            .list_filtered(Some(acc), Some(&asset), None, None, None)
            .await?
    } else {
        pnl_repo.list_by_asset(&asset).await?
    };

    let total_realized: Decimal = realized_pnls.iter().map(|p| p.realized_gain).sum();

    // Get unrealized P&L for this asset
    let total_unrealized =
        calculate_total_unrealized(account.as_deref(), Some(&asset), pool, opts).await?;

    let net_pnl = total_realized + total_unrealized;

    if opts.json {
        let output = AssetPnlOutput {
            asset: asset.to_uppercase(),
            account: account.clone(),
            realized_transactions: realized_pnls.len(),
            total_realized: total_realized.to_string(),
            total_unrealized: total_unrealized.to_string(),
            net_pnl: net_pnl.to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("\n=== P&L Breakdown: {} ===\n", asset.to_uppercase());

    println!("Realized P&L:");
    println!("  Transactions: {}", realized_pnls.len());
    println!("  Total Gain/Loss: {}", format_pnl(total_realized, true));

    println!("\nUnrealized P&L:");
    println!("  Total: {}", format_pnl(total_unrealized, true));

    println!("\n─────────────────────────────");
    println!(
        "Net P&L: {}",
        format_pnl(total_realized + total_unrealized, true)
    );

    Ok(())
}

async fn handle_backfill(
    yes: bool,
    account: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    if !yes && !opts.quiet {
        warning("This will clear all existing P&L data and recalculate from transactions.");
        warning("Use --yes to confirm.");
        return Ok(());
    }

    let tx_repo = TransactionRepository::new(pool);
    let pnl_calc = PnLCalculator::new(pool);
    let holding_repo = HoldingRepository::new(pool);

    // Get all transactions in chronological (oldest-first) order. Backfill MUST
    // see the full history and replay buys before sells; the paged `list`/
    // `list_by_account` cap at 50 rows and order DESC, which silently drops
    // history and mis-orders lots.
    let transactions = if let Some(ref acc) = account {
        let mut all = tx_repo.list_all_chronological().await?;
        all.retain(|t| {
            t.from_account_id.as_deref() == Some(acc.as_str())
                || t.to_account_id.as_deref() == Some(acc.as_str())
        });
        all
    } else {
        tx_repo.list_all_chronological().await?
    };

    if transactions.is_empty() {
        info("No transactions found to backfill.");
        return Ok(());
    }

    // Set of the user's own account ids — a transfer whose counterparty is in
    // this set is an INTERNAL move (basis travels with the coins); a counterparty
    // outside it means the coins left/entered the tracked set.
    let account_repo = AccountRepository::new(pool);
    let internal_accounts: std::collections::HashSet<String> = account_repo
        .list_all_ids()
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    if !opts.quiet {
        info(&format!(
            "Backfilling P&L for {} transactions...",
            transactions.len()
        ));
    }

    // Clear existing P&L data. Order matters: realized_pnl.tax_lot_id has a
    // FOREIGN KEY onto tax_lots(id), so the child table MUST be cleared before
    // the parent. Deleting tax_lots first aborts with FK error 787 on any
    // re-run (when realized_pnl already holds rows from a prior backfill).
    sqlx::query("DELETE FROM realized_pnl")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM tax_lots").execute(pool).await?;

    let method = CostBasisMethod::Fifo; // Default
    let mut buy_count = 0;
    let mut sell_count = 0;
    let mut swap_count = 0;
    let mut transfer_count = 0;
    let mut correction_count = 0;
    // Disposals (sells) that could not be matched to tax lots. Previously these
    // were swallowed silently, hiding the realized gain. We now collect them so
    // backfill surfaces exactly which sales lack basis.
    let mut unmatched_disposals: Vec<String> = Vec::new();

    // Replay transactions
    for tx in transactions {
        match tx.tx_type.as_str() {
            "buy" => {
                if let (Some(asset), Some(qty), Some(price)) =
                    (&tx.to_asset, tx.to_quantity, tx.price_usd)
                {
                    let _ = pnl_calc
                        .process_acquisition(
                            tx.id,
                            &tx.to_account_id.unwrap_or_default(),
                            asset,
                            qty,
                            price,
                            tx.timestamp,
                            method,
                        )
                        .await;
                    buy_count += 1;
                }
            }
            "sell" => {
                if let (Some(asset), Some(qty), Some(price)) =
                    (&tx.from_asset, tx.from_quantity, tx.price_usd)
                {
                    match pnl_calc
                        .process_disposal(
                            tx.id,
                            &tx.from_account_id.clone().unwrap_or_default(),
                            asset,
                            qty,
                            price,
                            tx.timestamp,
                            method,
                        )
                        .await
                    {
                        Ok(_) => sell_count += 1,
                        // A disposal that cannot be matched to lots used to be
                        // silently discarded (`let _ = ...`), so the realized gain
                        // for that sale vanished with no trace. Surface it instead:
                        // record the shortfall so backfill reports it and the
                        // operator can supply the missing basis (correction/buy).
                        Err(e) => unmatched_disposals.push(format!(
                            "sell {} {} @ {} on {}: {}",
                            qty,
                            asset,
                            price,
                            tx.timestamp.date_naive(),
                            e
                        )),
                    }
                }
            }
            "swap" => {
                // Process as disposal + acquisition. Price the source leg at its
                // average cost basis (no historical market price is stored on a
                // swap), so the source disposal realises ~zero; the destination
                // leg inherits the source cost spread over the acquired amount.
                if let (Some(from_asset), Some(from_qty), Some(to_asset), Some(to_qty)) = (
                    &tx.from_asset,
                    tx.from_quantity,
                    &tx.to_asset,
                    tx.to_quantity,
                ) {
                    let from_account = tx.from_account_id.clone().unwrap_or_default();
                    let to_account = tx.to_account_id.clone().unwrap_or_default();

                    let disposal_price = pnl_calc
                        .average_cost_basis(&from_account, from_asset, method)
                        .await
                        .ok()
                        .flatten()
                        .or(tx.price_usd);
                    let acquisition_price = disposal_price.map(|p| p * from_qty / to_qty);

                    if let Some(price) = disposal_price {
                        let _ = pnl_calc
                            .process_disposal(
                                tx.id,
                                &from_account,
                                from_asset,
                                from_qty,
                                price,
                                tx.timestamp,
                                method,
                            )
                            .await;
                    }
                    if let Some(price) = acquisition_price {
                        let _ = pnl_calc
                            .process_acquisition(
                                tx.id,
                                &to_account,
                                to_asset,
                                to_qty,
                                price,
                                tx.timestamp,
                                method,
                            )
                            .await;
                    }
                    swap_count += 1;
                }
            }
            "transfer_internal" => {
                // A move between the user's own accounts: basis travels with the
                // coins, no realized P&L, no new lot minted.
                if let (Some(asset), Some(qty), Some(from_acc), Some(to_acc)) = (
                    tx.from_asset.as_ref().or(tx.to_asset.as_ref()),
                    tx.from_quantity.or(tx.to_quantity),
                    tx.from_account_id.as_ref(),
                    tx.to_account_id.as_ref(),
                ) {
                    // Guard: a same-account transfer (from == to) moves nothing
                    // between tracked accounts — e.g. Binance "Transfer Between
                    // Spot and Strategy", which the importer records as two
                    // half-rows on the ONE Binance account. Calling transfer_lots
                    // here mints a duplicate destination lot (phantom basis), so
                    // skip it entirely.
                    if from_acc != to_acc {
                        let _ = pnl_calc
                            .transfer_lots(from_acc, to_acc, asset, qty, method)
                            .await;
                        transfer_count += 1;
                    }
                }
            }
            "transfer_out" => {
                // If the destination is one of the user's own accounts, move the
                // lots there (basis travels).
                //
                // If the destination is EXTERNAL, the coins leave this account —
                // but in a watch-only tracker an "external" withdrawal is almost
                // always a move to the user's own self-custody wallet, and the
                // same coins frequently round-trip back (deposit) and are later
                // sold. Dropping the lots here strands that later sale with no
                // basis (FIFO then raises InsufficientTaxLots and the realized
                // gain is lost). A withdrawal is NOT a sale, so we must not
                // recognize P&L either way. We therefore PRESERVE the lots on an
                // external withdrawal: basis is only consumed by an actual
                // disposal (sell/swap), never by a transfer. This keeps a
                // withdraw -> deposit -> sell round-trip correctly matched.
                if let (Some(asset), Some(qty), Some(from_acc)) = (
                    tx.from_asset.as_ref(),
                    tx.from_quantity,
                    tx.from_account_id.as_ref(),
                ) {
                    if let Some(to_acc) = tx.to_account_id.as_ref() {
                        if internal_accounts.contains(to_acc) {
                            let _ = pnl_calc
                                .transfer_lots(from_acc, to_acc, asset, qty, method)
                                .await;
                        }
                        // else: external withdrawal — preserve lots (see above).
                    }
                    transfer_count += 1;
                }
            }
            "transfer_in" => {
                // If the source is one of the user's own accounts, the paired
                // transfer_out already moved the lots — do nothing to avoid double
                // counting.
                //
                // If the source is external, this is a deposit. Only mint a lot
                // when the row carries a REAL stated acquisition price (price_usd
                // > 0): that represents coins genuinely entering the tracked set
                // with a known basis. A PRICELESS external deposit is almost
                // always a self-custody -> exchange move of coins already owned
                // (their basis lives at the origin, recorded via a WACB `buy` or
                // supplied by a `correction`). Minting a $0-cost lot for it is
                // wrong: FIFO then disposes that phantom lot first and reports a
                // near-100% "gain". So we skip it.
                let external = tx
                    .from_account_id
                    .as_ref()
                    .map(|a| !internal_accounts.contains(a))
                    .unwrap_or(true);
                if external {
                    if let (Some(asset), Some(qty), Some(to_acc), Some(price)) = (
                        tx.to_asset.as_ref(),
                        tx.to_quantity,
                        tx.to_account_id.as_ref(),
                        tx.price_usd.filter(|p| *p > Decimal::ZERO),
                    ) {
                        let _ = pnl_calc
                            .process_acquisition(
                                tx.id,
                                to_acc,
                                asset,
                                qty,
                                price,
                                tx.timestamp,
                                method,
                            )
                            .await;
                        transfer_count += 1;
                    }
                }
            }
            "correction" => {
                // Append-only corrections adjust the rebuilt lots:
                //   from_asset+from_quantity  -> REMOVE that qty of lots (unwind an
                //                                 over-recorded acquisition), no P&L.
                //   to_asset+to_quantity+price -> ADD a lot (supply missing basis).
                if let (Some(asset), Some(qty), Some(from_acc)) = (
                    tx.from_asset.as_ref(),
                    tx.from_quantity,
                    tx.from_account_id.as_ref(),
                ) {
                    let _ = pnl_calc.remove_lots(from_acc, asset, qty, method).await;
                    correction_count += 1;
                }
                if let (Some(asset), Some(qty), Some(to_acc)) = (
                    tx.to_asset.as_ref(),
                    tx.to_quantity,
                    tx.to_account_id.as_ref(),
                ) {
                    let price = tx.price_usd.unwrap_or(Decimal::ZERO);
                    let _ = pnl_calc
                        .process_acquisition(tx.id, to_acc, asset, qty, price, tx.timestamp, method)
                        .await;
                    correction_count += 1;
                }
            }
            _ => {} // stake/unstake/earn/airdrop/fee handled elsewhere or not lot-affecting
        }
    }

    success(&format!(
        "Backfill complete! Processed {buy_count} buys, {sell_count} sells, \
         {swap_count} swaps, {transfer_count} transfers, {correction_count} corrections"
    ));

    // Sync the holdings' avg_cost_basis from the rebuilt tax lots so the
    // portfolio cost-basis headline reflects the authoritative P&L basis.
    // Chain-synced holdings previously had NULL avg_cost_basis and were silently
    // excluded from the cost-basis total (understating cost, inflating P&L%).
    let cost_synced = sync_holding_cost_basis(&holding_repo, &pnl_calc, method).await?;
    if cost_synced > 0 && !opts.quiet {
        info(&format!(
            "Synced cost basis from tax lots for {cost_synced} holding(s)"
        ));
    }

    if !unmatched_disposals.is_empty() {
        warning(&format!(
            "{} disposal(s) could NOT be matched to tax lots — their realized P&L \
             is MISSING. Supply the missing basis (a priced `buy`/`correction` lot \
             dated before the sale) and re-run:",
            unmatched_disposals.len()
        ));
        for d in &unmatched_disposals {
            warning(&format!("  • {d}"));
        }
    }

    Ok(())
}

/// Update each holding's `avg_cost_basis` to the average cost of its rebuilt tax
/// lots. Holdings without lots (chain-synced balances with no acquisition record)
/// are left untouched so a manually-set basis is preserved. Returns the number of
/// holdings updated.
async fn sync_holding_cost_basis(
    holding_repo: &HoldingRepository<'_>,
    pnl_calc: &PnLCalculator<'_>,
    method: CostBasisMethod,
) -> Result<usize> {
    let holdings = holding_repo.list_all().await?;
    let mut updated = 0usize;
    for h in holdings {
        if h.quantity <= Decimal::ZERO {
            continue;
        }
        if let Some(avg) = pnl_calc
            .average_cost_basis(&h.account_id, &h.asset, method)
            .await?
        {
            holding_repo
                .set_quantity(&h.account_id, &h.asset, h.quantity, Some(avg))
                .await?;
            updated += 1;
        }
    }
    Ok(updated)
}

async fn calculate_total_unrealized(
    account: Option<&str>,
    asset: Option<&str>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<Decimal> {
    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;
    let holding_repo = HoldingRepository::new(pool);
    let pnl_calc = PnLCalculator::new(pool);

    // Get holdings
    let holdings = if let Some(acc) = account {
        holding_repo.list_by_account(acc).await?
    } else {
        holding_repo.list_all().await?
    };

    let filtered_holdings: Vec<_> = if let Some(ast) = asset {
        holdings
            .into_iter()
            .filter(|h| h.asset.eq_ignore_ascii_case(ast))
            .collect()
    } else {
        holdings
    };

    if filtered_holdings.is_empty() {
        return Ok(Decimal::ZERO);
    }

    // Fetch current prices through the SHARED valuation pipeline (classify DeFi
    // receipts and price their underlying, Binance + Alpha, stablecoin peg, LST
    // on-chain rate). Using the same path as `portfolio` is what keeps the two
    // commands in agreement; the old per-raw-symbol `get_price` loop here priced
    // receipt tokens by their own symbol and produced phantom multi-billion
    // unrealized figures.
    let price_map =
        crate::core::pricing::build_price_map(&filtered_holdings, &config, use_testnet).await;

    let mut total_unrealized = Decimal::ZERO;

    for holding in &filtered_holdings {
        if let Some(current_price) = crate::core::pricing::price_for_holding(holding, &price_map) {
            let unrealized_pnl = pnl_calc
                .calculate_unrealized_pnl(&holding.account_id, &holding.asset, current_price)
                .await
                .unwrap_or(Decimal::ZERO);

            total_unrealized += unrealized_pnl;
        }
    }

    Ok(total_unrealized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_pnl_output_serializes_decimals_as_strings() {
        let output = AssetPnlOutput {
            asset: "BTC".to_string(),
            account: None,
            realized_transactions: 3,
            total_realized: "1234.5".to_string(),
            total_unrealized: "-67.89".to_string(),
            net_pnl: "1166.61".to_string(),
        };
        let json = serde_json::to_string(&output).expect("AssetPnlOutput must serialize");
        assert!(json.contains("\"total_realized\":\"1234.5\""));
        assert!(json.contains("\"total_unrealized\":\"-67.89\""));
        assert!(json.contains("\"realized_transactions\":3"));
        assert!(json.contains("\"asset\":\"BTC\""));
    }
}
