use colored::Colorize;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::cli::output::{format_price_change, format_quantity, format_usd, print_kv, print_section, warning};
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::error::Result;
use crate::exchange::{BinanceClient, Exchange};

#[derive(Serialize)]
struct MarketOutput {
    symbol: String,
    base_asset: String,
    quote_asset: String,
    price: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ticker_24h: Option<Ticker24hOutput>,
}

#[derive(Serialize)]
struct Ticker24hOutput {
    price_change: String,
    price_change_percent: String,
    high_24h: String,
    low_24h: String,
    volume: String,
    quote_volume: String,
}

pub async fn handle_market_command(symbol: String, show_24h: bool, _pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
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

    let market = client.get_market_data(&symbol).await?;

    if opts.json {
        let output = MarketOutput {
            symbol: format!("{}{}", market.base_asset, market.quote_asset),
            base_asset: market.base_asset.clone(),
            quote_asset: market.quote_asset.clone(),
            price: market.price.to_string(),
            ticker_24h: if show_24h {
                market.ticker_24h.as_ref().map(|t| Ticker24hOutput {
                    price_change: t.price_change.to_string(),
                    price_change_percent: t.price_change_percent.to_string(),
                    high_24h: t.high_24h.to_string(),
                    low_24h: t.low_24h.to_string(),
                    volume: t.volume.to_string(),
                    quote_volume: t.quote_volume.to_string(),
                })
            } else {
                None
            },
        };
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    } else {
        println!();
        println!("{}", format!("{} / {}", market.base_asset, market.quote_asset).bold());
        println!();

        print_kv("Price", &format_usd(market.price));

        if show_24h {
            if let Some(ticker) = &market.ticker_24h {
                print_section("24h Statistics");

                let change_str = format_price_change(ticker.price_change, ticker.price_change_percent, config.display.color);
                print_kv("Change", &change_str);
                print_kv("High", &format_usd(ticker.high_24h));
                print_kv("Low", &format_usd(ticker.low_24h));
                print_kv("Volume", &format!("{} {}", format_quantity(ticker.volume), market.base_asset));
                print_kv("Quote Volume", &format_usd(ticker.quote_volume));
            }
        }

        println!();
    }

    Ok(())
}
