use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiProvider, ProviderConfig};
use crate::ai::conversation::ConversationState;
use crate::ai::intent::{Entity, Intent, ParsedInput};
use crate::config::AppConfig;
use crate::cli::notifications;
use crate::error::Result;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.2:3b";

/// Ollama provider for local LLM inference
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    config: ProviderConfig,
}

impl OllamaProvider {
    /// Create a new Ollama provider from app configuration
    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let base_url = config
            .ai
            .as_ref()
            .and_then(|ai| ai.ollama_url.clone())
            .or_else(|| std::env::var("OLLAMA_HOST").ok())
            .unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string());

        let model = config
            .ai
            .as_ref()
            .and_then(|ai| ai.local_model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let provider_config = ProviderConfig {
            model,
            max_tokens: 256,
            temperature: 0.1,
        };

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url,
            config: provider_config,
        })
    }

    /// Parse the AI response into structured intent
    pub fn parse_response(&self, content: &str, raw_input: &str) -> Result<ParsedInput> {
        // Try to extract JSON from the response
        let json_content = self.extract_json(content);

        if let Ok(response) = serde_json::from_str::<AiResponse>(&json_content) {
            let intent = self.map_intent(&response.intent);
            let entities = self.convert_entities(&response.entities);
            let missing = response.missing.unwrap_or_default();
            let confidence = response.confidence.unwrap_or(0.7);

            return Ok(ParsedInput {
                intent,
                entities,
                missing,
                confidence,
                raw_input: raw_input.to_string(),
            });
        }

        // Try rule-based fallback for common patterns
        self.rule_based_fallback(raw_input)
    }

    /// Extract JSON from potentially noisy LLM output
    pub fn extract_json(&self, content: &str) -> String {
        // Look for JSON object in the response
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                if end > start {
                    return content[start..=end].to_string();
                }
            }
        }
        content.to_string()
    }

    /// Rule-based fallback for when AI parsing fails
    pub fn rule_based_fallback(&self, input: &str) -> Result<ParsedInput> {
        let input_lower = input.to_lowercase();
        let _words: Vec<&str> = input_lower.split_whitespace().collect();

        // Price check patterns
        if input_lower.contains("price") || input_lower.contains("worth") || input_lower.starts_with("how much") {
            let symbols = self.extract_symbols(input);
            return Ok(ParsedInput {
                intent: Intent::PriceCheck,
                entities: if symbols.is_empty() {
                    HashMap::new()
                } else {
                    let mut e = HashMap::new();
                    e.insert("symbols".to_string(), Entity::Symbols(symbols));
                    e
                },
                missing: vec![],
                confidence: 0.6,
                raw_input: input.to_string(),
            });
        }

        // Portfolio view patterns
        if input_lower.contains("portfolio") || input_lower.contains("holdings") || input_lower.contains("what do i have") {
            return Ok(ParsedInput {
                intent: Intent::PortfolioView,
                entities: HashMap::new(),
                missing: vec![],
                confidence: 0.7,
                raw_input: input.to_string(),
            });
        }

        // Buy patterns
        if input_lower.contains("bought") || input_lower.contains("buy") || input_lower.contains("purchased") {
            let mut entities = HashMap::new();
            let mut missing = vec![];

            // Extract asset
            if let Some(asset) = self.extract_single_symbol(input) {
                entities.insert("asset".to_string(), Entity::String(asset));
            } else {
                missing.push("asset".to_string());
            }

            // Extract quantity
            if let Some(qty) = self.extract_quantity(input) {
                entities.insert("quantity".to_string(), Entity::Number(qty));
            } else {
                missing.push("quantity".to_string());
            }

            // Extract price
            if let Some(price) = self.extract_price(input) {
                entities.insert("price".to_string(), Entity::Number(price));
            } else {
                missing.push("price".to_string());
            }

            // Extract account
            if let Some(account) = self.extract_account(input) {
                entities.insert("account".to_string(), Entity::String(account));
            } else {
                missing.push("account".to_string());
            }

            return Ok(ParsedInput {
                intent: Intent::TxBuy,
                entities,
                missing,
                confidence: 0.6,
                raw_input: input.to_string(),
            });
        }

        // Sell patterns
        if input_lower.contains("sold") || input_lower.contains("sell") {
            let mut entities = HashMap::new();
            let mut missing = vec![];

            if let Some(asset) = self.extract_single_symbol(input) {
                entities.insert("asset".to_string(), Entity::String(asset));
            } else {
                missing.push("asset".to_string());
            }

            if let Some(qty) = self.extract_quantity(input) {
                entities.insert("quantity".to_string(), Entity::Number(qty));
            } else {
                missing.push("quantity".to_string());
            }

            if let Some(price) = self.extract_price(input) {
                entities.insert("price".to_string(), Entity::Number(price));
            } else {
                missing.push("price".to_string());
            }

            if let Some(account) = self.extract_account(input) {
                entities.insert("account".to_string(), Entity::String(account));
            } else {
                missing.push("account".to_string());
            }

            return Ok(ParsedInput {
                intent: Intent::TxSell,
                entities,
                missing,
                confidence: 0.6,
                raw_input: input.to_string(),
            });
        }

        // Transfer patterns
        if input_lower.contains("transfer") || input_lower.contains("move") || input_lower.contains("send") {
            return Ok(ParsedInput {
                intent: Intent::HoldingsMove,
                entities: HashMap::new(),
                missing: vec!["asset".to_string(), "quantity".to_string(), "from_account".to_string(), "to_account".to_string()],
                confidence: 0.5,
                raw_input: input.to_string(),
            });
        }

        // Sync patterns
        if input_lower.contains("sync") || input_lower.contains("refresh") || input_lower.contains("update") {
            return Ok(ParsedInput {
                intent: Intent::Sync,
                entities: HashMap::new(),
                missing: vec![],
                confidence: 0.6,
                raw_input: input.to_string(),
            });
        }

        // Help patterns
        if input_lower == "help" || input_lower == "?" || input_lower.contains("what can you") {
            return Ok(ParsedInput {
                intent: Intent::Help,
                entities: HashMap::new(),
                missing: vec![],
                confidence: 0.9,
                raw_input: input.to_string(),
            });
        }

        // Default: unclear
        Ok(ParsedInput {
            intent: Intent::Unclear,
            entities: HashMap::new(),
            missing: vec![],
            confidence: 0.0,
            raw_input: input.to_string(),
        })
    }

    /// Extract cryptocurrency symbols from text
    pub fn extract_symbols(&self, input: &str) -> Vec<String> {
        let known_symbols = [
            ("bitcoin", "BTC"),
            ("btc", "BTC"),
            ("ethereum", "ETH"),
            ("eth", "ETH"),
            ("solana", "SOL"),
            ("sol", "SOL"),
            ("cardano", "ADA"),
            ("ada", "ADA"),
            ("dogecoin", "DOGE"),
            ("doge", "DOGE"),
            ("xrp", "XRP"),
            ("ripple", "XRP"),
            ("polkadot", "DOT"),
            ("dot", "DOT"),
            ("avalanche", "AVAX"),
            ("avax", "AVAX"),
            ("matic", "MATIC"),
            ("polygon", "MATIC"),
            ("litecoin", "LTC"),
            ("ltc", "LTC"),
            ("chainlink", "LINK"),
            ("link", "LINK"),
        ];

        let input_lower = input.to_lowercase();
        let mut symbols = Vec::new();

        for (name, symbol) in known_symbols {
            if input_lower.contains(name) && !symbols.contains(&symbol.to_string()) {
                symbols.push(symbol.to_string());
            }
        }

        symbols
    }

    /// Extract single symbol
    pub fn extract_single_symbol(&self, input: &str) -> Option<String> {
        self.extract_symbols(input).into_iter().next()
    }

    /// Extract quantity from text
    pub fn extract_quantity(&self, input: &str) -> Option<f64> {
        // Look for patterns like "0.1", "0.5 BTC", etc.
        let re_patterns = [
            r"(\d+\.?\d*)\s*(?:btc|eth|sol|ada|doge|xrp|dot|avax|matic|ltc|link)",
            r"(\d+\.?\d*)\s+(?:bitcoin|ethereum|solana)",
            r"bought\s+(\d+\.?\d*)",
            r"sold\s+(\d+\.?\d*)",
        ];

        let input_lower = input.to_lowercase();

        for pattern in re_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(&input_lower) {
                    if let Some(m) = caps.get(1) {
                        if let Ok(qty) = m.as_str().parse::<f64>() {
                            return Some(qty);
                        }
                    }
                }
            }
        }

        // Simple number extraction
        for word in input.split_whitespace() {
            if let Ok(n) = word.replace(',', "").parse::<f64>() {
                if n > 0.0 && n < 1_000_000.0 {
                    return Some(n);
                }
            }
        }

        None
    }

    /// Extract price from text
    pub fn extract_price(&self, input: &str) -> Option<f64> {
        let input_clean = input.replace(',', "").replace('$', "");
        let input_lower = input_clean.to_lowercase();

        // Look for price patterns
        let patterns = [
            r"(?:at|for|@)\s*\$?(\d+\.?\d*)(?:k)?",
            r"\$(\d+\.?\d*)(?:k)?",
            r"(\d+\.?\d*)(?:k)?\s*(?:dollars?|usd|per)",
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(&input_lower) {
                    if let Some(m) = caps.get(1) {
                        if let Ok(price) = m.as_str().parse::<f64>() {
                            // Handle "k" suffix
                            if input_lower.contains("k") && price < 1000.0 {
                                return Some(price * 1000.0);
                            }
                            return Some(price);
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract account name from text
    pub fn extract_account(&self, input: &str) -> Option<String> {
        let known_accounts = [
            "binance",
            "coinbase",
            "kraken",
            "ledger",
            "trezor",
            "metamask",
            "phantom",
        ];

        let input_lower = input.to_lowercase();

        for account in known_accounts {
            if input_lower.contains(account) {
                // Capitalize first letter
                let mut chars = account.chars();
                let capitalized = match chars.next() {
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                    None => account.to_string(),
                };
                return Some(capitalized);
            }
        }

        // Look for "on <account>" pattern
        if let Some(idx) = input_lower.find(" on ") {
            let rest = &input[idx + 4..];
            let account = rest.split_whitespace().next();
            if let Some(a) = account {
                if a.len() > 2 {
                    return Some(a.to_string());
                }
            }
        }

        None
    }

    /// Map string intent to Intent enum
    pub fn map_intent(&self, intent_str: &str) -> Intent {
        match intent_str.to_lowercase().as_str() {
            "price.check" | "price_check" | "check_price" | "price" => Intent::PriceCheck,
            "market.view" | "market_view" | "market" => Intent::MarketView,
            "tx.buy" | "tx_buy" | "buy" | "record_buy" => Intent::TxBuy,
            "tx.sell" | "tx_sell" | "sell" | "record_sell" => Intent::TxSell,
            "tx.transfer" | "tx_transfer" | "transfer" => Intent::TxTransfer,
            "tx.swap" | "tx_swap" | "swap" => Intent::TxSwap,
            "portfolio.view" | "portfolio_view" | "view_portfolio" | "portfolio" => {
                Intent::PortfolioView
            }
            "holdings.list" | "holdings_list" | "list_holdings" | "holdings" => Intent::HoldingsList,
            "holdings.add" | "holdings_add" | "add_holdings" => Intent::HoldingsAdd,
            "holdings.remove" | "holdings_remove" => Intent::HoldingsRemove,
            "holdings.move" | "holdings_move" | "move_holdings" => Intent::HoldingsMove,
            "account.list" | "account_list" | "list_accounts" => Intent::AccountList,
            "account.add" | "account_add" | "add_account" => Intent::AccountAdd,
            "account.show" | "account_show" | "show_account" => Intent::AccountShow,
            "sync" | "sync_exchange" => Intent::Sync,
            "config.show" | "config_show" => Intent::ConfigShow,
            "config.set" | "config_set" => Intent::ConfigSet,
            "help" => Intent::Help,
            "ambiguous" => Intent::Ambiguous,
            "out_of_scope" | "out-of-scope" => Intent::OutOfScope,
            _ => Intent::Unclear,
        }
    }

    /// Convert entity values from JSON to our Entity enum
    pub fn convert_entities(
        &self,
        entities: &Option<HashMap<String, serde_json::Value>>,
    ) -> HashMap<String, Entity> {
        let mut result = HashMap::new();

        if let Some(entities) = entities {
            for (key, value) in entities {
                let entity = match value {
                    serde_json::Value::String(s) => Entity::String(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(f) = n.as_f64() {
                            Entity::Number(f)
                        } else {
                            continue;
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        let strings: Vec<String> = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        Entity::Symbols(strings)
                    }
                    serde_json::Value::Bool(b) => Entity::Boolean(*b),
                    _ => continue,
                };
                result.insert(key.clone(), entity);
            }
        }

        result
    }

    /// Build the prompt for Ollama
    pub fn build_prompt(&self, input: &str, context: &ConversationState) -> String {
        let mut prompt = String::new();

        prompt.push_str("Parse this crypto portfolio command into JSON.\n\n");
        prompt.push_str("INTENTS: price.check, tx.buy, tx.sell, portfolio.view, holdings.list, sync, help, unclear\n\n");

        if context.last_account.is_some() || context.last_asset.is_some() {
            prompt.push_str("CONTEXT:\n");
            if let Some(ref account) = context.last_account {
                prompt.push_str(&format!("- Last account: {}\n", account));
            }
            if let Some(ref asset) = context.last_asset {
                prompt.push_str(&format!("- Last asset: {}\n", asset));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("INPUT: \"{}\"\n\n", input));
        prompt.push_str("RESPOND WITH JSON ONLY:\n");
        prompt.push_str("{\"intent\": \"...\", \"entities\": {...}, \"missing\": [...], \"confidence\": 0.0-1.0}");

        prompt
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    async fn parse_input(&self, input: &str, context: &ConversationState) -> Result<ParsedInput> {
        // First, check if Ollama is running
        if !self.health_check().await {
            // Fall back to rule-based parsing with notification
            notifications::warn_ai_fallback("Ollama not running at localhost:11434");
            return self.rule_based_fallback(input);
        }

        let prompt = self.build_prompt(input, context);

        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: false,
            options: Some(OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens as i32,
            }),
        };

        let url = format!("{}/api/generate", self.base_url);

        match self.client.post(&url).json(&request).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(ollama_response) = response.json::<OllamaResponse>().await {
                        return self.parse_response(&ollama_response.response, input);
                    }
                }
                // Fallback on error
                notifications::warn_ai_fallback("Ollama response parsing failed");
                self.rule_based_fallback(input)
            }
            Err(e) => {
                notifications::warn_ai_fallback(&format!("Ollama request failed: {}", e));
                self.rule_based_fallback(input)
            }
        }
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    fn name(&self) -> &'static str {
        "Ollama"
    }
}

// Ollama API types

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

/// Parsed AI response
#[derive(Debug, Deserialize)]
struct AiResponse {
    intent: String,
    entities: Option<HashMap<String, serde_json::Value>>,
    missing: Option<Vec<String>>,
    confidence: Option<f64>,
}
