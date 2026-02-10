mod claude;
mod ollama;

pub use claude::ClaudeProvider;
pub use ollama::OllamaProvider;

use async_trait::async_trait;

use super::conversation::ConversationState;
use super::intent::ParsedInput;
use crate::error::Result;

/// Configuration for AI providers
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_tokens: 256,
            temperature: 0.1,
        }
    }
}

/// Trait for AI providers
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Parse natural language input into structured intent
    async fn parse_input(&self, input: &str, context: &ConversationState) -> Result<ParsedInput>;

    /// Check if the provider is available/healthy
    async fn health_check(&self) -> bool;

    /// Get provider name
    fn name(&self) -> &'static str;
}
