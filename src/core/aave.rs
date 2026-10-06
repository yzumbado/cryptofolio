//! Aave V3 position health — pure, watch-only.
//!
//! Fetches the protocol's own `getUserAccountData(address)` view (read-only,
//! no keys, no spending) and decodes its six `uint256` return words. Aave
//! computes the health factor with its own risk parameters, so this is the
//! authoritative liquidation-risk number rather than a local recomputation.
//!
//! Scaling (Aave V3, Ethereum mainnet):
//! * collateral / debt / available-borrows are USD with **8** decimals;
//! * liquidation threshold and LTV are stored as `percentage × 100` (so a raw
//!   `8000` means 80.00%);
//! * health factor is **1e18**-scaled (i.e. `2.0` = `2e18`); `< 1.0` = at risk.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::{CryptofolioError, Result};

/// Aave V3 Pool contract on Ethereum mainnet.
pub const AAVE_V3_POOL: &str = "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2";

/// 4-byte selector for `getUserAccountData(address)`.
pub const GET_USER_ACCOUNT_DATA_SELECTOR: &str = "0xbf92857c";

/// A user's Aave V3 account health, decoded from `getUserAccountData`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AaveUserAccountData {
    /// Total supplied collateral, USD.
    pub total_collateral_usd: Decimal,
    /// Total borrowed debt, USD.
    pub total_debt_usd: Decimal,
    /// Remaining borrowing capacity, USD.
    pub available_borrows_usd: Decimal,
    /// Weighted liquidation threshold, percent (e.g. `80.00`).
    pub liquidation_threshold_pct: Decimal,
    /// Weighted max loan-to-value, percent (e.g. `75.00`).
    pub ltv_pct: Decimal,
    /// Health factor (collateral-weighted / debt). `< 1.0` means liquidation risk.
    pub health_factor: Decimal,
}

/// Build the calldata for `getUserAccountData(address)`: selector + the address
/// left-padded to a 32-byte word.
pub fn build_user_account_data_call(address: &str) -> String {
    format!(
        "{}{}{}",
        GET_USER_ACCOUNT_DATA_SELECTOR,
        "000000000000000000000000",
        address.trim_start_matches("0x")
    )
}

/// Decode the raw `getUserAccountData` result (six 32-byte words) into typed,
/// human-scale values. `hex` is the `eth_call` result with or without the `0x`.
pub fn decode_user_account_data(hex: &str) -> Result<AaveUserAccountData> {
    let hex = hex.trim_start_matches("0x");
    const WORDS: usize = 6;
    if hex.len() < WORDS * 64 {
        return Err(CryptofolioError::Other(format!(
            "getUserAccountData returned {} bytes, expected {}",
            hex.len() / 2,
            WORDS * 32
        )));
    }

    let word = |i: usize| -> Result<u128> {
        let s = &hex[i * 64..(i + 1) * 64];
        u128::from_str_radix(s, 16)
            .map_err(|e| CryptofolioError::Other(format!("invalid uint256 word: {}", e)))
    };

    Ok(AaveUserAccountData {
        total_collateral_usd: Decimal::from(word(0)?) / Decimal::from(100_000_000u64),
        total_debt_usd: Decimal::from(word(1)?) / Decimal::from(100_000_000u64),
        available_borrows_usd: Decimal::from(word(2)?) / Decimal::from(100_000_000u64),
        liquidation_threshold_pct: Decimal::from(word(3)?) / Decimal::from(100u64),
        ltv_pct: Decimal::from(word(4)?) / Decimal::from(100u64),
        health_factor: Decimal::from(word(5)?) / Decimal::from(1_000_000_000_000_000_000u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn build_call_pads_address_to_32_bytes() {
        let call = build_user_account_data_call("0x9fB13AB78F63742d2bBCda01E57a6F44A1E12032");
        assert!(call.starts_with("0xbf92857c"));
        // 4-byte selector + 32-byte argument = 36 bytes = 72 hex chars (plus 0x).
        assert_eq!(call.len(), 2 + 72);
        assert!(call.ends_with("9fB13AB78F63742d2bBCda01E57a6F44A1E12032"));
    }

    #[test]
    fn decode_scales_all_six_words() {
        let word = |n: u128| format!("{:064x}", n);
        let hex = format!(
            "{}{}{}{}{}{}",
            word(10_000_000_000_000u128), // $100,000 collateral (8dp)
            word(1_464_656_000_000u128),  // $14,646.56 debt (8dp)
            word(2_000_000_000_000u128),  // $20,000 available (8dp)
            word(8_000u128),              // 80.00% liquidation threshold (1e4)
            word(7_500u128),              // 75.00% LTV (1e4)
            word(2_000_000_000_000_000_000u128), // 2.0 health factor (1e18)
        );

        let d = decode_user_account_data(&hex).unwrap();
        assert_eq!(d.total_collateral_usd, Decimal::from(100_000));
        assert_eq!(d.total_debt_usd, Decimal::from_str("14646.56").unwrap());
        assert_eq!(d.available_borrows_usd, Decimal::from(20_000));
        assert_eq!(d.liquidation_threshold_pct, Decimal::from(80));
        assert_eq!(d.ltv_pct, Decimal::from(75));
        assert_eq!(d.health_factor, Decimal::from(2));
    }

    #[test]
    fn decode_rejects_short_input() {
        assert!(decode_user_account_data("0x1234").is_err());
    }
}
