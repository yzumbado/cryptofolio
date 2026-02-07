use colored::Colorize;
use rust_decimal::Decimal;
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
use crate::exchange::{BinanceAlphaClient, BinanceClient, Exchange};

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

    // Collect all unique assets
    let all_holdings = holding_repo.list_all().await?;
    let unique_assets: Vec<String> = all_holdings
        .iter()
        .map(|h| h.asset.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Fetch prices
    let client = BinanceClient::new(
        use_testnet,
        config.binance.api_key.clone(),
        config.binance.api_secret.clone(),
    );

    let asset_refs: Vec<&str> = unique_assets.iter().map(|s| s.as_str()).collect();
    let prices = client.get_prices(&asset_refs).await.unwrap_or_default();

    let mut price_map: HashMap<String, Decimal> = prices
        .into_iter()
        .map(|p| (p.symbol.to_uppercase(), p.price))
        .collect();

    // Find assets without prices and try Binance Alpha API
    let missing_assets: Vec<&str> = unique_assets
        .iter()
        .filter(|a| !price_map.contains_key(&a.to_uppercase()))
        .map(|s| s.as_str())
        .collect();

    if !missing_assets.is_empty() {
        let alpha_client = BinanceAlphaClient::new();
        if let Ok(alpha_prices) = alpha_client.get_prices(&missing_assets).await {
            for (symbol, price) in alpha_prices {
                price_map.insert(symbol, price);
            }
        }
    }

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
            let cat_name = category_map.get(&acc.category_id).cloned().unwrap_or_default();
            if cat_name.to_lowercase() != filter_category.to_lowercase() {
                continue;
            }
        }

        let holdings = holding_repo.list_by_account(&acc.id).await?;
        let holdings_with_price: Vec<HoldingWithPrice> = holdings
            .into_iter()
            .map(|h| {
                let price = price_map.get(&h.asset.to_uppercase()).copied();
                HoldingWithPrice::from_holding(h, price)
            })
            .collect();

        if !holdings_with_price.is_empty() {
            entries.push(PortfolioEntry {
                account_id: acc.id.clone(),
                account_name: acc.name.clone(),
                category_id: acc.category_id.clone(),
                category_name: category_map.get(&acc.category_id).cloned().unwrap_or_else(|| "-".to_string()),
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
            entries: portfolio.entries.iter().map(|e| PortfolioEntryOutput {
                account_name: e.account_name.clone(),
                category_name: e.category_name.clone(),
                holdings: e.holdings.iter().map(|h| HoldingOutput {
                    asset: h.holding.asset.clone(),
                    quantity: h.holding.quantity.to_string(),
                    current_price: h.current_price.map(|p| p.to_string()),
                    current_value: h.current_value.map(|v| v.to_string()),
                    cost_basis: h.holding.avg_cost_basis.map(|c| c.to_string()),
                    unrealized_pnl: h.unrealized_pnl.map(|p| p.to_string()),
                    unrealized_pnl_percent: h.unrealized_pnl_percent.map(|p| p.to_string()),
                }).collect(),
            }).collect(),
        };
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
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

    println!("  Total Value:     {}", format_usd(portfolio.total_value_usd).bold());
    println!("  Cost Basis:      {}", format_usd(portfolio.total_cost_basis));
    println!(
        "  Unrealized P&L:  {} ({})",
        format_pnl(portfolio.unrealized_pnl, config.display.color),
        format_pnl_percent(portfolio.unrealized_pnl_percent, config.display.color)
    );
    println!();

    if by_category {
        // Group by category
        let category_summaries = portfolio.by_category();

        for summary in category_summaries {
            println!("{}", format!("  {} [{}]", summary.category_name, format_usd(summary.total_value)).bold());

            for entry in &summary.accounts {
                println!("    {} ({})", entry.account_name, format_usd(entry.total_value()));

                for h in &entry.holdings {
                    print_holding(h, &config, 6);
                }
            }
            println!();
        }
    } else if by_account {
        // Group by account
        for entry in &portfolio.entries {
            println!(
                "  {} [{}]",
                entry.account_name.bold(),
                format_usd(entry.total_value())
            );

            for h in &entry.holdings {
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
            println!("  {}", entry.account_name.dimmed());

            for h in &entry.holdings {
                let price_str = h.current_price
                    .map(|p| format_usd(p))
                    .unwrap_or_else(|| "-".to_string());

                let value_str = h.current_value
                    .map(|v| format_usd(v))
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
                    "  {:8}  {:>12}  {:>12}  {:>12}  {:>15}",
                    h.holding.asset,
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
    let asset_totals = portfolio.asset_totals();
    if !asset_totals.is_empty() {
        println!();
        println!("{}", "ASSET TOTALS".bold());
        print!(" ");
        for (i, total) in asset_totals.iter().take(5).enumerate() {
            if i > 0 {
                print!("  |  ");
            }
            print!("{}: {} ({})", total.asset, format_quantity(total.quantity), format_usd(total.value));
        }
        println!();
    }

    println!();

    Ok(())
}

fn print_holding(h: &HoldingWithPrice, config: &AppConfig, indent: usize) {
    let spaces = " ".repeat(indent);

    let price_str = h.current_price
        .map(|p| format_usd(p))
        .unwrap_or_else(|| "-".to_string());

    let value_str = h.current_value
        .map(|v| format_usd(v))
        .unwrap_or_else(|| "-".to_string());

    let pnl_str = h.unrealized_pnl
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
