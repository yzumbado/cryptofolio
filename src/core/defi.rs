//! DeFi position classification.
//!
//! On-chain sync surfaces protocol receipt tokens that are NOT plain assets:
//! Aave V3 issues an **aToken** for supplied collateral (e.g. `aEthUSDT`,
//! `aEthwstETH`) and a **variable/stable debt token** for borrowed funds
//! (e.g. `variableDebtEthGHO`). Treated naively, every holding is a positive
//! quantity priced by its own symbol — but:
//!
//! * aTokens have no price feed of their own; they track the **underlying**
//!   ~1:1 in token terms, so they must be priced by the underlying's USD price.
//! * debt tokens are **liabilities** — their value must be **subtracted** from
//!   net worth, not added.
//!
//! This module is a pure, watch-only classifier: it maps a raw symbol to a
//! [`DefiKind`] and its underlying asset symbol. Valuation code prices the
//! underlying and applies the sign. No network, no keys, no DB.

use serde::{Deserialize, Serialize};

/// How a holding contributes to net worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefiKind {
    /// A plain asset — value counts positively at its own price.
    Plain,
    /// Supplied collateral (Aave aToken) — value counts positively, priced by
    /// the underlying asset.
    Supply,
    /// Borrowed debt (Aave variable/stable debt token) — value counts
    /// **negatively**, priced by the underlying asset.
    Debt,
}

/// A symbol resolved to its economic meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefiAsset {
    pub kind: DefiKind,
    /// Symbol whose USD price should be used to value this holding. For a plain
    /// asset this is the symbol itself; for an aToken/debt token it is the
    /// underlying (e.g. `aEthUSDT` -> `USDT`, `variableDebtEthGHO` -> `GHO`).
    pub underlying: String,
}

impl DefiAsset {
    /// The multiplier applied to the underlying-priced value: +1 for plain and
    /// supply positions, -1 for debt.
    pub fn sign(&self) -> i8 {
        match self.kind {
            DefiKind::Debt => -1,
            _ => 1,
        }
    }

    /// Whether this symbol is a DeFi protocol token (not a plain asset).
    pub fn is_defi(&self) -> bool {
        self.kind != DefiKind::Plain
    }
}

/// Aave V3 debt-token prefixes (case-insensitive), longest first so that a
/// debt token is never mistaken for a supply token.
const DEBT_PREFIXES: &[&str] = &[
    "variabledebteth",
    "stabledebteth",
    "variabledebt",
    "stabledebt",
];

/// Aave V3 supply (aToken) prefixes (case-insensitive), longest first. `aEth`
/// is the Ethereum-market prefix; bare `a` is the legacy/other-market form and
/// is matched last and only when the remainder looks like a real symbol.
const SUPPLY_PREFIXES: &[&str] = &["aeth", "aarb", "aopt", "abas", "apol", "aava"];

/// Classify a raw holding symbol into its DeFi kind and underlying asset.
///
/// Matching is case-insensitive on the prefix; the returned `underlying`
/// preserves the original casing of the remainder so it still matches the
/// price feed (which is itself upper-cased at lookup time).
pub fn classify(symbol: &str) -> DefiAsset {
    let lower = symbol.to_lowercase();

    // Debt first — a debt prefix must win over any supply prefix.
    for p in DEBT_PREFIXES {
        if lower.starts_with(p) && lower.len() > p.len() {
            return DefiAsset {
                kind: DefiKind::Debt,
                underlying: symbol[p.len()..].to_string(),
            };
        }
    }

    for p in SUPPLY_PREFIXES {
        if lower.starts_with(p) && lower.len() > p.len() {
            return DefiAsset {
                kind: DefiKind::Supply,
                underlying: symbol[p.len()..].to_string(),
            };
        }
    }

    DefiAsset {
        kind: DefiKind::Plain,
        underlying: symbol.to_string(),
    }
}

/// USD-pegged stablecoins that price feeds often omit (e.g. Binance has no
/// `USDTUSDT` pair, and GHO — Aave's stablecoin — is not on Binance spot).
/// Used as a $1.00 fallback ONLY when a live feed returns no price.
const STABLECOINS: &[&str] = &[
    "USDT", "USDC", "DAI", "GHO", "USDS", "FDUSD", "TUSD", "USDP",
];

