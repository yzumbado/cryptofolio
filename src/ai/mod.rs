#![allow(dead_code)]

mod conversation;
pub mod intent;
mod providers;
mod tools;

pub use conversation::{ConversationAction, ConversationManager, ConversationState};
pub use intent::{Intent, ParsedInput};
pub use providers::AiProvider;

use crate::config::AppConfig;
use crate::error::Result;

/// AI mode configuration
#[derive(Debug, Clone, PartialEq)]
pub enum AiMode {
    /// Only use Claude API
    Online,
    /// Only use local Ollama
    Offline,
    /// Use local for simple tasks, Claude for complex (default)
    Hybrid,
    /// Disable AI features
    Disabled,
}

impl Default for AiMode {
    fn default() -> Self {
        Self::Hybrid
    }
}

impl std::str::FromStr for AiMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "online" | "claude" => Ok(Self::Online),
            "offline" | "local" | "ollama" => Ok(Self::Offline),
            "hybrid" | "auto" => Ok(Self::Hybrid),
            "disabled" | "off" | "none" => Ok(Self::Disabled),
            _ => Err(format!("Unknown AI mode: {}", s)),
        }
    }
}

/// Task complexity for routing decisions
#[derive(Debug, Clone, PartialEq)]
pub enum Complexity {
    /// Simple corrections, typo fixes
    Low,
    /// Single-turn intent classification
    Medium,
    /// Multi-turn conversations, complex reasoning
    High,
}

/// AI service for natural language understanding
pub struct AiService {
    mode: AiMode,
    claude: Option<providers::ClaudeProvider>,
    ollama: Option<providers::OllamaProvider>,
}

impl AiService {
    /// Create a new AI service from configuration
    pub fn new(config: &AppConfig) -> Result<Self> {
        let mode = config
            .ai
            .as_ref()
            .and_then(|ai| ai.mode.as_ref())
            .and_then(|m| m.parse().ok())
            .unwrap_or_default();

        let claude = if matches!(mode, AiMode::Online | AiMode::Hybrid) {
            providers::ClaudeProvider::from_config(config).ok()
        } else {
            None
        };

        let ollama = if matches!(mode, AiMode::Offline | AiMode::Hybrid) {
            providers::OllamaProvider::from_config(config).ok()
        } else {
            None
        };

        Ok(Self {
            mode,
            claude,
            ollama,
        })
    }

    /// Check if AI features are available
    pub fn is_available(&self) -> bool {
        !matches!(self.mode, AiMode::Disabled)
            && (self.claude.is_some() || self.ollama.is_some())
    }

    /// Get current AI mode
    pub fn mode(&self) -> &AiMode {
        &self.mode
    }

    /// Parse natural language input
    pub async fn parse_input(&self, input: &str, context: &ConversationState) -> Result<ParsedInput> {
        let complexity = self.assess_complexity(input);

        match self.select_provider(&complexity) {
            Some(Provider::Claude) => {
                if let Some(ref claude) = self.claude {
                    return claude.parse_input(input, context).await;
                }
            }
            Some(Provider::Ollama) => {
                if let Some(ref ollama) = self.ollama {
                    return ollama.parse_input(input, context).await;
                }
            }
            None => {}
        }

        // Fallback: try other provider
        if let Some(ref ollama) = self.ollama {
            ollama.parse_input(input, context).await
        } else if let Some(ref claude) = self.claude {
            claude.parse_input(input, context).await
        } else {
            // No AI available, return as unclear
            Ok(ParsedInput {
                intent: Intent::Unclear,
                entities: std::collections::HashMap::new(),
                missing: vec![],
                confidence: 0.0,
                raw_input: input.to_string(),
            })
        }
    }

    /// Check if Ollama is running
    pub async fn check_ollama(&self) -> bool {
        if let Some(ref ollama) = self.ollama {
            ollama.health_check().await
        } else {
            false
        }
    }

    /// Assess complexity of the input
    fn assess_complexity(&self, input: &str) -> Complexity {
        let input_lower = input.to_lowercase();
        let word_count = input.split_whitespace().count();

        // Simple inputs (corrections, single words)
        if word_count <= 3 {
            return Complexity::Low;
        }

        // Complex indicators
        let complex_patterns = [
            "and then",
            "after that",
            "also",
            "but first",
            "if",
            "when",
            "multiple",
            "all my",
            "everything",
        ];

        if complex_patterns.iter().any(|p| input_lower.contains(p)) {
            return Complexity::High;
        }

        // Default to medium for typical single-turn requests
        Complexity::Medium
    }

    /// Select appropriate provider based on complexity and mode
    fn select_provider(&self, complexity: &Complexity) -> Option<Provider> {
        match self.mode {
            AiMode::Disabled => None,
            AiMode::Online => {
                if self.claude.is_some() {
                    Some(Provider::Claude)
                } else {
                    None
                }
            }
            AiMode::Offline => {
                if self.ollama.is_some() {
                    Some(Provider::Ollama)
                } else {
                    None
                }
            }
            AiMode::Hybrid => {
                match complexity {
                    Complexity::Low => {
                        // Prefer local for simple tasks
                        if self.ollama.is_some() {
                            Some(Provider::Ollama)
                        } else {
                            self.claude.as_ref().map(|_| Provider::Claude)
                        }
                    }
                    Complexity::Medium => {
                        // Try local first, fallback available
                        if self.ollama.is_some() {
                            Some(Provider::Ollama)
                        } else {
                            self.claude.as_ref().map(|_| Provider::Claude)
                        }
                    }
                    Complexity::High => {
                        // Prefer Claude for complex tasks
                        if self.claude.is_some() {
                            Some(Provider::Claude)
                        } else {
                            self.ollama.as_ref().map(|_| Provider::Ollama)
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Provider {
    Claude,
    Ollama,
}
