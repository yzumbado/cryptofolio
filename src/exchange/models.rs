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
