#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Exchange,
    HardwareWallet,
    SoftwareWallet,
    CustodialService,
    Bank,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::Exchange => "exchange",
            AccountType::HardwareWallet => "hardware_wallet",
            AccountType::SoftwareWallet => "software_wallet",
            AccountType::CustodialService => "custodial_service",
            AccountType::Bank => "bank",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "exchange" => Some(AccountType::Exchange),
            "hardware_wallet" => Some(AccountType::HardwareWallet),
            "software_wallet" => Some(AccountType::SoftwareWallet),
            "custodial_service" => Some(AccountType::CustodialService),
            "bank" => Some(AccountType::Bank),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AccountType::Exchange => "Exchange",
            AccountType::HardwareWallet => "Hardware Wallet",
            AccountType::SoftwareWallet => "Software Wallet",
            AccountType::CustodialService => "Custodial Service",
            AccountType::Bank => "Bank",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    #[serde(default)]
    pub is_testnet: bool,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self { is_testnet: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub category_id: String,
    pub account_type: AccountType,
    pub config: AccountConfig,
    pub sync_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAddress {
    pub id: i64,
    pub account_id: String,
    pub blockchain: String,
    pub address: String,
    pub address_type: Option<String>,
    pub label: Option<String>,
    pub xpub: Option<String>,
    pub derivation_path: Option<String>,
    pub network: Option<String>, // "mainnet" or "testnet"
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl Category {
    pub fn new(id: impl Into<String>, name: impl Into<String>, sort_order: i32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            sort_order,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_type_as_str() {
        assert_eq!(AccountType::Exchange.as_str(), "exchange");
        assert_eq!(AccountType::HardwareWallet.as_str(), "hardware_wallet");
        assert_eq!(AccountType::SoftwareWallet.as_str(), "software_wallet");
        assert_eq!(AccountType::CustodialService.as_str(), "custodial_service");
        assert_eq!(AccountType::Bank.as_str(), "bank");
    }

    #[test]
    fn test_account_type_from_str() {
        assert_eq!(
            AccountType::from_str("exchange"),
            Some(AccountType::Exchange)
        );
        assert_eq!(
            AccountType::from_str("hardware_wallet"),
            Some(AccountType::HardwareWallet)
        );
        assert_eq!(
            AccountType::from_str("software_wallet"),
            Some(AccountType::SoftwareWallet)
        );
        assert_eq!(
            AccountType::from_str("custodial_service"),
            Some(AccountType::CustodialService)
        );
        assert_eq!(AccountType::from_str("bank"), Some(AccountType::Bank));
        assert_eq!(AccountType::from_str("invalid"), None);
    }

    #[test]
    fn test_account_type_display_name() {
        assert_eq!(AccountType::Exchange.display_name(), "Exchange");
        assert_eq!(
            AccountType::HardwareWallet.display_name(),
            "Hardware Wallet"
        );
        assert_eq!(
            AccountType::SoftwareWallet.display_name(),
            "Software Wallet"
        );
        assert_eq!(
            AccountType::CustodialService.display_name(),
            "Custodial Service"
        );
        assert_eq!(AccountType::Bank.display_name(), "Bank");
    }

    #[test]
    fn test_account_config_default() {
        let config = AccountConfig::default();
        assert!(!config.is_testnet);
    }

    #[test]
    fn test_category_new() {
        let category = Category::new("test-id", "Test Category", 10);
        assert_eq!(category.id, "test-id");
        assert_eq!(category.name, "Test Category");
        assert_eq!(category.sort_order, 10);
    }

    #[test]
    fn test_account_type_serialization() {
        // Test that AccountType can be serialized/deserialized correctly
        let exchange = AccountType::Exchange;
        let json = serde_json::to_string(&exchange).unwrap();
        assert_eq!(json, "\"exchange\"");

        let deserialized: AccountType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AccountType::Exchange);
    }

    #[test]
    fn test_account_config_testnet_flag() {
        let config = AccountConfig { is_testnet: true };
        assert!(config.is_testnet);

        let config = AccountConfig { is_testnet: false };
        assert!(!config.is_testnet);
    }
}
