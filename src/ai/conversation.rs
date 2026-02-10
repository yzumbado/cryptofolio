#![allow(dead_code)]

use std::collections::HashMap;

use super::intent::{Entity, Intent, ParsedInput};

/// State of an ongoing conversation
#[derive(Debug, Clone, Default)]
pub struct ConversationState {
    /// Current intent being worked on
    pub current_intent: Option<Intent>,
    /// Entities collected so far
    pub collected_entities: HashMap<String, Entity>,
    /// Entities still needed
    pub missing_entities: Vec<String>,
    /// Is a confirmation pending?
    pub confirmation_pending: bool,
    /// Last account mentioned (for context)
    pub last_account: Option<String>,
    /// Last asset mentioned (for context)
    pub last_asset: Option<String>,
    /// Conversation history (last N turns)
    pub history: Vec<ConversationTurn>,
}

/// A single turn in the conversation
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    User,
    Assistant,
}

impl ConversationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update state from shell context
    pub fn from_shell_context(last_account: Option<String>, last_asset: Option<String>) -> Self {
        Self {
            last_account,
            last_asset,
            ..Default::default()
        }
    }

    /// Add a turn to the conversation history
    pub fn add_turn(&mut self, role: Role, content: String) {
        self.history.push(ConversationTurn { role, content });
        // Keep last 10 turns
        if self.history.len() > 10 {
            self.history.remove(0);
        }
    }

    /// Clear current operation state (after completion or cancellation)
    pub fn clear_operation(&mut self) {
        self.current_intent = None;
        self.collected_entities.clear();
        self.missing_entities.clear();
        self.confirmation_pending = false;
    }

    /// Update context from parsed input
    pub fn update_from_parsed(&mut self, parsed: &ParsedInput) {
        // Update last known account
        if let Some(account) = parsed.get_string("account") {
            self.last_account = Some(account.to_string());
        }
        if let Some(account) = parsed.get_string("from_account") {
            self.last_account = Some(account.to_string());
        }

        // Update last known asset
        if let Some(asset) = parsed.get_string("asset") {
            self.last_asset = Some(asset.to_string());
        }
    }

    /// Get context summary for display
    pub fn context_summary(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(ref intent) = self.current_intent {
            parts.push(format!("intent: {:?}", intent));
        }
        if !self.collected_entities.is_empty() {
            let entities: Vec<String> = self
                .collected_entities
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            parts.push(format!("collected: {}", entities.join(", ")));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }
}

/// Actions the conversation manager can take
#[derive(Debug, Clone)]
pub enum ConversationAction {
    /// Ask a clarifying question
    Clarify {
        question: String,
        field: String,
        suggestions: Vec<String>,
    },
    /// Show confirmation for execution
    Confirm {
        summary: String,
        command: String,
        details: Vec<(String, String)>,
    },
    /// Execute the command
    Execute { command: String },
    /// Cancel the current operation
    Cancel { message: String },
    /// Operation was ambiguous, offer choices
    Disambiguate {
        message: String,
        options: Vec<String>,
    },
    /// Direct response (no action needed)
    Respond { message: String },
    /// Input is out of scope
    OutOfScope { message: String },
}

