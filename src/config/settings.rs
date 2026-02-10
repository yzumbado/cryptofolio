#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{CryptofolioError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub binance: BinanceConfig,

    #[serde(default)]
    pub display: DisplayConfig,

    #[serde(default)]
    pub ai: Option<AiConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            binance: BinanceConfig::default(),
            display: DisplayConfig::default(),
            ai: Some(AiConfig::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// AI mode: "online", "offline", "hybrid", "disabled"
    #[serde(default = "default_ai_mode")]
    pub mode: Option<String>,

    /// Claude API key (can also be set via ANTHROPIC_API_KEY env var)
    #[serde(default)]
    pub claude_api_key: Option<String>,

    /// Claude model to use
    #[serde(default = "default_claude_model")]
    pub claude_model: Option<String>,

    /// Local model for Ollama
    #[serde(default = "default_local_model")]
    pub local_model: Option<String>,

    /// Ollama server URL
    #[serde(default)]
    pub ollama_url: Option<String>,
}

fn default_ai_mode() -> Option<String> {
    Some("hybrid".to_string())
}

fn default_claude_model() -> Option<String> {
    Some("claude-sonnet-4-20250514".to_string())
}

fn default_local_model() -> Option<String> {
    Some("llama3.2:3b".to_string())
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            mode: default_ai_mode(),
            claude_api_key: None,
            claude_model: default_claude_model(),
            local_model: default_local_model(),
            ollama_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default)]
    pub default_account: Option<String>,

    #[serde(default)]
    pub use_testnet: bool,

    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_account: None,
            use_testnet: true, // Default to testnet for safety
            currency: default_currency(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceConfig {
    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default)]
    pub api_secret: Option<String>,
}

impl Default for BinanceConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_secret: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_color")]
    pub color: bool,

    #[serde(default = "default_decimals")]
    pub decimals: u8,
}

fn default_color() -> bool {
    true
}

fn default_decimals() -> u8 {
    8
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            color: default_color(),
            decimals: default_decimals(),
        }
    }
}

impl AppConfig {
    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|p| p.join("cryptofolio"))
            .ok_or_else(|| CryptofolioError::Config("Could not determine config directory".into()))
    }

    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Get the database file path
    pub fn database_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("database.sqlite"))
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        fs::create_dir_all(&config_dir)?;

        let config_path = Self::config_path()?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| CryptofolioError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, content)?;
        Ok(())
    }

    /// Set a config value by key path (e.g., "binance.api_key")
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "general.default_account" => {
                self.general.default_account = Some(value.to_string());
            }
            "general.use_testnet" => {
                self.general.use_testnet = value.parse().map_err(|_| {
                    CryptofolioError::Config("Invalid boolean value".into())
                })?;
            }
            "general.currency" => {
                self.general.currency = value.to_string();
            }
            "binance.api_key" => {
                self.binance.api_key = Some(value.to_string());
            }
            "binance.api_secret" => {
                self.binance.api_secret = Some(value.to_string());
            }
            "display.color" => {
                self.display.color = value.parse().map_err(|_| {
                    CryptofolioError::Config("Invalid boolean value".into())
                })?;
            }
            "display.decimals" => {
                self.display.decimals = value.parse().map_err(|_| {
                    CryptofolioError::Config("Invalid number value".into())
                })?;
            }
            "ai.mode" => {
                self.ensure_ai_config();
                if let Some(ref mut ai) = self.ai {
                    ai.mode = Some(value.to_string());
                }
            }
            "ai.claude_api_key" => {
                self.ensure_ai_config();
                if let Some(ref mut ai) = self.ai {
                    ai.claude_api_key = Some(value.to_string());
                }
            }
            "ai.claude_model" => {
                self.ensure_ai_config();
                if let Some(ref mut ai) = self.ai {
                    ai.claude_model = Some(value.to_string());
                }
            }
            "ai.local_model" => {
                self.ensure_ai_config();
                if let Some(ref mut ai) = self.ai {
                    ai.local_model = Some(value.to_string());
                }
            }
            "ai.ollama_url" => {
                self.ensure_ai_config();
                if let Some(ref mut ai) = self.ai {
                    ai.ollama_url = Some(value.to_string());
                }
            }
            _ => {
                return Err(CryptofolioError::Config(format!("Unknown config key: {}", key)));
            }
        }
        Ok(())
    }

    /// Ensure AI config exists
    fn ensure_ai_config(&mut self) {
        if self.ai.is_none() {
            self.ai = Some(AiConfig::default());
        }
    }

    /// Check if Binance API credentials are configured
    pub fn has_binance_credentials(&self) -> bool {
        self.binance.api_key.is_some() && self.binance.api_secret.is_some()
    }

    /// Get Binance base URL based on testnet setting
    pub fn binance_base_url(&self) -> &'static str {
        if self.general.use_testnet {
            "https://testnet.binance.vision"
        } else {
            "https://api.binance.com"
        }
    }
}