/// If `symbol` is a known USD-pegged stablecoin, return its $1.00 peg. Returns
/// `None` for everything else so callers fall through to the real price feed.
/// This is a pragmatic peg, not an oracle — a depegged stablecoin will read
/// slightly off, which is acceptable for a watch-only tracker.
pub fn stablecoin_peg(symbol: &str) -> Option<rust_decimal::Decimal> {
    let up = symbol.to_uppercase();
    if STABLECOINS.contains(&up.as_str()) {
        Some(rust_decimal::Decimal::ONE)
    } else {
        None
    }
}

/// A liquid-staking token priceable from its own on-chain ETH exchange rate.
#[derive(Debug, Clone, Copy)]
pub struct LstInfo {
    /// The LST token contract on Ethereum mainnet.
    pub contract: &'static str,
    /// 4-byte selector for the contract's ETH-rate getter.
    pub rate_selector: &'static str,
}

/// Resolve a symbol to its LST on-chain rate source, if it is a supported
/// liquid-staking token. Priced as `rate × ETH_price`, with the rate read LIVE
/// from the contract (rETH `getExchangeRate()`, wstETH `stEthPerToken()`).
pub fn lst_info(symbol: &str) -> Option<LstInfo> {
    match symbol.to_uppercase().as_str() {
        "RETH" => Some(LstInfo {
            contract: "0xae78736Cd615f374D3085123A210448E74Fc6393",
            rate_selector: "0xe6aa216c", // getExchangeRate()
        }),
        "WSTETH" => Some(LstInfo {
            contract: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0",
            rate_selector: "0x035faf82", // stEthPerToken()
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aave_supply_tokens_map_to_underlying() {
        let a = classify("aEthUSDT");
        assert_eq!(a.kind, DefiKind::Supply);
        assert_eq!(a.underlying, "USDT");
        assert_eq!(a.sign(), 1);

        assert_eq!(classify("aEthwstETH").underlying, "wstETH");
        assert_eq!(classify("aEthrETH").underlying, "rETH");
    }

    #[test]
    fn aave_debt_tokens_are_negative() {
        let d = classify("variableDebtEthGHO");
        assert_eq!(d.kind, DefiKind::Debt);
        assert_eq!(d.underlying, "GHO");
        assert_eq!(d.sign(), -1);
        assert!(d.is_defi());
    }

    #[test]
    fn debt_prefix_wins_over_supply_prefix() {
        // Ensure "variableDebtEth..." is not mis-read as an "a..."/supply token.
        assert_eq!(classify("stableDebtEthDAI").kind, DefiKind::Debt);
        assert_eq!(classify("stableDebtEthDAI").underlying, "DAI");
    }

    #[test]
    fn plain_assets_are_untouched() {
        for s in ["ETH", "BTC", "GHO", "USDT", "ADA", "SOL", "NIGHT"] {
            let c = classify(s);
            assert_eq!(c.kind, DefiKind::Plain, "{s} should be plain");
            assert_eq!(c.underlying, s);
            assert!(!c.is_defi());
        }
    }

    #[test]
    fn bare_prefix_without_remainder_is_plain() {
        // A symbol that is exactly a prefix (no underlying) must not classify.
        assert_eq!(classify("aeth").kind, DefiKind::Plain);
    }

    #[test]
    fn lst_registry_resolves_reth_and_wsteth() {
        let reth = lst_info("rETH").expect("rETH is an LST");
        assert_eq!(reth.rate_selector, "0xe6aa216c"); // getExchangeRate()
        let wsteth = lst_info("WSTETH").expect("wstETH is an LST (case-insensitive)");
        assert_eq!(wsteth.rate_selector, "0x035faf82"); // stEthPerToken()
        assert!(lst_info("ETH").is_none());
        assert!(lst_info("USDT").is_none());
    }

    #[test]
    fn aave_lst_collateral_maps_to_lst_underlying() {
        // aEthrETH -> Supply/rETH, and rETH is itself LST-priceable.
        let a = classify("aEthrETH");
        assert_eq!(a.kind, DefiKind::Supply);
        assert_eq!(a.underlying, "rETH");
        assert!(lst_info(&a.underlying).is_some());
    }
}
