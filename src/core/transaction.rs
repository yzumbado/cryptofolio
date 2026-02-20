use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Buy,
    Sell,
    TransferIn,
    TransferOut,
    TransferInternal,
    Swap,
    Receive,
    Fee,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Buy => "buy",
            TransactionType::Sell => "sell",
            TransactionType::TransferIn => "transfer_in",
            TransactionType::TransferOut => "transfer_out",
            TransactionType::TransferInternal => "transfer_internal",
            TransactionType::Swap => "swap",
            TransactionType::Receive => "receive",
            TransactionType::Fee => "fee",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "buy" => Some(TransactionType::Buy),
            "sell" => Some(TransactionType::Sell),
            "transfer_in" | "deposit" => Some(TransactionType::TransferIn),
            "transfer_out" | "withdrawal" => Some(TransactionType::TransferOut),
            "transfer_internal" | "transfer" => Some(TransactionType::TransferInternal),
            "swap" | "trade" => Some(TransactionType::Swap),
            "receive" | "airdrop" | "reward" => Some(TransactionType::Receive),
            "fee" => Some(TransactionType::Fee),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TransactionType::Buy => "Buy",
            TransactionType::Sell => "Sell",
            TransactionType::TransferIn => "Transfer In",
            TransactionType::TransferOut => "Transfer Out",
            TransactionType::TransferInternal => "Internal Transfer",
            TransactionType::Swap => "Swap",
            TransactionType::Receive => "Receive",
            TransactionType::Fee => "Fee",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub tx_type: TransactionType,

    // Source
    pub from_account_id: Option<String>,
    pub from_asset: Option<String>,
    pub from_quantity: Option<Decimal>,

    // Destination
    pub to_account_id: Option<String>,
    pub to_asset: Option<String>,
    pub to_quantity: Option<Decimal>,

    // Pricing (USD) - kept for backwards compatibility
    pub price_usd: Option<Decimal>,

    // Multi-currency pricing
    pub price_currency: Option<String>,  // Currency for price_amount
    pub price_amount: Option<Decimal>,   // Price in price_currency

    // Exchange rate (for fiat conversions)
    pub exchange_rate: Option<Decimal>,     // e.g., 550 CRC per 1 USD
    pub exchange_rate_pair: Option<String>, // e.g., "CRC/USD"

    pub fee: Option<Decimal>,
    pub fee_asset: Option<String>,

    // Metadata
    pub external_id: Option<String>,
    pub notes: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Transaction {
    pub fn new_buy(
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        price_usd: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            tx_type: TransactionType::Buy,
            from_account_id: None,
            from_asset: None,
            from_quantity: None,
            to_account_id: Some(account_id.to_string()),
            to_asset: Some(asset.to_string()),
            to_quantity: Some(quantity),
            price_usd: Some(price_usd),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp,
            created_at: Utc::now(),
        }
    }

    pub fn new_sell(
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        price_usd: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            tx_type: TransactionType::Sell,
            from_account_id: Some(account_id.to_string()),
            from_asset: Some(asset.to_string()),
            from_quantity: Some(quantity),
            to_account_id: None,
            to_asset: None,
            to_quantity: None,
            price_usd: Some(price_usd),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp,
            created_at: Utc::now(),
        }
    }

    pub fn new_transfer(
        from_account_id: &str,
        to_account_id: &str,
        asset: &str,
        quantity: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            tx_type: TransactionType::TransferInternal,
            from_account_id: Some(from_account_id.to_string()),
            from_asset: Some(asset.to_string()),
            from_quantity: Some(quantity),
            to_account_id: Some(to_account_id.to_string()),
            to_asset: Some(asset.to_string()),
            to_quantity: Some(quantity),
            price_usd: None,
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp,
            created_at: Utc::now(),
        }
    }

    pub fn new_swap(
        account_id: &str,
        from_asset: &str,
        from_quantity: Decimal,
        to_asset: &str,
        to_quantity: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            tx_type: TransactionType::Swap,
            from_account_id: Some(account_id.to_string()),
            from_asset: Some(from_asset.to_string()),
            from_quantity: Some(from_quantity),
            to_account_id: Some(account_id.to_string()),
            to_asset: Some(to_asset.to_string()),
            to_quantity: Some(to_quantity),
            price_usd: None,
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp,
            created_at: Utc::now(),
        }
    }
}
