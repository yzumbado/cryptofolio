use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Recognized intents from natural language
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    // Price operations
    #[serde(rename = "price.check")]
    PriceCheck,

    // Market data
    #[serde(rename = "market.view")]
    MarketView,

    // Transaction operations
    #[serde(rename = "tx.buy")]
    TxBuy,
    #[serde(rename = "tx.sell")]
    TxSell,
    #[serde(rename = "tx.transfer")]
    TxTransfer,
    #[serde(rename = "tx.swap")]
    TxSwap,

    // Portfolio operations
    #[serde(rename = "portfolio.view")]
    PortfolioView,

    // Holdings operations
    #[serde(rename = "holdings.list")]
    HoldingsList,
    #[serde(rename = "holdings.add")]
    HoldingsAdd,
    #[serde(rename = "holdings.remove")]
    HoldingsRemove,
    #[serde(rename = "holdings.move")]
    HoldingsMove,

    // Account operations
    #[serde(rename = "account.list")]
    AccountList,
    #[serde(rename = "account.add")]
    AccountAdd,
    #[serde(rename = "account.show")]
    AccountShow,

    // Sync operations
    #[serde(rename = "sync")]
    Sync,

    // Config operations
    #[serde(rename = "config.show")]
    ConfigShow,
    #[serde(rename = "config.set")]
    ConfigSet,

    // Special intents
    #[serde(rename = "help")]
    Help,
    #[serde(rename = "unclear")]
    Unclear,
    #[serde(rename = "ambiguous")]
    Ambiguous,
    #[serde(rename = "out_of_scope")]
    OutOfScope,
}

impl Intent {
    /// Get the CLI command corresponding to this intent
    pub fn to_command(&self) -> Option<&'static str> {
        match self {
            Intent::PriceCheck => Some("price"),
            Intent::MarketView => Some("market"),
            Intent::TxBuy => Some("tx buy"),
            Intent::TxSell => Some("tx sell"),
            Intent::TxTransfer => Some("tx transfer"),
            Intent::TxSwap => Some("tx swap"),
            Intent::PortfolioView => Some("portfolio"),
            Intent::HoldingsList => Some("holdings list"),
            Intent::HoldingsAdd => Some("holdings add"),
            Intent::HoldingsRemove => Some("holdings remove"),
            Intent::HoldingsMove => Some("holdings move"),
            Intent::AccountList => Some("account list"),
            Intent::AccountAdd => Some("account add"),
            Intent::AccountShow => Some("account show"),
            Intent::Sync => Some("sync"),
            Intent::ConfigShow => Some("config show"),
            Intent::ConfigSet => Some("config set"),
            Intent::Help => Some("help"),
            Intent::Unclear | Intent::Ambiguous | Intent::OutOfScope => None,
        }
    }

    /// Get required entities for this intent
    pub fn required_entities(&self) -> Vec<&'static str> {
        match self {
            Intent::PriceCheck => vec!["symbols"],
            Intent::MarketView => vec!["symbol"],
            Intent::TxBuy => vec!["asset", "quantity", "account", "price"],
            Intent::TxSell => vec!["asset", "quantity", "account", "price"],
            Intent::TxTransfer => vec!["asset", "quantity", "from_account", "to_account"],
            Intent::TxSwap => vec!["from_asset", "from_quantity", "to_asset", "to_quantity", "account"],
            Intent::HoldingsAdd => vec!["asset", "quantity", "account"],
            Intent::HoldingsRemove => vec!["asset", "quantity", "account"],
            Intent::HoldingsMove => vec!["asset", "quantity", "from_account", "to_account"],
            Intent::AccountAdd => vec!["name", "account_type", "category"],
            Intent::AccountShow => vec!["name"],
            Intent::Sync => vec![],
            _ => vec![],
        }
    }

    /// Check if this intent requires confirmation before execution
    pub fn requires_confirmation(&self) -> bool {
        matches!(
            self,
            Intent::TxBuy
                | Intent::TxSell
                | Intent::TxTransfer
                | Intent::TxSwap
                | Intent::HoldingsAdd
                | Intent::HoldingsRemove
                | Intent::HoldingsMove
                | Intent::AccountAdd
        )
    }
}

/// Entity types extracted from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Entity {
    String(String),
    Number(f64),
    Symbols(Vec<String>),
    Boolean(bool),
}

impl Entity {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Entity::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Entity::Number(n) => Some(*n),
            Entity::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_symbols(&self) -> Option<&Vec<String>> {
        match self {
            Entity::Symbols(v) => Some(v),
            _ => None,
        }
    }
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Entity::String(s) => write!(f, "{}", s),
            Entity::Number(n) => write!(f, "{}", n),
            Entity::Symbols(v) => write!(f, "{}", v.join(", ")),
            Entity::Boolean(b) => write!(f, "{}", b),
        }
    }
}

