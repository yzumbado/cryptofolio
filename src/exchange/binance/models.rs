#![allow(dead_code)]

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BinancePriceResponse {
    pub symbol: String,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct BinanceTicker24hResponse {
    pub symbol: String,
    #[serde(rename = "lastPrice", deserialize_with = "deserialize_decimal")]
    pub last_price: Decimal,
    #[serde(rename = "priceChange", deserialize_with = "deserialize_decimal")]
    pub price_change: Decimal,
    #[serde(rename = "priceChangePercent", deserialize_with = "deserialize_decimal")]
    pub price_change_percent: Decimal,
    #[serde(rename = "highPrice", deserialize_with = "deserialize_decimal")]
    pub high_price: Decimal,
    #[serde(rename = "lowPrice", deserialize_with = "deserialize_decimal")]
    pub low_price: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub volume: Decimal,
    #[serde(rename = "quoteVolume", deserialize_with = "deserialize_decimal")]
    pub quote_volume: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct BinanceAccountResponse {
    pub balances: Vec<BinanceBalance>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub free: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub locked: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct BinanceExchangeInfo {
    pub symbols: Vec<BinanceSymbolInfo>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceSymbolInfo {
    pub symbol: String,
    #[serde(rename = "baseAsset")]
    pub base_asset: String,
    #[serde(rename = "quoteAsset")]
    pub quote_asset: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceError {
    pub code: i32,
    pub msg: String,
}

// Custom deserializer for Decimal from string
fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
