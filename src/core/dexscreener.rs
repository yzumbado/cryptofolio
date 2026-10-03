//! DexScreener price fallback for long-tail / DePIN tokens.
//!
//! Jupiter's strict list, Binance spot/Alpha, the stablecoin peg and the LST
//! rate chain cover mainstream assets. DePIN reward tokens (Geodnet GEOD,
//! Wingbits WINGS, …) are priced by none of them, so without this fallback they
//! show as "unvalued" in the portfolio total.
//!
//! DexScreener's public token endpoint takes a token CONTRACT/MINT address (not
//! a symbol) and returns the pairs it trades in; we take the first pair's USD
//! price. Read-only, no key, watch-only compliant.

use rust_decimal::Decimal;
use std::str::FromStr;

/// Map a known long-tail token SYMBOL (uppercase) to its canonical mint/contract
/// so DexScreener can be queried by address. Symbol-only DexScreener search is
/// ambiguous (many scams reuse a ticker), so we pin the exact mint.
fn mint_for_symbol(symbol_upper: &str) -> Option<&'static str> {
    match symbol_upper {
        "GEOD" => Some("7JA5eZdCzztSfQbJvS8aVVxMFfd81Rs9VvwnocV1mKHu"),
        "WINGS" => Some("WingsAYbfs4qnEgcw8jpSvetqp8XHM3GkKvow54WLcd"),
        _ => None,
    }
}

/// Fetch the USD price for a token by its mint/contract address from DexScreener.
/// Returns `None` on any network/parse failure or when no pair is listed — the
/// caller treats a missing price as "unvalued", never zero.
pub async fn price_by_mint(mint: &str) -> Option<Decimal> {
    let url = format!("https://api.dexscreener.com/latest/dex/tokens/{mint}");
    let resp = reqwest::get(&url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let pairs = body.get("pairs")?.as_array()?;
    // Prefer the pair with the deepest liquidity to avoid a thin/stale quote.
    let best = pairs.iter().max_by(|a, b| {
        let la = a
            .get("liquidity")
            .and_then(|l| l.get("usd"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let lb = b
            .get("liquidity")
            .and_then(|l| l.get("usd"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
    })?;
    let price_str = best.get("priceUsd")?.as_str()?;
    Decimal::from_str(price_str).ok()
}

/// Resolve the USD price for a known long-tail symbol via its pinned mint.
pub async fn price_for_symbol(symbol_upper: &str) -> Option<Decimal> {
    let mint = mint_for_symbol(symbol_upper)?;
    price_by_mint(mint).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_depin_symbols_map_to_mints() {
        assert_eq!(
            mint_for_symbol("GEOD"),
            Some("7JA5eZdCzztSfQbJvS8aVVxMFfd81Rs9VvwnocV1mKHu")
        );
        assert_eq!(
            mint_for_symbol("WINGS"),
            Some("WingsAYbfs4qnEgcw8jpSvetqp8XHM3GkKvow54WLcd")
        );
        assert_eq!(mint_for_symbol("BTC"), None);
    }
}