/// Manages multi-turn conversations
pub struct ConversationManager {
    state: ConversationState,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            state: ConversationState::new(),
        }
    }

    pub fn with_context(last_account: Option<String>, last_asset: Option<String>) -> Self {
        Self {
            state: ConversationState::from_shell_context(last_account, last_asset),
        }
    }

    /// Get current state
    pub fn state(&self) -> &ConversationState {
        &self.state
    }

    /// Get mutable state
    pub fn state_mut(&mut self) -> &mut ConversationState {
        &mut self.state
    }

    /// Process parsed input and determine next action
    pub fn process(&mut self, parsed: ParsedInput) -> ConversationAction {
        // Record the turn
        self.state.add_turn(Role::User, parsed.raw_input.clone());

        // Handle special intents
        match parsed.intent {
            Intent::Unclear => {
                return ConversationAction::Clarify {
                    question: "I'm not sure what you'd like to do. Could you rephrase that?".to_string(),
                    field: "intent".to_string(),
                    suggestions: vec![
                        "check prices".to_string(),
                        "view portfolio".to_string(),
                        "record a transaction".to_string(),
                    ],
                };
            }
            Intent::Ambiguous => {
                return ConversationAction::Disambiguate {
                    message: "I could help with a few things here.".to_string(),
                    options: vec![
                        "Check price".to_string(),
                        "View holdings".to_string(),
                    ],
                };
            }
            Intent::OutOfScope => {
                return ConversationAction::OutOfScope {
                    message: "I can only help with cryptocurrency portfolio management.".to_string(),
                };
            }
            Intent::Help => {
                return ConversationAction::Respond {
                    message: "I can help you manage your crypto portfolio. Try things like:\n  \
                        - \"What's the price of Bitcoin?\"\n  \
                        - \"I bought 0.1 BTC on Binance\"\n  \
                        - \"Show my portfolio\"\n  \
                        - \"Sync my exchanges\"".to_string(),
                };
            }
            _ => {}
        }

        // Update state with new intent/entities
        self.state.current_intent = Some(parsed.intent.clone());
        self.state.missing_entities = parsed.missing.clone();

        // Merge entities
        for (key, value) in parsed.entities.iter() {
            self.state.collected_entities.insert(key.clone(), value.clone());
        }

        // Apply context defaults
        self.apply_context_defaults(&parsed);

        // Update context from this input
        self.state.update_from_parsed(&parsed);

        // Recalculate missing entities after context application
        let still_missing = self.calculate_missing(&parsed.intent);

        // Decide next action
        if !still_missing.is_empty() {
            let (question, suggestions) = self.get_clarification_question(&still_missing[0], &parsed.intent);
            return ConversationAction::Clarify {
                question,
                field: still_missing[0].clone(),
                suggestions,
            };
        }

        // All entities collected - build confirmation if needed
        if parsed.intent.requires_confirmation() {
            self.state.confirmation_pending = true;
            let (summary, details) = self.build_confirmation_summary(&parsed.intent);
            let command = self.build_command(&parsed.intent);
            return ConversationAction::Confirm {
                summary,
                command,
                details,
            };
        }

        // Execute immediately (read-only operations)
        let command = self.build_command(&parsed.intent);
        self.state.clear_operation();
        ConversationAction::Execute { command }
    }

    /// Handle user confirmation response
    pub fn handle_confirmation(&mut self, input: &str) -> ConversationAction {
        let input_lower = input.to_lowercase().trim().to_string();

        match input_lower.as_str() {
            "y" | "yes" | "" => {
                if let Some(ref intent) = self.state.current_intent.clone() {
                    let command = self.build_command(intent);
                    self.state.clear_operation();
                    ConversationAction::Execute { command }
                } else {
                    ConversationAction::Cancel {
                        message: "No pending operation.".to_string(),
                    }
                }
            }
            "n" | "no" | "cancel" | "abort" => {
                self.state.clear_operation();
                ConversationAction::Cancel {
                    message: "Operation cancelled.".to_string(),
                }
            }
            _ => {
                // Could be providing more info or corrections
                ConversationAction::Clarify {
                    question: "Please confirm with 'y' or cancel with 'n'.".to_string(),
                    field: "confirmation".to_string(),
                    suggestions: vec!["y".to_string(), "n".to_string()],
                }
            }
        }
    }

    /// Handle partial input when waiting for an entity
    pub fn handle_entity_input(&mut self, input: &str, expected_field: &str) -> Option<Entity> {
        let input = input.trim();

        match expected_field {
            "quantity" | "price" | "cost_basis" | "fee" => {
                // Try to parse as number
                let cleaned = input
                    .replace(',', "")
                    .replace('$', "")
                    .replace("k", "000")
                    .replace("K", "000");
                if let Ok(n) = cleaned.parse::<f64>() {
                    return Some(Entity::Number(n));
                }
            }
            "symbols" => {
                // Parse as list of symbols
                let symbols: Vec<String> = input
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_uppercase())
                    .collect();
                if !symbols.is_empty() {
                    return Some(Entity::Symbols(symbols));
                }
            }
            _ => {
                // String value
                if !input.is_empty() {
                    return Some(Entity::String(input.to_string()));
                }
            }
        }

        None
    }

    /// Apply context defaults (last used account, asset, etc.)
    fn apply_context_defaults(&mut self, parsed: &ParsedInput) {
        // If account is missing and we have a last_account, use it
        if parsed.missing.contains(&"account".to_string()) {
            if let Some(ref account) = self.state.last_account {
                self.state.collected_entities.insert(
                    "account".to_string(),
                    Entity::String(account.clone()),
                );
            }
        }
    }

    /// Calculate which entities are still missing
    fn calculate_missing(&self, intent: &Intent) -> Vec<String> {
        let required = intent.required_entities();
        required
            .into_iter()
            .filter(|e| !self.state.collected_entities.contains_key(*e))
            .map(|s| s.to_string())
            .collect()
    }

    /// Get clarification question for a missing field
    fn get_clarification_question(&self, field: &str, intent: &Intent) -> (String, Vec<String>) {
        let question = match (field, intent) {
            ("quantity", Intent::TxBuy) => "How much did you buy?",
            ("quantity", Intent::TxSell) => "How much did you sell?",
            ("quantity", _) => "What quantity?",
            ("price", Intent::TxBuy) => "What price did you pay per unit?",
            ("price", Intent::TxSell) => "What price did you sell at?",
            ("account", Intent::TxBuy) => "Which account did you buy on?",
            ("account", Intent::TxSell) => "Which account did you sell from?",
            ("account", _) => "Which account?",
            ("from_account", _) => "Which account to transfer from?",
            ("to_account", _) => "Which account to transfer to?",
            ("asset", _) => "Which cryptocurrency?",
            ("symbol", _) => "Which cryptocurrency?",
            ("symbols", _) => "Which cryptocurrency(s)?",
            ("name", Intent::AccountAdd) => "What name for the account?",
            ("account_type", _) => "What type? (exchange, hardware_wallet, software_wallet)",
            ("category", _) => "Which category? (trading, cold-storage, hot-wallets)",
            _ => "Please provide the missing information.",
        };

        let suggestions: Vec<String> = match field {
            "asset" | "symbol" => vec!["BTC".into(), "ETH".into(), "SOL".into()],
            "account_type" => vec![
                "exchange".into(),
                "hardware_wallet".into(),
                "software_wallet".into(),
            ],
            "category" => vec![
                "trading".into(),
                "cold-storage".into(),
                "hot-wallets".into(),
            ],
            _ => vec![],
        };

        (question.to_string(), suggestions)
    }

    /// Build confirmation summary
    fn build_confirmation_summary(&self, intent: &Intent) -> (String, Vec<(String, String)>) {
        let mut details = Vec::new();

        let action = match intent {
            Intent::TxBuy => "BUY",
            Intent::TxSell => "SELL",
            Intent::TxTransfer => "TRANSFER",
            Intent::TxSwap => "SWAP",
            Intent::HoldingsAdd => "ADD HOLDINGS",
            Intent::HoldingsRemove => "REMOVE HOLDINGS",
            Intent::HoldingsMove => "MOVE HOLDINGS",
            Intent::AccountAdd => "ADD ACCOUNT",
            _ => "EXECUTE",
        };

        if let Some(Entity::String(asset)) = self.state.collected_entities.get("asset") {
            details.push(("Asset".to_string(), asset.clone()));
        }
        if let Some(Entity::Number(qty)) = self.state.collected_entities.get("quantity") {
            details.push(("Quantity".to_string(), qty.to_string()));
        }
        if let Some(Entity::Number(price)) = self.state.collected_entities.get("price") {
            details.push(("Price".to_string(), format!("${:.2}", price)));
        }
        if let Some(Entity::String(account)) = self.state.collected_entities.get("account") {
            details.push(("Account".to_string(), account.clone()));
        }
        if let Some(Entity::String(from)) = self.state.collected_entities.get("from_account") {
            details.push(("From".to_string(), from.clone()));
        }
        if let Some(Entity::String(to)) = self.state.collected_entities.get("to_account") {
            details.push(("To".to_string(), to.clone()));
        }

        // Calculate total if buy/sell
        if matches!(intent, Intent::TxBuy | Intent::TxSell) {
            if let (Some(Entity::Number(qty)), Some(Entity::Number(price))) = (
                self.state.collected_entities.get("quantity"),
                self.state.collected_entities.get("price"),
            ) {
                let total = qty * price;
                details.push(("Total".to_string(), format!("${:.2}", total)));
            }
        }

        let summary = format!("Transaction: {}", action);
        (summary, details)
    }

    /// Build CLI command from collected entities
    fn build_command(&self, intent: &Intent) -> String {
        let parsed = ParsedInput {
            intent: intent.clone(),
            entities: self.state.collected_entities.clone(),
            missing: vec![],
            confidence: 1.0,
            raw_input: String::new(),
        };

        parsed.to_cli_command().unwrap_or_default()
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}
