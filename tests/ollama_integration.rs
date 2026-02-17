use std::collections::HashMap;

use cryptofolio::ai::conversation::ConversationState;
use cryptofolio::ai::intent::{Entity, Intent};
use cryptofolio::ai::providers::ollama::OllamaProvider;
use cryptofolio::ai::providers::AiProvider;
use cryptofolio::config::{AiConfig, AppConfig};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a provider with default config (localhost:11434)
fn default_provider() -> OllamaProvider {
    let config = AppConfig::default();
    OllamaProvider::from_config(&config).expect("default provider")
}

/// Create a provider pointing at a custom URL
fn provider_with_url(url: &str) -> OllamaProvider {
    let mut config = AppConfig::default();
    config.ai = Some(AiConfig {
        ollama_url: Some(url.to_string()),
        ..AiConfig::default()
    });
    OllamaProvider::from_config(&config).expect("custom provider")
}

/// Empty conversation state
fn empty_context() -> ConversationState {
    ConversationState::default()
}

// ===========================================================================
// Section A: Unit tests (no Ollama required)
// ===========================================================================

// ---- Provider creation ----------------------------------------------------

#[test]
fn test_provider_creation_default_config() {
    let provider = default_provider();
    assert_eq!(provider.name(), "Ollama");
}

#[test]
fn test_provider_creation_custom_url() {
    let provider = provider_with_url("http://remote-host:11434");
    assert_eq!(provider.name(), "Ollama");
}

#[test]
fn test_provider_creation_no_ai_config() {
    let mut config = AppConfig::default();
    config.ai = None;
    // Even with ai=None, from_config should succeed using defaults
    let provider = OllamaProvider::from_config(&config).expect("should use defaults");
    assert_eq!(provider.name(), "Ollama");
}

// ---- extract_json ---------------------------------------------------------

#[test]
fn test_extract_json_clean() {
    let p = default_provider();
    let input = r#"{"intent":"price.check"}"#;
    assert_eq!(p.extract_json(input), input);
}

