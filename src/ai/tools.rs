#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Tool definitions for Claude API tool use
/// These are used when Claude needs to call back into the application

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Get all available tools for the crypto portfolio assistant
pub fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "check_price".to_string(),
            description: "Get current price for one or more cryptocurrencies".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbols": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Cryptocurrency symbols (e.g., BTC, ETH)"
                    }
                },
                "required": ["symbols"]
            }),
        },
        Tool {
            name: "record_transaction".to_string(),
            description: "Record a buy, sell, or transfer transaction".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["buy", "sell", "transfer"],
                        "description": "Transaction type"
                    },
                    "asset": {
                        "type": "string",
                        "description": "Cryptocurrency symbol"
                    },
                    "quantity": {
                        "type": "number",
                        "description": "Amount of cryptocurrency"
                    },
                    "price": {
                        "type": "number",
                        "description": "Price per unit in USD"
                    },
                    "account": {
                        "type": "string",
                        "description": "Account name"
                    },
                    "from_account": {
                        "type": "string",
                        "description": "Source account (for transfers)"
                    },
                    "to_account": {
                        "type": "string",
                        "description": "Destination account (for transfers)"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes"
                    }
                },
                "required": ["type", "asset", "quantity"]
            }),
        },
        Tool {
            name: "view_portfolio".to_string(),
            description: "Show current portfolio with holdings and P&L".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "group_by": {
                        "type": "string",
                        "enum": ["account", "category", "none"],
                        "description": "How to group portfolio items"
                    },
                    "account": {
                        "type": "string",
                        "description": "Filter by specific account"
                    },
                    "category": {
                        "type": "string",
                        "description": "Filter by specific category"
                    }
                }
            }),
        },
        Tool {
            name: "list_holdings".to_string(),
            description: "List all holdings across accounts".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "account": {
                        "type": "string",
                        "description": "Filter by specific account"
                    }
                }
            }),
        },
        Tool {
            name: "add_holdings".to_string(),
            description: "Add cryptocurrency holdings to an account".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "asset": {
                        "type": "string",
                        "description": "Cryptocurrency symbol"
                    },
                    "quantity": {
                        "type": "number",
                        "description": "Amount to add"
                    },
                    "account": {
                        "type": "string",
                        "description": "Account name"
                    },
                    "cost_basis": {
                        "type": "number",
                        "description": "Cost per unit in USD"
                    }
                },
                "required": ["asset", "quantity", "account"]
            }),
        },
        Tool {
            name: "move_holdings".to_string(),
            description: "Move holdings from one account to another".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "asset": {
                        "type": "string",
                        "description": "Cryptocurrency symbol"
                    },
                    "quantity": {
                        "type": "number",
                        "description": "Amount to move"
                    },
                    "from_account": {
                        "type": "string",
                        "description": "Source account"
                    },
                    "to_account": {
                        "type": "string",
                        "description": "Destination account"
                    }
                },
                "required": ["asset", "quantity", "from_account", "to_account"]
            }),
        },
        Tool {
            name: "list_accounts".to_string(),
            description: "List all configured accounts".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "sync_exchange".to_string(),
            description: "Sync holdings from an exchange account".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "account": {
                        "type": "string",
                        "description": "Exchange account name to sync"
                    }
                }
            }),
        },
        Tool {
            name: "get_market_data".to_string(),
            description: "Get detailed market data for a cryptocurrency".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Cryptocurrency symbol"
                    },
                    "show_24h": {
                        "type": "boolean",
                        "description": "Include 24-hour statistics"
                    }
                },
                "required": ["symbol"]
            }),
        },
    ]
}

/// Convert tools to Claude API format
pub fn tools_for_claude() -> Vec<serde_json::Value> {
    get_tools()
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema
            })
        })
        .collect()
}
