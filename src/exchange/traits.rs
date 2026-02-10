#![allow(dead_code)]

use async_trait::async_trait;

use super::models::{AccountBalance, MarketData, PriceData, Ticker24h};
use crate::error::Result;

#[async_trait]
pub trait Exchange: Send + Sync {
    /// Get the exchange name
    fn name(&self) -> &str;

    /// Check if this is a testnet connection
    fn is_testnet(&self) -> bool;

    /// Get current price for a symbol
    async fn get_price(&self, symbol: &str) -> Result<PriceData>;

    /// Get prices for multiple symbols
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<PriceData>>;

    /// Get 24h ticker data
    async fn get_ticker_24h(&self, symbol: &str) -> Result<Ticker24h>;

    /// Get full market data for a symbol
    async fn get_market_data(&self, symbol: &str) -> Result<MarketData>;

    /// Get account balances (requires authentication)
    async fn get_balances(&self) -> Result<Vec<AccountBalance>>;

    /// Check if the client has authentication configured
    fn has_credentials(&self) -> bool;
}