#[test]
fn test_extract_json_with_preamble() {
    let p = default_provider();
    let input = r#"Here is the JSON: {"intent":"price.check"}"#;
    assert_eq!(p.extract_json(input), r#"{"intent":"price.check"}"#);
}

#[test]
fn test_extract_json_with_trailing_text() {
    let p = default_provider();
    let input = r#"{"intent":"buy"} hope that helps!"#;
    assert_eq!(p.extract_json(input), r#"{"intent":"buy"}"#);
}

#[test]
fn test_extract_json_no_json() {
    let p = default_provider();
    let input = "no json here";
    assert_eq!(p.extract_json(input), "no json here");
}

#[test]
fn test_extract_json_nested() {
    let p = default_provider();
    let input = r#"{"intent":"buy","entities":{"asset":"BTC"}}"#;
    assert_eq!(p.extract_json(input), input);
}

// ---- map_intent -----------------------------------------------------------

#[test]
fn test_map_intent_price_check() {
    let p = default_provider();
    assert_eq!(p.map_intent("price.check"), Intent::PriceCheck);
}

#[test]
fn test_map_intent_price_check_underscore() {
    let p = default_provider();
    assert_eq!(p.map_intent("price_check"), Intent::PriceCheck);
}

#[test]
fn test_map_intent_tx_buy() {
    let p = default_provider();
    assert_eq!(p.map_intent("tx.buy"), Intent::TxBuy);
}

#[test]
fn test_map_intent_tx_sell() {
    let p = default_provider();
    assert_eq!(p.map_intent("tx.sell"), Intent::TxSell);
}

#[test]
fn test_map_intent_portfolio_view() {
    let p = default_provider();
    assert_eq!(p.map_intent("portfolio.view"), Intent::PortfolioView);
}

#[test]
fn test_map_intent_holdings_list() {
    let p = default_provider();
    assert_eq!(p.map_intent("holdings.list"), Intent::HoldingsList);
}

#[test]
fn test_map_intent_sync() {
    let p = default_provider();
    assert_eq!(p.map_intent("sync"), Intent::Sync);
}

#[test]
fn test_map_intent_help() {
    let p = default_provider();
    assert_eq!(p.map_intent("help"), Intent::Help);
}

#[test]
fn test_map_intent_case_insensitive() {
    let p = default_provider();
    assert_eq!(p.map_intent("PRICE.CHECK"), Intent::PriceCheck);
    assert_eq!(p.map_intent("Help"), Intent::Help);
    assert_eq!(p.map_intent("TX.BUY"), Intent::TxBuy);
}

#[test]
fn test_map_intent_unknown() {
    let p = default_provider();
    assert_eq!(p.map_intent("something_random"), Intent::Unclear);
}

// ---- convert_entities -----------------------------------------------------

#[test]
fn test_convert_entities_string() {
    let p = default_provider();
    let mut map = HashMap::new();
    map.insert("asset".to_string(), serde_json::json!("BTC"));
    let result = p.convert_entities(&Some(map));
    assert!(matches!(result.get("asset"), Some(Entity::String(s)) if s == "BTC"));
}

#[test]
fn test_convert_entities_number() {
    let p = default_provider();
    let mut map = HashMap::new();
    map.insert("quantity".to_string(), serde_json::json!(0.5));
    let result = p.convert_entities(&Some(map));
    assert!(matches!(result.get("quantity"), Some(Entity::Number(n)) if (*n - 0.5).abs() < f64::EPSILON));
}

#[test]
fn test_convert_entities_array() {
    let p = default_provider();
    let mut map = HashMap::new();
    map.insert("symbols".to_string(), serde_json::json!(["BTC", "ETH"]));
    let result = p.convert_entities(&Some(map));
    match result.get("symbols") {
        Some(Entity::Symbols(v)) => {
            assert_eq!(v, &vec!["BTC".to_string(), "ETH".to_string()]);
        }
        other => panic!("expected Symbols, got {:?}", other),
    }
}

#[test]
fn test_convert_entities_boolean() {
    let p = default_provider();
    let mut map = HashMap::new();
    map.insert("show_24h".to_string(), serde_json::json!(true));
    let result = p.convert_entities(&Some(map));
    assert!(matches!(result.get("show_24h"), Some(Entity::Boolean(true))));
}

#[test]
fn test_convert_entities_none() {
    let p = default_provider();
    let result = p.convert_entities(&None);
    assert!(result.is_empty());
}

// ---- extract_symbols ------------------------------------------------------

#[test]
fn test_extract_symbols_by_ticker() {
    let p = default_provider();
    let syms = p.extract_symbols("check BTC and ETH");
    assert!(syms.contains(&"BTC".to_string()));
    assert!(syms.contains(&"ETH".to_string()));
}

#[test]
fn test_extract_symbols_by_name() {
    let p = default_provider();
    let syms = p.extract_symbols("what is bitcoin worth?");
    assert_eq!(syms, vec!["BTC".to_string()]);
}

#[test]
fn test_extract_symbols_no_duplicates() {
    let p = default_provider();
    let syms = p.extract_symbols("btc bitcoin BTC");
    assert_eq!(syms.len(), 1);
    assert_eq!(syms[0], "BTC");
}

#[test]
fn test_extract_symbols_none_found() {
    let p = default_provider();
    let syms = p.extract_symbols("hello world");
    assert!(syms.is_empty());
}

// ---- extract_single_symbol ------------------------------------------------

#[test]
fn test_extract_single_symbol_present() {
    let p = default_provider();
    assert_eq!(
        p.extract_single_symbol("bought some ethereum"),
        Some("ETH".to_string())
    );
}

#[test]
fn test_extract_single_symbol_none() {
    let p = default_provider();
    assert_eq!(p.extract_single_symbol("bought some stuff"), None);
}

// ---- extract_quantity -----------------------------------------------------

#[test]
fn test_extract_quantity_with_symbol() {
    let p = default_provider();
    assert_eq!(p.extract_quantity("bought 0.5 btc"), Some(0.5));
}

#[test]
fn test_extract_quantity_plain_number() {
    let p = default_provider();
    assert_eq!(p.extract_quantity("bought 100 tokens"), Some(100.0));
}

#[test]
fn test_extract_quantity_none() {
    let p = default_provider();
    assert_eq!(p.extract_quantity("bought some bitcoin"), None);
}

// ---- extract_price --------------------------------------------------------

#[test]
fn test_extract_price_at_format() {
    let p = default_provider();
    assert_eq!(p.extract_price("bought btc at 50000"), Some(50000.0));
}

#[test]
fn test_extract_price_dollar_sign() {
    let p = default_provider();
    assert_eq!(p.extract_price("bought btc for $50000"), Some(50000.0));
}

#[test]
fn test_extract_price_k_suffix() {
    let p = default_provider();
    assert_eq!(p.extract_price("bought btc at 50k"), Some(50000.0));
}

#[test]
fn test_extract_price_none() {
    let p = default_provider();
    assert_eq!(p.extract_price("bought some bitcoin"), None);
}

// ---- extract_account ------------------------------------------------------

#[test]
fn test_extract_account_known() {
    let p = default_provider();
    assert_eq!(
        p.extract_account("bought on binance"),
        Some("Binance".to_string())
    );
}

#[test]
fn test_extract_account_on_pattern() {
    let p = default_provider();
    assert_eq!(
        p.extract_account("bought on MyExchange"),
        Some("MyExchange".to_string())
    );
}

#[test]
fn test_extract_account_none() {
    let p = default_provider();
    assert_eq!(p.extract_account("bought some bitcoin"), None);
}

// ---- rule_based_fallback --------------------------------------------------

#[test]
fn test_fallback_price() {
    let p = default_provider();
    let result = p.rule_based_fallback("what is the price of bitcoin").unwrap();
    assert_eq!(result.intent, Intent::PriceCheck);
    assert!(result.confidence > 0.0);
}

#[test]
fn test_fallback_portfolio() {
    let p = default_provider();
    let result = p.rule_based_fallback("show my portfolio").unwrap();
    assert_eq!(result.intent, Intent::PortfolioView);
}

#[test]
fn test_fallback_buy() {
    let p = default_provider();
    let result = p
        .rule_based_fallback("I bought 0.5 btc at 50000 on binance")
        .unwrap();
    assert_eq!(result.intent, Intent::TxBuy);
    assert!(result.missing.is_empty() || result.missing.len() < 4);
}

#[test]
fn test_fallback_buy_missing_fields() {
    let p = default_provider();
    let result = p.rule_based_fallback("I bought something").unwrap();
    assert_eq!(result.intent, Intent::TxBuy);
    assert!(!result.missing.is_empty());
}

#[test]
fn test_fallback_sell() {
    let p = default_provider();
    let result = p.rule_based_fallback("sold 1 eth").unwrap();
    assert_eq!(result.intent, Intent::TxSell);
}

#[test]
fn test_fallback_transfer() {
    let p = default_provider();
    let result = p.rule_based_fallback("transfer btc to ledger").unwrap();
    assert_eq!(result.intent, Intent::HoldingsMove);
}

#[test]
fn test_fallback_sync() {
    let p = default_provider();
    let result = p.rule_based_fallback("sync my exchange").unwrap();
    assert_eq!(result.intent, Intent::Sync);
}

#[test]
fn test_fallback_help() {
    let p = default_provider();
    let result = p.rule_based_fallback("help").unwrap();
    assert_eq!(result.intent, Intent::Help);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_fallback_unclear() {
    let p = default_provider();
    let result = p.rule_based_fallback("asdf jkl").unwrap();
    assert_eq!(result.intent, Intent::Unclear);
    assert_eq!(result.confidence, 0.0);
}

#[test]
fn test_fallback_preserves_raw_input() {
    let p = default_provider();
    let input = "show my portfolio please";
    let result = p.rule_based_fallback(input).unwrap();
    assert_eq!(result.raw_input, input);
}

// ---- parse_response -------------------------------------------------------

#[test]
fn test_parse_response_valid_json() {
    let p = default_provider();
    let json = r#"{"intent":"price.check","entities":{"symbols":["BTC"]},"confidence":0.95}"#;
    let result = p.parse_response(json, "check btc price").unwrap();
    assert_eq!(result.intent, Intent::PriceCheck);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_parse_response_noisy_json() {
    let p = default_provider();
    let json = r#"Sure! {"intent":"tx.buy","entities":{"asset":"ETH"},"confidence":0.8} There you go."#;
    let result = p.parse_response(json, "buy eth").unwrap();
    assert_eq!(result.intent, Intent::TxBuy);
}

#[test]
fn test_parse_response_invalid_falls_back() {
    let p = default_provider();
    let result = p.parse_response("not json at all", "show my portfolio").unwrap();
    // Should fall back to rule-based, which matches "portfolio"
    assert_eq!(result.intent, Intent::PortfolioView);
}

#[test]
fn test_parse_response_partial_json() {
    let p = default_provider();
    let result = p.parse_response("{broken json", "help").unwrap();
    // Falls back to rule-based
    assert_eq!(result.intent, Intent::Help);
}

// ---- build_prompt ---------------------------------------------------------

#[test]
fn test_build_prompt_basic() {
    let p = default_provider();
    let ctx = empty_context();
    let prompt = p.build_prompt("check btc price", &ctx);
    assert!(prompt.contains("check btc price"));
    assert!(prompt.contains("INTENTS:"));
    assert!(prompt.contains("JSON"));
}

#[test]
fn test_build_prompt_with_context() {
    let p = default_provider();
    let ctx = ConversationState {
        last_account: Some("Binance".to_string()),
        last_asset: Some("BTC".to_string()),
        ..ConversationState::default()
    };
    let prompt = p.build_prompt("how much?", &ctx);
    assert!(prompt.contains("Binance"));
    assert!(prompt.contains("BTC"));
    assert!(prompt.contains("CONTEXT:"));
}

// ===========================================================================
// Section B: Integration tests (require running Ollama)
// ===========================================================================

#[tokio::test]
#[ignore]
async fn test_ollama_health_check() {
    let provider = default_provider();
    let healthy = provider.health_check().await;
    assert!(healthy, "Ollama should be running at localhost:11434");
}

#[tokio::test]
#[ignore]
async fn test_ollama_health_check_bad_url() {
    let provider = provider_with_url("http://localhost:1");
    let healthy = provider.health_check().await;
    assert!(!healthy, "Bad URL should fail health check");
}

#[tokio::test]
#[ignore]
async fn test_ollama_model_listing() {
    let _provider = default_provider();
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .expect("request should succeed");
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.expect("valid json");
    assert!(body.get("models").is_some(), "response should have models");
}

#[tokio::test]
#[ignore]
async fn test_ollama_basic_inference() {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    let resp = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "llama3.2:3b",
            "prompt": "Say hello in one word.",
            "stream": false,
            "options": { "num_predict": 10 }
        }))
        .send()
        .await
        .expect("inference should succeed");
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.expect("valid json");
    let response_text = body["response"].as_str().unwrap_or("");
    assert!(!response_text.is_empty(), "response should not be empty");
}

#[tokio::test]
#[ignore]
async fn test_ollama_parse_input_price_check() {
    let provider = default_provider();
    let ctx = empty_context();
    let result = provider
        .parse_input("what is the price of bitcoin?", &ctx)
        .await
        .expect("parse_input should succeed");
    assert_eq!(result.intent, Intent::PriceCheck);
}

#[tokio::test]
#[ignore]
async fn test_ollama_parse_input_buy_transaction() {
    let provider = default_provider();
    let ctx = empty_context();
    let result = provider
        .parse_input("I bought 0.5 BTC at 60000 on Binance", &ctx)
        .await
        .expect("parse_input should succeed");
    assert_eq!(result.intent, Intent::TxBuy);
}

#[tokio::test]
#[ignore]
async fn test_ollama_parse_input_portfolio_view() {
    let provider = default_provider();
    let ctx = empty_context();
    let result = provider
        .parse_input("show my portfolio", &ctx)
        .await
        .expect("parse_input should succeed");
    assert_eq!(result.intent, Intent::PortfolioView);
}

#[tokio::test]
#[ignore]
async fn test_ollama_parse_input_with_context() {
    let provider = default_provider();
    let ctx = ConversationState {
        last_account: Some("Coinbase".to_string()),
        last_asset: Some("ETH".to_string()),
        ..ConversationState::default()
    };
    let result = provider
        .parse_input("how much is it worth?", &ctx)
        .await
        .expect("parse_input should succeed");
    // Should produce some intent (may vary by model)
    assert!(result.confidence > 0.0 || result.intent != Intent::Unclear);
}

#[tokio::test]
#[ignore]
async fn test_ollama_parse_input_help() {
    let provider = default_provider();
    let ctx = empty_context();
    let result = provider
        .parse_input("help", &ctx)
        .await
        .expect("parse_input should succeed");
    assert_eq!(result.intent, Intent::Help);
}

#[tokio::test]
#[ignore]
async fn test_ollama_graceful_fallback_on_garbage() {
    let provider = default_provider();
    let ctx = empty_context();
    // Should not panic or error, just return some result
    let result = provider
        .parse_input("🎲🎲🎲 xyzzy plugh", &ctx)
        .await;
    assert!(result.is_ok(), "garbage input should not cause an error");
}
