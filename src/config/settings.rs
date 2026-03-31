#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{CryptofolioError, Result};

#[cfg(target_os = "macos")]
use super::keychain::get_keychain;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub binance: BinanceConfig,

    #[serde(default)]
    pub etherscan: EtherscanConfig,

    #[serde(default)]
    pub blockfrost: BlockfrostConfig,

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
            etherscan: EtherscanConfig::default(),
            blockfrost: BlockfrostConfig::default(),
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BinanceConfig {
    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default)]
    pub api_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EtherscanConfig {
    /// Etherscan API key for Ethereum mainnet (also used for Sepolia testnet)
    /// Can also be set via ETHERSCAN_API_KEY environment variable
    #[serde(default)]
    pub api_key: Option<String>,
}

impl EtherscanConfig {
    /// Get the API key from config file or env var (does NOT check keychain)
    pub fn resolve_api_key(&self) -> Option<String> {
        std::env::var("ETHERSCAN_API_KEY")
            .ok()
            .or_else(|| self.api_key.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockfrostConfig {
    /// Blockfrost API key for Cardano mainnet
    #[serde(default)]
    pub mainnet_api_key: Option<String>,

    /// Blockfrost API key for Cardano Preprod testnet
    #[serde(default)]
    pub preprod_api_key: Option<String>,

    /// Blockfrost API key for Cardano Preview testnet
    #[serde(default)]
    pub preview_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Enable colored output
    #[serde(default = "default_color")]
    pub color: bool,

    /// Decimal places for quantity display (crypto amounts)
    #[serde(default = "default_decimals")]
    pub decimals: u8,

    /// Decimal places for price display (USD amounts)
    #[serde(default = "default_price_decimals")]
    pub price_decimals: u8,

    /// Use thousands separator in numbers (e.g., 1,234.56)
    #[serde(default = "default_thousands_separator")]
    pub thousands_separator: bool,
}

fn default_color() -> bool {
    true
}

fn default_decimals() -> u8 {
    8
}

fn default_price_decimals() -> u8 {
    2
}

fn default_thousands_separator() -> bool {
    true
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            color: default_color(),
            decimals: default_decimals(),
            price_decimals: default_price_decimals(),
            thousands_separator: default_thousands_separator(),
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
                self.general.use_testnet = value
                    .parse()
                    .map_err(|_| CryptofolioError::Config("Invalid boolean value".into()))?;
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
            "etherscan.api_key" => {
                self.etherscan.api_key = Some(value.to_string());
            }
            "blockfrost.mainnet_api_key" | "blockfrost.api_key" => {
                self.blockfrost.mainnet_api_key = Some(value.to_string());
            }
            "blockfrost.preprod_api_key" => {
                self.blockfrost.preprod_api_key = Some(value.to_string());
            }
            "blockfrost.preview_api_key" => {
                self.blockfrost.preview_api_key = Some(value.to_string());
            }
            "display.color" => {
                self.display.color = value
                    .parse()
                    .map_err(|_| CryptofolioError::Config("Invalid boolean value".into()))?;
            }
            "display.decimals" => {
                self.display.decimals = value
                    .parse()
                    .map_err(|_| CryptofolioError::Config("Invalid number value".into()))?;
            }
            "display.price_decimals" => {
                self.display.price_decimals = value
                    .parse()
                    .map_err(|_| CryptofolioError::Config("Invalid number value".into()))?;
            }
            "display.thousands_separator" => {
                self.display.thousands_separator = value
                    .parse()
                    .map_err(|_| CryptofolioError::Config("Invalid boolean value".into()))?;
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
                return Err(CryptofolioError::Config(format!(
                    "Unknown config key: {}",
                    key
                )));
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
        // Check TOML first
        let has_toml_creds = self.binance.api_key.is_some() && self.binance.api_secret.is_some();

        // If TOML has credentials, return true
        if has_toml_creds {
            return true;
        }

        // Otherwise, check keychain (macOS only)
        #[cfg(target_os = "macos")]
        {
            let keychain = get_keychain();
            keychain.exists("binance.api_key") && keychain.exists("binance.api_secret")
        }

        #[cfg(not(target_os = "macos"))]
        false
    }

    /// Get Etherscan API key: env var → keychain → config file
    pub fn get_etherscan_api_key(&self) -> Option<String> {
        // 1. Env var takes highest priority
        if let Ok(key) = std::env::var("ETHERSCAN_API_KEY") {
            return Some(key);
        }
        // 2. Keychain (macOS)
        #[cfg(target_os = "macos")]
        {
            let keychain = get_keychain();
            if keychain.exists("etherscan.api_key") {
                if let Ok(key) = keychain.retrieve("etherscan.api_key") {
                    return Some(key);
                }
            }
        }
        // 3. Config file
        self.etherscan.api_key.clone()
    }

    /// Get a secret value (checks keychain first, then TOML)
    pub fn get_secret(&self, key: &str) -> Result<Option<String>> {
        // Try keychain first (macOS only)
        #[cfg(target_os = "macos")]
        {
            let keychain = get_keychain();
            if keychain.exists(key) {
                match keychain.retrieve(key) {
                    Ok(value) => return Ok(Some(value)),
                    Err(e) => {
                        eprintln!("Warning: Failed to retrieve '{}' from keychain: {}", key, e);
                        eprintln!("Falling back to TOML config...");
                        // Fall through to TOML
                    }
                }
            }
        }

        // Fall back to TOML
        let value = match key {
            "binance.api_key" => self.binance.api_key.clone(),
            "binance.api_secret" => self.binance.api_secret.clone(),
            "blockfrost.mainnet_api_key" | "blockfrost.api_key" => {
                self.blockfrost.mainnet_api_key.clone()
            }
            "blockfrost.preprod_api_key" => self.blockfrost.preprod_api_key.clone(),
            "blockfrost.preview_api_key" => self.blockfrost.preview_api_key.clone(),
            "ai.claude_api_key" => self.ai.as_ref().and_then(|ai| ai.claude_api_key.clone()),
            _ => None,
        };

        Ok(value)
    }

    /// Get Binance API key (from keychain or TOML)
    pub fn get_binance_api_key(&self) -> Result<Option<String>> {
        self.get_secret("binance.api_key")
    }

    /// Get Binance API secret (from keychain or TOML)
    pub fn get_binance_api_secret(&self) -> Result<Option<String>> {
        self.get_secret("binance.api_secret")
    }

    /// Get Claude API key (from keychain or TOML)
    pub fn get_claude_api_key(&self) -> Result<Option<String>> {
        self.get_secret("ai.claude_api_key")
    }

    /// Get Blockfrost API key based on network (mainnet/preprod/preview)
    /// Falls back to environment variable BLOCKFROST_API_KEY if not in config
    pub fn get_blockfrost_api_key(
        &self,
        is_testnet: bool,
        network: Option<&str>,
    ) -> Option<String> {
        // First, check environment variable
        if let Ok(env_key) = std::env::var("BLOCKFROST_API_KEY") {
            if !env_key.is_empty() {
                return Some(env_key);
            }
        }

        // Then check config based on network
        if is_testnet {
            // For testnet, check network-specific key or default to preprod
            match network {
                Some("preview") => self.blockfrost.preview_api_key.clone(),
                _ => self.blockfrost.preprod_api_key.clone(),
            }
        } else {
            // For mainnet
            self.blockfrost.mainnet_api_key.clone()
        }
    }

    /// Check if Blockfrost API key is configured for the given network
    pub fn has_blockfrost_key(&self, is_testnet: bool, network: Option<&str>) -> bool {
        self.get_blockfrost_api_key(is_testnet, network).is_some()
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
