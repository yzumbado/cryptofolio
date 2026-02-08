use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiProvider, ProviderConfig};
use crate::ai::conversation::ConversationState;
use crate::ai::intent::{Entity, Intent, ParsedInput};
use crate::config::AppConfig;
use crate::error::{CryptofolioError, Result};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Claude API provider for natural language understanding
pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    config: ProviderConfig,
}

impl ClaudeProvider {
    /// Create a new Claude provider from app configuration
    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let api_key = config
            .ai
            .as_ref()
            .and_then(|ai| ai.claude_api_key.clone())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                CryptofolioError::Config(
                    "ANTHROPIC_API_KEY not found. Set it in config or environment.".to_string(),
                )
            })?;

        let model = config
            .ai
            .as_ref()
            .and_then(|ai| ai.claude_model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let provider_config = ProviderConfig {
            model,
            max_tokens: 512,
            temperature: 0.1,
        };

        Ok(Self {
            client: Client::new(),
            api_key,
            config: provider_config,
        })
    }

    /// Parse the AI response into structured intent
    fn parse_response(&self, content: &str, raw_input: &str) -> Result<ParsedInput> {
        // Try to parse as JSON
        if let Ok(response) = serde_json::from_str::<AiResponse>(content) {
            let intent = self.map_intent(&response.intent);
            let entities = self.convert_entities(&response.entities);
            let missing = response.missing.unwrap_or_default();
            let confidence = response.confidence.unwrap_or(0.8);

            return Ok(ParsedInput {
                intent,
                entities,
                missing,
                confidence,
                raw_input: raw_input.to_string(),
            });
        }

        // If JSON parsing fails, return unclear intent
        Ok(ParsedInput {
            intent: Intent::Unclear,
            entities: HashMap::new(),
            missing: vec![],
            confidence: 0.0,
            raw_input: raw_input.to_string(),
        })
    }

    /// Map string intent to Intent enum
    fn map_intent(&self, intent_str: &str) -> Intent {
        match intent_str.to_lowercase().as_str() {
            "price.check" | "price_check" | "check_price" => Intent::PriceCheck,
            "market.view" | "market_view" => Intent::MarketView,
            "tx.buy" | "tx_buy" | "buy" | "record_buy" => Intent::TxBuy,
            "tx.sell" | "tx_sell" | "sell" | "record_sell" => Intent::TxSell,
            "tx.transfer" | "tx_transfer" | "transfer" => Intent::TxTransfer,
            "tx.swap" | "tx_swap" | "swap" => Intent::TxSwap,
            "portfolio.view" | "portfolio_view" | "view_portfolio" | "portfolio" => {
                Intent::PortfolioView
            }
            "holdings.list" | "holdings_list" | "list_holdings" => Intent::HoldingsList,
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
    fn convert_entities(
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

    /// Build the system prompt for intent classification
    fn build_system_prompt(&self) -> String {
        r#"You are a crypto portfolio assistant that parses natural language into structured commands.

TASK: Analyze user input and extract:
1. intent: The action the user wants to perform
2. entities: Structured data extracted from the input
3. missing: Required fields that weren't provided

AVAILABLE INTENTS:
- price.check: Get cryptocurrency price (entities: symbols[])
- market.view: Detailed market data (entities: symbol, show_24h)
- tx.buy: Record a buy transaction (entities: asset, quantity, account, price)
- tx.sell: Record a sell transaction (entities: asset, quantity, account, price)
- tx.transfer: Transfer between accounts (entities: asset, quantity, from_account, to_account)
- tx.swap: Swap one crypto for another (entities: from_asset, from_quantity, to_asset, to_quantity, account)
- portfolio.view: View portfolio (entities: account?, category?, by_account?, by_category?)
- holdings.list: List holdings (entities: account?)
- holdings.add: Add holdings (entities: asset, quantity, account, cost_basis?)
- holdings.move: Move holdings between accounts (entities: asset, quantity, from_account, to_account)
- account.list: List accounts
- account.add: Add account (entities: name, account_type, category)
- sync: Sync from exchange (entities: account?)
- help: User needs help
- ambiguous: Input could mean multiple things
- out_of_scope: Request is not about crypto portfolio management

ENTITY NORMALIZATION:
- Crypto symbols should be uppercase: "bitcoin" → "BTC", "ethereum" → "ETH"
- Account names preserve case
- Numbers should be parsed: "0.5", "half" → 0.5, "1k" → 1000

RESPOND IN JSON FORMAT ONLY:
{
  "intent": "tx.buy",
  "entities": {
    "asset": "BTC",
    "quantity": 0.1,
    "account": "Binance",
    "price": 95000
  },
  "missing": [],
  "confidence": 0.95
}

IMPORTANT:
- If critical info is missing, list it in "missing" array
- confidence should be 0.0-1.0
- For ambiguous inputs, use "ambiguous" intent
- For non-crypto questions, use "out_of_scope" intent"#
            .to_string()
    }

    /// Build context from conversation state
    fn build_context(&self, context: &ConversationState) -> String {
        let mut ctx_parts = Vec::new();

        if let Some(ref account) = context.last_account {
            ctx_parts.push(format!("Last used account: {}", account));
        }
        if let Some(ref asset) = context.last_asset {
            ctx_parts.push(format!("Last mentioned asset: {}", asset));
        }

        if !context.collected_entities.is_empty() {
            let entities: Vec<String> = context
                .collected_entities
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            ctx_parts.push(format!("Already collected: {}", entities.join(", ")));
        }

        if ctx_parts.is_empty() {
            String::new()
        } else {
            format!("\n\nCONTEXT:\n{}", ctx_parts.join("\n"))
        }
    }
}

#[async_trait]
impl AiProvider for ClaudeProvider {
    async fn parse_input(&self, input: &str, context: &ConversationState) -> Result<ParsedInput> {
        let system_prompt = self.build_system_prompt() + &self.build_context(context);

        let request = ApiRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            system: Some(system_prompt),
            messages: vec![Message {
                role: "user".to_string(),
                content: input.to_string(),
            }],
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CryptofolioError::Other(format!(
                "Claude API error ({}): {}",
                status, error_text
            )));
        }

        let api_response: ApiResponse = response.json().await?;

        // Extract text content from response
        let content = api_response
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    Some(c.text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        self.parse_response(&content, input)
    }

    async fn health_check(&self) -> bool {
        // Simple check - just verify we have an API key
        !self.api_key.is_empty()
    }

    fn name(&self) -> &'static str {
        "Claude"
    }
}

// API request/response types

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// Parsed AI response
#[derive(Debug, Deserialize)]
struct AiResponse {
    intent: String,
    entities: Option<HashMap<String, serde_json::Value>>,
    missing: Option<Vec<String>>,
    confidence: Option<f64>,
}
