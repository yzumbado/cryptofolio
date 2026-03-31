#![allow(dead_code)]

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

        // Read response text first so we can log it if parsing fails
        let response_text = response.text().await?;

        match serde_json::from_str(&response_text) {
            Ok(data) => Ok(data),
            Err(e) => {
                // Log the raw response to help debug API changes
                eprintln!("[ERROR] Failed to parse Binance API response from {}", endpoint);
                eprintln!("[ERROR] Parse error: {}", e);
                eprintln!("[ERROR] Raw response (first 500 chars): {}",
                    &response_text.chars().take(500).collect::<String>());

                Err(CryptofolioError::ExchangeApi(format!(
                    "Failed to parse API response from {}: error decoding response body",
                    endpoint
                )))
            }
        }
    }

    /// Signed GET with additional query parameters.
    /// Params are appended before timestamp so the signature covers everything.
    async fn get_signed_with_params<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| CryptofolioError::AuthRequired("API key not configured".into()))?;

        let timestamp = Self::get_timestamp();

        // Build query string: extra params + timestamp
        let mut parts: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        parts.push(format!("timestamp={}", timestamp));
        let query = parts.join("&");

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

        // Read response text first so we can log it if parsing fails
        let response_text = response.text().await?;

        // Try to deserialize, but provide better error context if it fails
        match serde_json::from_str::<T>(&response_text) {
            Ok(data) => Ok(data),
            Err(e) => {
                // Log the raw response to help debug API changes
                eprintln!("[ERROR] Failed to parse Binance API response from {}", endpoint);
                eprintln!("[ERROR] Parse error: {}", e);
                eprintln!("[ERROR] Raw response (first 500 chars): {}",
                    &response_text.chars().take(500).collect::<String>());

                Err(CryptofolioError::ExchangeApi(format!(
                    "Failed to parse API response from {}: error decoding response body. The API response format may have changed.",
                    endpoint
                )))
            }
        }
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

    // -------------------------------------------------------------------------
    // History methods (Task #56)
    // -------------------------------------------------------------------------

    /// Fetch trade history for a single symbol.
    /// API: GET /api/v3/myTrades
    /// Max 1000 results per call. Use `from_id` for pagination.
    pub async fn get_my_trades(
        &self,
        symbol: &str,
        from_id: Option<i64>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: Option<u16>,
    ) -> Result<Vec<BinanceTrade>> {
        let mut params: Vec<(&str, String)> = vec![("symbol", symbol.to_uppercase())];

        if let Some(id) = from_id {
            params.push(("fromId", id.to_string()));
        }
        if let Some(ts) = start_time {
            params.push(("startTime", ts.to_string()));
        }
        if let Some(ts) = end_time {
            params.push(("endTime", ts.to_string()));
        }
        if let Some(l) = limit {
            params.push(("limit", l.min(1000).to_string()));
        }

        self.get_signed_with_params(MY_TRADES, &params).await
    }

    /// Fetch deposit history.
    /// API: GET /sapi/v1/capital/deposit/hisrec
    /// Max 1000 results per call. Use `offset` for pagination.
    pub async fn get_deposit_history(
        &self,
        coin: Option<&str>,
        status: Option<i32>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        offset: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<BinanceDeposit>> {
        let mut params: Vec<(&str, String)> = vec![];

        if let Some(c) = coin {
            params.push(("coin", c.to_uppercase()));
        }
        if let Some(s) = status {
            params.push(("status", s.to_string()));
        }
        if let Some(ts) = start_time {
            params.push(("startTime", ts.to_string()));
        }
        if let Some(ts) = end_time {
            params.push(("endTime", ts.to_string()));
        }
        if let Some(o) = offset {
            params.push(("offset", o.to_string()));
        }
        if let Some(l) = limit {
            params.push(("limit", l.min(1000).to_string()));
        }

        self.get_signed_with_params(DEPOSIT_HISTORY, &params).await
    }

    /// Fetch withdrawal history.
    /// API: GET /sapi/v1/capital/withdraw/history
    /// Max 1000 results per call. Use `offset` for pagination.
    pub async fn get_withdrawal_history(
        &self,
        coin: Option<&str>,
        status: Option<i32>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        offset: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<BinanceWithdrawal>> {
        let mut params: Vec<(&str, String)> = vec![];

        if let Some(c) = coin {
            params.push(("coin", c.to_uppercase()));
        }
        if let Some(s) = status {
            params.push(("status", s.to_string()));
        }
        if let Some(ts) = start_time {
            params.push(("startTime", ts.to_string()));
        }
        if let Some(ts) = end_time {
            params.push(("endTime", ts.to_string()));
        }
        if let Some(o) = offset {
            params.push(("offset", o.to_string()));
        }
        if let Some(l) = limit {
            params.push(("limit", l.min(1000).to_string()));
        }

        self.get_signed_with_params(WITHDRAWAL_HISTORY, &params).await
    }

    /// Fetch fiat purchase/sell history (e.g. credit card → USDT).
    /// API: GET /sapi/v1/fiat/orders
    /// `transaction_type`: "0" = deposit (fiat → crypto), "1" = withdrawal (crypto → fiat).
    /// Max 500 rows per page.
    pub async fn get_fiat_orders(
        &self,
        transaction_type: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
        page: Option<i32>,
        rows: Option<i32>,
    ) -> Result<BinanceFiatOrderResponse> {
        let mut params: Vec<(&str, String)> =
            vec![("transactionType", transaction_type.to_string())];

        if let Some(ts) = start_time {
            params.push(("beginTime", ts.to_string()));
        }
        if let Some(ts) = end_time {
            params.push(("endTime", ts.to_string()));
        }
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }
        if let Some(r) = rows {
            params.push(("rows", r.min(500).to_string()));
        }

        self.get_signed_with_params(FIAT_DEPOSIT_HISTORY, &params).await
    }

    /// Fetch internal transfer history (e.g. Spot ↔ Earn, bots, etc.).
    /// API: GET /sapi/v1/asset/transfer
    /// Common transfer types: "MAIN_UMFUTURE" (Spot→Futures), "UMFUTURE_MAIN" (Futures→Spot),
    /// "MAIN_C2C" (Spot→C2C), "MAIN_MINING" (Spot→Mining).
    /// Max 100 rows per page.
    pub async fn get_transfer_history(
        &self,
        transfer_type: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
        current: Option<i32>,
        size: Option<i32>,
    ) -> Result<BinanceTransferResponse> {
        let mut params: Vec<(&str, String)> = vec![("type", transfer_type.to_string())];

        if let Some(ts) = start_time {
            params.push(("startTime", ts.to_string()));
        }
        if let Some(ts) = end_time {
            params.push(("endTime", ts.to_string()));
        }
        if let Some(c) = current {
            params.push(("current", c.to_string()));
        }
        if let Some(s) = size {
            params.push(("size", s.min(100).to_string()));
        }

        self.get_signed_with_params(UNIVERSAL_TRANSFER_HISTORY, &params).await
    }

    /// Fetch info for all coins (networks, deposit/withdrawal status).
    /// API: GET /sapi/v1/capital/config/getall
    pub async fn get_all_coins_info(&self) -> Result<Vec<BinanceCoinInfo>> {
        self.get_signed_with_params(ALL_COINS_INFO, &[]).await
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
