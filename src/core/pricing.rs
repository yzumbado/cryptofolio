//! Shared holding valuation — ONE source of truth for USD pricing.
//!
//! Both `portfolio` and `pnl summary` must value holdings identically, or the
//! two commands disagree (historically `pnl summary` priced each raw symbol
//! directly and reported a multi-billion-dollar phantom, while `portfolio`
//! classified DeFi receipts and pegged stablecoins correctly). This module
//! builds the price map once, applying the full chain:
//!
//!   1. Classify each holding (Aave aToken / debt / Binance Earn `LD*`) and
//!      price the UNDERLYING, not the receipt symbol.
//!   2. Binance spot, then Binance Alpha for long-tail symbols.
//!   3. Stablecoin $1.00 peg for USD-pegged assets with no feed.
//!   4. Liquid-staking tokens (rETH, wstETH) from their LIVE on-chain
//!      ETH-exchange rate × ETH price (needs an Etherscan key; skipped silently
//!      when unavailable).
//!
//! Watch-only: no keys beyond read-only API access, no signing.

use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::config::AppConfig;
use crate::core::defi;
use crate::core::holdings::Holding;
use crate::exchange::{BinanceAlphaClient, BinanceClient, Exchange};

/// Build a `UPPERCASE symbol -> USD price` map covering every holding and the
/// underlying of every DeFi/Earn receipt token among them.
///
/// Prices that cannot be resolved by any source are simply absent from the map;
/// callers treat a missing price as "unvalued" rather than zero.
pub async fn build_price_map(
    holdings: &[Holding],
    config: &AppConfig,
    use_testnet: bool,
) -> HashMap<String, Decimal> {
    // Every symbol we need: the raw holding symbol AND its classified underlying.
    let unique_assets: Vec<String> = holdings
        .iter()
        .flat_map(|h| {
            let classified = defi::classify(&h.asset);
            vec![h.asset.clone(), classified.underlying]
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

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

    // Binance Alpha for long-tail symbols with no spot pair.
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

    // Stablecoin $1.00 peg for USD-pegged assets with no feed.
    for asset in &unique_assets {
        let up = asset.to_uppercase();
        if let std::collections::hash_map::Entry::Vacant(e) = price_map.entry(up) {
            if let Some(peg) = defi::stablecoin_peg(e.key()) {
                e.insert(peg);
            }
        }
    }

    // Liquid-staking tokens from live on-chain rate × ETH price.
    if let Some(eth_price) = price_map.get("ETH").copied() {
        let etherscan_key = std::env::var("ETHERSCAN_API_KEY")
            .ok()
            .or_else(|| config.get_etherscan_api_key());
        if etherscan_key.is_some() {
            use crate::blockchain::ethereum::EtherscanClient;
            let eth_client = EtherscanClient::new(use_testnet, etherscan_key);
            for asset in &unique_assets {
                let up = asset.to_uppercase();
                if price_map.contains_key(&up) {
                    continue;
                }
                if let Some(lst) = defi::lst_info(&up) {
                    if let Ok(rate) = eth_client
                        .get_lst_eth_rate(lst.contract, lst.rate_selector)
                        .await
                    {
                        price_map.insert(up, rate * eth_price);
                    }
                }
            }
        }
    }

    price_map
}

/// Resolve the USD price for a single holding, applying DeFi/Earn classification
/// so an aToken/debt/`LD*` symbol is priced by its underlying. Returns `None`
/// when no source priced the (underlying) symbol.
pub fn price_for_holding(
    holding: &Holding,
    price_map: &HashMap<String, Decimal>,
) -> Option<Decimal> {
    let classified = defi::classify(&holding.asset);
    price_map
        .get(&classified.underlying.to_uppercase())
        .copied()
}
