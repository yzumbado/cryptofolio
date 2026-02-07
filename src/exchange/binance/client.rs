use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

use super::endpoints::*;
use super::models::*;
use crate::error::{CryptofolioError, Result};
use crate::exchange::models::{AccountBalance, MarketData, PriceData, Ticker24h};
use crate::exchange::traits::Exchange;

type HmacSha256 = Hmac<Sha256>;

pub struct BinanceClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    api_secret: Option<String>,
    is_testnet: bool,
}

impl BinanceClient {
    pub fn new(is_testnet: bool, api_key: Option<String>, api_secret: Option<String>) -> Self {
        let base_url = if is_testnet {
            TESTNET_BASE_URL.to_string()
        } else {
            MAINNET_BASE_URL.to_string()
        };

        Self {
            client: Client::new(),
            base_url,
            api_key,
            api_secret,
            is_testnet,
        }
    }

    fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }

    fn sign(&self, query: &str) -> Result<String> {
        let secret = self.api_secret.as_ref()
            .ok_or_else(|| CryptofolioError::AuthRequired("API secret not configured".into()))?;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| CryptofolioError::Other(format!("HMAC error: {}", e)))?;

        mac.update(query.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    async fn get_public<T: serde::de::DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            let error: BinanceError = response.json().await
                .unwrap_or(BinanceError { code: -1, msg: "Unknown error".into() });
            return Err(CryptofolioError::ExchangeApi(format!("[{}] {}", error.code, error.msg)));
        }

        Ok(response.json().await?)
    }

    async fn get_public_with_params<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);

        let response = self.client
            .get(&url)
            .query(params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error: BinanceError = response.json().await
                .unwrap_or(BinanceError { code: -1, msg: "Unknown error".into() });
            return Err(CryptofolioError::ExchangeApi(format!("[{}] {}", error.code, error.msg)));
        }

        Ok(response.json().await?)
    }

    async fn get_signed<T: serde::de::DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| CryptofolioError::AuthRequired("API key not configured".into()))?;

        let timestamp = Self::get_timestamp();
        let query = format!("timestamp={}", timestamp);
        let signature = self.sign(&query)?;

        let url = format!("{}{}?{}&signature={}", self.base_url, endpoint, query, signature);

        let response = self.client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: BinanceError = response.json().await
                .unwrap_or(BinanceError { code: status.as_u16() as i32, msg: "Unknown error".into() });
            return Err(CryptofolioError::ExchangeApi(format!("[{}] {}", error.code, error.msg)));
        }

        Ok(response.json().await?)
    }

    /// Normalize symbol to Binance format (e.g., "BTC" -> "BTCUSDT")
    fn normalize_symbol(&self, symbol: &str) -> String {
        let symbol = symbol.to_uppercase();
        // Check if it's already a trading pair (e.g., BTCUSDT, ETHBTC)
        // Only consider it a pair if it ends with a quote asset AND has more than just the quote asset
        let is_pair = (symbol.ends_with("USDT") && symbol.len() > 4)
            || (symbol.ends_with("BUSD") && symbol.len() > 4)
            || (symbol.ends_with("BTC") && symbol.len() > 3 && symbol != "BTC");

        if is_pair {
            symbol
        } else {
            format!("{}USDT", symbol)
        }
    }

    /// Extract base asset from symbol
    fn extract_base_asset(&self, symbol: &str) -> String {
        let symbol = symbol.to_uppercase();
        if symbol.ends_with("USDT") {
            symbol.trim_end_matches("USDT").to_string()
        } else if symbol.ends_with("BUSD") {
            symbol.trim_end_matches("BUSD").to_string()
        } else if symbol.ends_with("BTC") && symbol != "BTC" {
            symbol.trim_end_matches("BTC").to_string()
        } else {
            symbol
        }
    }
}

#[async_trait]
impl Exchange for BinanceClient {
    fn name(&self) -> &str {
        "Binance"
    }

    fn is_testnet(&self) -> bool {
        self.is_testnet
    }

    fn has_credentials(&self) -> bool {
        self.api_key.is_some() && self.api_secret.is_some()
    }

    async fn get_price(&self, symbol: &str) -> Result<PriceData> {
        let normalized = self.normalize_symbol(symbol);

        let response: BinancePriceResponse = self.get_public_with_params(
            TICKER_PRICE,
            &[("symbol", &normalized)],
        ).await?;

        Ok(PriceData {
            symbol: self.extract_base_asset(&response.symbol),
            price: response.price,
        })
    }

    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<PriceData>> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }

        // Fetch all prices and filter
        let all_prices: Vec<BinancePriceResponse> = self.get_public(TICKER_PRICE).await?;

        let normalized_symbols: Vec<String> = symbols
            .iter()
            .map(|s| self.normalize_symbol(s))
            .collect();

        let filtered: Vec<PriceData> = all_prices
            .into_iter()
            .filter(|p| normalized_symbols.contains(&p.symbol))
            .map(|p| PriceData {
                symbol: self.extract_base_asset(&p.symbol),
                price: p.price,
            })
            .collect();

        Ok(filtered)
    }

    async fn get_ticker_24h(&self, symbol: &str) -> Result<Ticker24h> {
        let normalized = self.normalize_symbol(symbol);

        let response: BinanceTicker24hResponse = self.get_public_with_params(
            TICKER_24H,
            &[("symbol", &normalized)],
        ).await?;

        Ok(Ticker24h {
            symbol: self.extract_base_asset(&response.symbol),
            price: response.last_price,
            price_change: response.price_change,
            price_change_percent: response.price_change_percent,
            high_24h: response.high_price,
            low_24h: response.low_price,
            volume: response.volume,
            quote_volume: response.quote_volume,
        })
    }

    async fn get_market_data(&self, symbol: &str) -> Result<MarketData> {
        let normalized = self.normalize_symbol(symbol);
        let ticker = self.get_ticker_24h(symbol).await?;

        // Determine quote asset
        let quote_asset = if normalized.ends_with("USDT") {
            "USDT"
        } else if normalized.ends_with("BUSD") {
            "BUSD"
        } else if normalized.ends_with("BTC") {
            "BTC"
        } else {
            "USDT"
        };

        Ok(MarketData {
            symbol: normalized.clone(),
            base_asset: self.extract_base_asset(&normalized),
            quote_asset: quote_asset.to_string(),
            price: ticker.price,
            ticker_24h: Some(ticker),
        })
    }

    async fn get_balances(&self) -> Result<Vec<AccountBalance>> {
        let response: BinanceAccountResponse = self.get_signed(ACCOUNT).await?;

        let balances: Vec<AccountBalance> = response.balances
            .into_iter()
            .filter(|b| b.free > rust_decimal::Decimal::ZERO || b.locked > rust_decimal::Decimal::ZERO)
            .map(|b| AccountBalance {
                asset: b.asset,
                free: b.free,
                locked: b.locked,
            })
            .collect();

        Ok(balances)
    }
}