/// Parsed result from natural language input
#[derive(Debug, Clone)]
pub struct ParsedInput {
    /// Recognized intent
    pub intent: Intent,
    /// Extracted entities
    pub entities: HashMap<String, Entity>,
    /// Missing required entities
    pub missing: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Original input text
    pub raw_input: String,
}

impl ParsedInput {
    /// Check if all required entities are present
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }

    /// Get an entity by name
    pub fn get(&self, key: &str) -> Option<&Entity> {
        self.entities.get(key)
    }

    /// Get string entity
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.entities.get(key).and_then(|e| e.as_string())
    }

    /// Get number entity
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.entities.get(key).and_then(|e| e.as_number())
    }

    /// Build CLI command from parsed input
    pub fn to_cli_command(&self) -> Option<String> {
        let base_cmd = self.intent.to_command()?;
        let mut parts = vec![base_cmd.to_string()];

        match self.intent {
            Intent::PriceCheck => {
                if let Some(Entity::Symbols(symbols)) = self.entities.get("symbols") {
                    parts.extend(symbols.iter().cloned());
                } else if let Some(Entity::String(s)) = self.entities.get("symbols") {
                    parts.push(s.clone());
                }
            }
            Intent::MarketView => {
                if let Some(s) = self.get_string("symbol") {
                    parts.push(s.to_string());
                }
                if let Some(Entity::Boolean(true)) = self.entities.get("show_24h") {
                    parts.push("--24h".to_string());
                }
            }
            Intent::TxBuy | Intent::TxSell => {
                if let Some(s) = self.get_string("asset") {
                    parts.push(s.to_string());
                }
                if let Some(n) = self.get_number("quantity") {
                    parts.push(n.to_string());
                }
                if let Some(s) = self.get_string("account") {
                    parts.push("--account".to_string());
                    parts.push(format!("\"{}\"", s));
                }
                if let Some(n) = self.get_number("price") {
                    parts.push("--price".to_string());
                    parts.push(n.to_string());
                }
            }
            Intent::TxTransfer | Intent::HoldingsMove => {
                if let Some(s) = self.get_string("asset") {
                    parts.push(s.to_string());
                }
                if let Some(n) = self.get_number("quantity") {
                    parts.push(n.to_string());
                }
                if let Some(s) = self.get_string("from_account") {
                    parts.push("--from".to_string());
                    parts.push(format!("\"{}\"", s));
                }
                if let Some(s) = self.get_string("to_account") {
                    parts.push("--to".to_string());
                    parts.push(format!("\"{}\"", s));
                }
            }
            Intent::HoldingsAdd => {
                if let Some(s) = self.get_string("asset") {
                    parts.push(s.to_string());
                }
                if let Some(n) = self.get_number("quantity") {
                    parts.push(n.to_string());
                }
                if let Some(s) = self.get_string("account") {
                    parts.push("--account".to_string());
                    parts.push(format!("\"{}\"", s));
                }
                if let Some(n) = self.get_number("cost_basis") {
                    parts.push("--cost".to_string());
                    parts.push(n.to_string());
                }
            }
            Intent::PortfolioView => {
                if let Some(s) = self.get_string("account") {
                    parts.push("--account".to_string());
                    parts.push(format!("\"{}\"", s));
                }
                if let Some(s) = self.get_string("category") {
                    parts.push("--category".to_string());
                    parts.push(format!("\"{}\"", s));
                }
                if let Some(Entity::Boolean(true)) = self.entities.get("by_account") {
                    parts.push("--by-account".to_string());
                }
                if let Some(Entity::Boolean(true)) = self.entities.get("by_category") {
                    parts.push("--by-category".to_string());
                }
            }
            Intent::Sync => {
                if let Some(s) = self.get_string("account") {
                    parts.push("--account".to_string());
                    parts.push(format!("\"{}\"", s));
                }
            }
            Intent::AccountAdd => {
                if let Some(s) = self.get_string("name") {
                    parts.push(format!("\"{}\"", s));
                }
                if let Some(s) = self.get_string("account_type") {
                    parts.push("--type".to_string());
                    parts.push(s.to_string());
                }
                if let Some(s) = self.get_string("category") {
                    parts.push("--category".to_string());
                    parts.push(s.to_string());
                }
            }
            _ => {}
        }

        Some(parts.join(" "))
    }
}

/// Response from AI when intent is ambiguous
#[derive(Debug, Clone)]
pub struct AmbiguousResponse {
    pub possible_intents: Vec<Intent>,
    pub clarification: String,
}
