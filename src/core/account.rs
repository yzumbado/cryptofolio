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
    pub label: Option<String>,
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
