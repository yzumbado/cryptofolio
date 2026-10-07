#![allow(dead_code)]

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    pub price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker24h {
    pub symbol: String,
    pub price: Decimal,
    pub price_change: Decimal,
    pub price_change_percent: Decimal,
    pub high_24h: Decimal,
    pub low_24h: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub price: Decimal,
    pub ticker_24h: Option<Ticker24h>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
}

impl AccountBalance {
    pub fn total(&self) -> Decimal {
        self.free + self.locked
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub quote_quantity: Decimal,
    pub commission: Decimal,
    pub commission_asset: String,
    pub time: i64,
    pub is_buyer: bool,
    pub is_maker: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// A Spot balance held entirely in open orders (locked) must count toward
    /// the account total. Regressing to reading only `free` would report this
    /// position as 0 even though the funds exist.
    #[test]
    fn total_includes_locked_balance() {
        let balance = AccountBalance {
            asset: "USDT".to_string(),
            free: Decimal::ZERO,
            locked: Decimal::from_str("2.28").unwrap(),
        };

        assert_eq!(balance.total(), Decimal::from_str("2.28").unwrap());
    }

    #[test]
    fn total_sums_free_and_locked() {
        let balance = AccountBalance {
            asset: "BTC".to_string(),
            free: Decimal::from_str("0.5").unwrap(),
            locked: Decimal::from_str("0.25").unwrap(),
        };

        assert_eq!(balance.total(), Decimal::from_str("0.75").unwrap());
    }
}
