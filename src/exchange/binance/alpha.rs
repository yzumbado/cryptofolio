use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;

use crate::error::{CryptofolioError, Result};

const ALPHA_TOKEN_LIST_URL: &str = "https://www.binance.com/bapi/defi/v1/public/wallet-direct/buw/wallet/cex/alpha/all/token/list";

#[derive(Debug, Deserialize)]
struct AlphaResponse {
    code: String,
    data: Vec<AlphaToken>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaToken {
    pub symbol: String,
    pub name: String,
    pub price: String,
    pub percent_change_24h: Option<String>,
    pub volume_24h: Option<String>,
    pub market_cap: Option<String>,
    pub chain_name: String,
}

/// Client for Binance Alpha market API
pub struct BinanceAlphaClient {
    client: Client,
}

impl BinanceAlphaClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch all Alpha tokens with their prices
    pub async fn get_all_tokens(&self) -> Result<Vec<AlphaToken>> {
        let response = self.client
            .get(ALPHA_TOKEN_LIST_URL)
            .header("Accept-Encoding", "identity")
            .send()
            .await?;

        let alpha_response: AlphaResponse = response.json().await?;

        if alpha_response.code != "000000" {
            return Err(CryptofolioError::ExchangeApi(format!(
                "Alpha API error: {}",
                alpha_response.code
            )));
        }

        Ok(alpha_response.data)
    }

    /// Get price for a specific Alpha token by symbol
    pub async fn get_price(&self, symbol: &str) -> Result<Option<Decimal>> {
        let tokens = self.get_all_tokens().await?;
        let symbol_upper = symbol.to_uppercase();

        for token in tokens {
            if token.symbol.to_uppercase() == symbol_upper {
                if let Ok(price) = Decimal::from_str(&token.price) {
                    return Ok(Some(price));
                }
            }
        }

        Ok(None)
    }

    /// Get prices for multiple Alpha tokens
    /// Returns a HashMap of symbol -> price
    pub async fn get_prices(&self, symbols: &[&str]) -> Result<HashMap<String, Decimal>> {
        let tokens = self.get_all_tokens().await?;
        let symbols_upper: Vec<String> = symbols.iter().map(|s| s.to_uppercase()).collect();

        let mut prices = HashMap::new();

        for token in tokens {
            let token_symbol = token.symbol.to_uppercase();
            if symbols_upper.contains(&token_symbol) {
                if let Ok(price) = Decimal::from_str(&token.price) {
                    prices.insert(token_symbol, price);
                }
            }
        }

        Ok(prices)
    }
}

impl Default for BinanceAlphaClient {
    fn default() -> Self {
        Self::new()
    }
}
