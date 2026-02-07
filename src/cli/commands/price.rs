use colored::Colorize;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::cli::output::{format_usd, print_header, print_row, warning};
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::error::Result;
use crate::exchange::{BinanceAlphaClient, BinanceClient, Exchange};

#[derive(Serialize)]
struct PriceOutput {
    symbol: String,
    price: String,
}

pub async fn handle_price_command(symbols: Vec<String>, _pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;

    let client = BinanceClient::new(
        use_testnet,
        config.binance.api_key.clone(),
        config.binance.api_secret.clone(),
    );

    if !opts.quiet && use_testnet {
        warning("Testnet Mode");
    }

    let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
    let mut prices = client.get_prices(&symbol_refs).await?;
    let mut found_symbols: Vec<String> = prices.iter().map(|p| p.symbol.to_uppercase()).collect();

    // Try Binance Alpha for missing symbols
    let missing_symbols: Vec<&str> = symbols
        .iter()
        .filter(|s| !found_symbols.contains(&s.to_uppercase()))
        .map(|s| s.as_str())
        .collect();

    if !missing_symbols.is_empty() {
        let alpha_client = BinanceAlphaClient::new();
        if let Ok(alpha_prices) = alpha_client.get_prices(&missing_symbols).await {
            for (symbol, price) in alpha_prices {
                prices.push(crate::exchange::PriceData {
                    symbol: symbol.clone(),
                    price,
                });
                found_symbols.push(symbol);
            }
        }
    }

    if opts.json {
        // JSON output
        let output: Vec<PriceOutput> = prices
            .iter()
            .map(|p| PriceOutput {
                symbol: p.symbol.clone(),
                price: p.price.to_string(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    } else if symbols.len() == 1 && prices.len() == 1 {
        // Single symbol - simple output
        let price = &prices[0];
        println!("{}: {}", price.symbol.bold(), format_usd(price.price));
    } else {
        // Multiple symbols - table output
        print_header(&[("Symbol", 10), ("Price", 15)]);

        for price in &prices {
            print_row(&[
                (&price.symbol, 10),
                (&format_usd(price.price), 15),
            ]);
        }

        // Show any symbols that weren't found
        for symbol in &symbols {
            if !found_symbols.contains(&symbol.to_uppercase()) {
                println!("{}: {}", symbol.to_uppercase(), "Not found".red());
            }
        }
    }

    Ok(())
}
