use colored::Colorize;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::cli::output::{format_pnl, format_pnl_percent, format_quantity, format_usd, warning};
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::core::holdings::HoldingWithPrice;
use crate::core::portfolio::{Portfolio, PortfolioEntry};
use crate::db::{AccountRepository, HoldingRepository};
use crate::error::Result;

#[derive(Serialize)]
struct PortfolioOutput {
    total_value_usd: String,
    total_cost_basis: String,
    unrealized_pnl: String,
    unrealized_pnl_percent: String,
    entries: Vec<PortfolioEntryOutput>,
}

#[derive(Serialize)]
struct PortfolioEntryOutput {
    account_name: String,
    category_name: String,
    holdings: Vec<HoldingOutput>,
}

#[derive(Serialize)]
struct HoldingOutput {
    asset: String,
    quantity: String,
    current_price: Option<String>,
    current_value: Option<String>,
    cost_basis: Option<String>,
    unrealized_pnl: Option<String>,
    unrealized_pnl_percent: Option<String>,
    /// "plain" | "supply" | "debt" — debt positions carry a negative value.
    defi_kind: String,
}

pub async fn handle_portfolio_command(
    by_account: bool,
    by_category: bool,
    account: Option<String>,
    category: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;
    let account_repo = AccountRepository::new(pool);
    let holding_repo = HoldingRepository::new(pool);

    // Fetch all accounts and holdings
    let accounts = account_repo.list_accounts().await?;
    let categories = account_repo.list_categories().await?;

    if accounts.is_empty() {
        println!("No accounts configured. Use 'cryptofolio account add' to create one.");
        return Ok(());
    }

    // Create category lookup
    let category_map: HashMap<String, String> = categories
        .iter()
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();

    // All holdings — the shared pricer derives the symbols it needs (including
    // the underlying of every DeFi/Earn receipt) from this set.
    let all_holdings = holding_repo.list_all().await?;

    // Build the price map through the SHARED valuation pipeline (classify DeFi
    // receipts -> price underlying, Binance + Alpha, stablecoin peg, LST on-chain
    // rate). `pnl summary` uses the exact same path, so the two commands never
    // disagree on a holding's USD value.
    let price_map =
        crate::core::pricing::build_price_map(&all_holdings, &config, use_testnet).await;

    // Build portfolio entries
    let mut entries: Vec<PortfolioEntry> = Vec::new();

    for acc in &accounts {
        // Apply filters
        if let Some(ref filter_account) = account {
            if acc.name.to_lowercase() != filter_account.to_lowercase() {
                continue;
            }
        }

        if let Some(ref filter_category) = category {
            let cat_name = category_map
                .get(&acc.category_id)
                .cloned()
                .unwrap_or_default();
            if cat_name.to_lowercase() != filter_category.to_lowercase() {
                continue;
            }
        }

        let holdings = holding_repo.list_by_account(&acc.id).await?;
        let holdings_with_price: Vec<HoldingWithPrice> = holdings
            .into_iter()
            .map(|h| {
                let classified = crate::core::defi::classify(&h.asset);
                let price = price_map
                    .get(&classified.underlying.to_uppercase())
                    .copied();
                HoldingWithPrice::from_holding_defi(h, &classified, price)
            })
            .collect();

        if !holdings_with_price.is_empty() {
            entries.push(PortfolioEntry {
                account_id: acc.id.clone(),
                account_name: acc.name.clone(),
                category_id: acc.category_id.clone(),
                category_name: category_map
                    .get(&acc.category_id)
                    .cloned()
                    .unwrap_or_else(|| "-".to_string()),
                holdings: holdings_with_price,
            });
        }
    }

    let portfolio = Portfolio::from_entries(entries);

    if portfolio.entries.is_empty() {
        println!("No holdings found.");
        return Ok(());
    }

    // JSON output
    if opts.json {
        let output = PortfolioOutput {
            total_value_usd: portfolio.total_value_usd.to_string(),
            total_cost_basis: portfolio.total_cost_basis.to_string(),
            unrealized_pnl: portfolio.unrealized_pnl.to_string(),
            unrealized_pnl_percent: portfolio.unrealized_pnl_percent.to_string(),
            entries: portfolio
                .entries
                .iter()
                .filter(|e| has_visible_holding(e, &config))
                .map(|e| PortfolioEntryOutput {
                    account_name: e.account_name.clone(),
                    category_name: e.category_name.clone(),
                    holdings: e
                        .holdings
                        .iter()
                        .filter(|h| is_visible_holding(h, &config))
                        .map(|h| HoldingOutput {
                            asset: h.holding.asset.clone(),
                            quantity: h.holding.quantity.to_string(),
                            current_price: h.current_price.map(|p| p.to_string()),
                            current_value: h.current_value.map(|v| v.to_string()),
                            cost_basis: h.holding.avg_cost_basis.map(|c| c.to_string()),
                            unrealized_pnl: h.unrealized_pnl.map(|p| p.to_string()),
                            unrealized_pnl_percent: h.unrealized_pnl_percent.map(|p| p.to_string()),
                            defi_kind: match h.defi_kind {
                                crate::core::defi::DefiKind::Plain => "plain",
                                crate::core::defi::DefiKind::Supply => "supply",
                                crate::core::defi::DefiKind::Debt => "debt",
                            }
                            .to_string(),
                        })
                        .collect(),
                })
                .collect(),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
        return Ok(());
    }

    // Print header
    println!();
    if use_testnet && !opts.quiet {
        warning("Testnet Mode");
    }

    println!("{}", "PORTFOLIO OVERVIEW".bold());
    println!("{}", "=".repeat(70));
    println!();

    println!(
        "  Total Value:     {}",
        format_usd(portfolio.total_value_usd).bold()
    );
    println!(
        "{}",
        format_cost_basis_headline(&portfolio, config.display.color)
    );
    println!();

    if by_category {
        // Group by category
        let category_summaries = portfolio.by_category();

        for summary in category_summaries {
            let visible_accounts: Vec<&PortfolioEntry> = summary
                .accounts
                .iter()
                .filter(|e| has_visible_holding(e, &config))
                .collect();
            if visible_accounts.is_empty() {
                continue;
            }

            println!(
                "{}",
                format!(
                    "  {} [{}]",
                    summary.category_name,
                    format_usd(summary.total_value)
                )
                .bold()
            );

            for entry in visible_accounts {
                println!(
                    "    {} ({})",
                    entry.account_name,
                    format_usd(entry.total_value())
                );

                for h in entry
                    .holdings
                    .iter()
                    .filter(|h| is_visible_holding(h, &config))
                {
                    print_holding(h, &config, 6);
                }
            }
            println!();
        }
    } else if by_account {
        // Group by account
        for entry in &portfolio.entries {
            if !has_visible_holding(entry, &config) {
                continue;
            }

            println!(
                "  {} [{}]",
                entry.account_name.bold(),
                format_usd(entry.total_value())
            );

            for h in entry
                .holdings
                .iter()
                .filter(|h| is_visible_holding(h, &config))
            {
                print_holding(h, &config, 4);
            }
            println!();
        }
    } else {
        // Default: flat list grouped by account
        println!("{}", "-".repeat(70));
        println!(
            "  {:8}  {:>12}  {:>12}  {:>12}  {:>15}",
            "Asset", "Quantity", "Price", "Value", "P&L"
        );
        println!("{}", "-".repeat(70));

        for entry in &portfolio.entries {
            if !has_visible_holding(entry, &config) {
                continue;
            }

            println!("  {}", entry.account_name.dimmed());

            for h in entry
                .holdings
                .iter()
                .filter(|h| is_visible_holding(h, &config))
            {
                let price_str = h
                    .current_price
                    .map(format_usd)
                    .unwrap_or_else(|| "-".to_string());

                let value_str = h
                    .current_value
                    .map(format_usd)
                    .unwrap_or_else(|| "-".to_string());

                let pnl_str = match (h.unrealized_pnl, h.unrealized_pnl_percent) {
                    (Some(pnl), Some(pct)) => format!(
                        "{} ({})",
                        format_pnl(pnl, config.display.color),
                        format_pnl_percent(pct, config.display.color)
                    ),
                    _ => "-".to_string(),
                };

                println!(
                    "  {:16}  {:>12}  {:>12}  {:>12}  {:>15}",
                    crate::core::defi::display_label(&h.holding.asset),
                    format_quantity(h.holding.quantity),
                    price_str,
                    value_str,
                    pnl_str
                );
            }
        }

        println!("{}", "-".repeat(70));
    }

    // Asset totals
    // Display-only: hidden symbols are skipped here too, but `asset_totals()`
    // itself (and every total above) still values the full portfolio.
    let asset_totals: Vec<crate::core::portfolio::AssetTotal> = portfolio
        .asset_totals()
        .into_iter()
        .filter(|t| !config.display.is_hidden(&t.asset))
        .collect();
    if !asset_totals.is_empty() {
        println!();
        println!("{}", "ASSET TOTALS".bold());
        print!(" ");
        for (i, total) in asset_totals.iter().take(5).enumerate() {
            if i > 0 {
                print!("  |  ");
            }
            print!(
                "{}: {} ({})",
                crate::core::defi::display_label(&total.asset),
                format_quantity(total.quantity),
                format_usd(total.value)
            );
        }
        println!();
    }

    println!();

    Ok(())
}

/// Whether a holding is shown under `[display] hidden_assets`.
///
/// Presentation-only: the portfolio (and its totals) is still built from every
/// holding; this only decides what gets rendered.
fn is_visible_holding(h: &HoldingWithPrice, config: &AppConfig) -> bool {
    !config.display.is_hidden(&h.holding.asset)
}

/// Whether an account entry has at least one holding left to display.
fn has_visible_holding(entry: &PortfolioEntry, config: &AppConfig) -> bool {
    entry.holdings.iter().any(|h| is_visible_holding(h, config))
}

/// Render the cost-basis / unrealized-P&L headline shown above the holdings
/// table. Pure (no I/O) so it can be unit-tested; `color` mirrors
/// `config.display.color` and is passed through to the P&L helpers.
///
/// The percentage is division-by-zero safe: `Portfolio::from_entries` already
/// computes `unrealized_pnl_percent` as `Decimal::ZERO` when
/// `total_cost_basis <= 0`, so a zero basis renders as `+0.00%` rather than
/// panicking or producing NaN.
fn format_cost_basis_headline(portfolio: &Portfolio, color: bool) -> String {
    format!(
        "  Cost Basis:      {}  |  Unrealized P&L:  {} ({})",
        format_usd(portfolio.total_cost_basis),
        format_pnl(portfolio.unrealized_pnl, color),
        format_pnl_percent(portfolio.unrealized_pnl_percent, color)
    )
}

fn print_holding(h: &HoldingWithPrice, config: &AppConfig, indent: usize) {
    let spaces = " ".repeat(indent);

    let price_str = h
        .current_price
        .map(format_usd)
        .unwrap_or_else(|| "-".to_string());

    let value_str = h
        .current_value
        .map(format_usd)
        .unwrap_or_else(|| "-".to_string());

    let pnl_str = h
        .unrealized_pnl
        .map(|pnl| format_pnl(pnl, config.display.color))
        .unwrap_or_else(|| "-".to_string());

    println!(
        "{}{}: {} @ {} = {} ({})",
        spaces,
        h.holding.asset,
        format_quantity(h.holding.quantity),
        price_str,
        value_str,
        pnl_str
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::defi::DefiKind;
    use crate::core::holdings::Holding;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// Build a single-holding portfolio. `cost_basis` is `None` to model a
    /// holding with unknown basis (contributes 0 to the portfolio cost basis).
    fn portfolio_with(cost_basis: Option<Decimal>, current_value: Decimal) -> Portfolio {
        let quantity = Decimal::ONE;
        let holding = Holding {
            id: 1,
            account_id: "acc".to_string(),
            asset: "BTC".to_string(),
            quantity,
            avg_cost_basis: cost_basis,
            cost_basis_currency: Some("USD".to_string()),
            avg_cost_basis_base: cost_basis,
            updated_at: Utc::now(),
        };
        let holding_with_price = HoldingWithPrice {
            holding,
            current_price: Some(current_value),
            current_value: Some(current_value),
            unrealized_pnl: cost_basis.map(|c| current_value - c * quantity),
            unrealized_pnl_percent: cost_basis
                .filter(|c| *c > Decimal::ZERO)
                .map(|c| ((current_value - c * quantity) / (c * quantity)) * Decimal::from(100)),
            defi_kind: DefiKind::Plain,
        };
        Portfolio::from_entries(vec![PortfolioEntry {
            account_id: "acc".to_string(),
            account_name: "Acc".to_string(),
            category_id: "cat".to_string(),
            category_name: "Cat".to_string(),
            holdings: vec![holding_with_price],
        }])
    }

    #[test]
    fn headline_includes_cost_basis_and_signed_unrealized_pnl() {
        let portfolio = portfolio_with(Some(Decimal::from(100)), Decimal::from(150));
        let line = format_cost_basis_headline(&portfolio, false);
        assert_eq!(
            line,
            "  Cost Basis:      $100.00  |  Unrealized P&L:  +$50.00 (+50.00%)"
        );
    }

    #[test]
    fn headline_negative_pnl_has_no_plus_sign() {
        let portfolio = portfolio_with(Some(Decimal::from(200)), Decimal::from(150));
        let line = format_cost_basis_headline(&portfolio, false);
        assert_eq!(
            line,
            "  Cost Basis:      $200.00  |  Unrealized P&L:  $-50.00 (-25.00%)"
        );
    }

    #[test]
    fn headline_zero_cost_basis_shows_zero_percent_not_a_panic() {
        // No cost basis -> `Portfolio::from_entries` guards the division and
        // stores `unrealized_pnl_percent = 0`; the headline must render that.
        let portfolio = portfolio_with(None, Decimal::from(150));
        assert_eq!(portfolio.total_cost_basis, Decimal::ZERO);
        assert_eq!(portfolio.unrealized_pnl_percent, Decimal::ZERO);
        let line = format_cost_basis_headline(&portfolio, false);
        assert_eq!(
            line,
            "  Cost Basis:      $0.00  |  Unrealized P&L:  +$150.00 (+0.00%)"
        );
    }

    /// Synthetic placeholder symbols only — never real airdrop tokens.
    fn holding_named(asset: &str) -> HoldingWithPrice {
        HoldingWithPrice {
            holding: Holding {
                id: 1,
                account_id: "acc".to_string(),
                asset: asset.to_string(),
                quantity: Decimal::ONE,
                avg_cost_basis: None,
                cost_basis_currency: None,
                avg_cost_basis_base: None,
                updated_at: Utc::now(),
            },
            current_price: None,
            current_value: None,
            unrealized_pnl: None,
            unrealized_pnl_percent: None,
            defi_kind: DefiKind::Plain,
        }
    }

    fn config_with_hidden(symbols: &[&str]) -> AppConfig {
        let mut config = AppConfig::default();
        config.display.hidden_assets = symbols.iter().map(|s| s.to_string()).collect();
        config
    }

    #[test]
    fn is_visible_holding_hides_configured_symbol_case_insensitively() {
        let config = config_with_hidden(&["SCAMTOKENA"]);
        assert!(!is_visible_holding(&holding_named("scamtokena"), &config));
        assert!(!is_visible_holding(&holding_named("ScamTokenA"), &config));
        assert!(is_visible_holding(&holding_named("SCAMTOKENB"), &config));
        assert!(is_visible_holding(&holding_named("BTC"), &config));
    }

    #[test]
    fn is_visible_holding_empty_list_shows_everything() {
        let config = AppConfig::default();
        assert!(is_visible_holding(&holding_named("SCAMTOKENA"), &config));
    }

    #[test]
    fn has_visible_holding_is_false_only_when_every_row_is_hidden() {
        let entry = |assets: &[&str]| PortfolioEntry {
            account_id: "acc".to_string(),
            account_name: "Acc".to_string(),
            category_id: "cat".to_string(),
            category_name: "Cat".to_string(),
            holdings: assets.iter().map(|a| holding_named(a)).collect(),
        };
        let config = config_with_hidden(&["SCAMTOKENA", "SCAMTOKENB"]);

        assert!(!has_visible_holding(
            &entry(&["SCAMTOKENA", "SCAMTOKENB"]),
            &config
        ));
        assert!(has_visible_holding(&entry(&["SCAMTOKENA", "BTC"]), &config));
        assert!(has_visible_holding(&entry(&["BTC"]), &config));
    }

    #[test]
    fn hidden_assets_do_not_change_portfolio_totals() {
        // Deliberate: the filter is presentation-only. The `Portfolio` is still
        // built from every holding (exactly as before this feature), so a
        // hidden symbol keeps contributing to the totals — no valuation,
        // cost-basis, or P&L change. Only rendering is filtered.
        let config = config_with_hidden(&["SCAMTOKENA"]);

        let mut hidden = holding_named("SCAMTOKENA");
        hidden.current_value = Some(Decimal::from(999));
        let portfolio = Portfolio::from_entries(vec![PortfolioEntry {
            account_id: "acc".to_string(),
            account_name: "Acc".to_string(),
            category_id: "cat".to_string(),
            category_name: "Cat".to_string(),
            holdings: vec![hidden, holding_named("BTC")],
        }]);

        assert!(!is_visible_holding(
            &portfolio.entries[0].holdings[0],
            &config
        ));
        assert_eq!(portfolio.total_value_usd, Decimal::from(999));
    }
}
