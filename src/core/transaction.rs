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
    pub price_currency: Option<String>, // Currency for price_amount
    pub price_amount: Option<Decimal>,  // Price in price_currency

    // Exchange rate (for fiat conversions)
    pub exchange_rate: Option<Decimal>, // e.g., 550 CRC per 1 USD
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_transaction_type_as_str() {
        assert_eq!(TransactionType::Buy.as_str(), "buy");
        assert_eq!(TransactionType::Sell.as_str(), "sell");
        assert_eq!(TransactionType::TransferIn.as_str(), "transfer_in");
        assert_eq!(TransactionType::TransferOut.as_str(), "transfer_out");
        assert_eq!(
            TransactionType::TransferInternal.as_str(),
            "transfer_internal"
        );
        assert_eq!(TransactionType::Swap.as_str(), "swap");
        assert_eq!(TransactionType::Receive.as_str(), "receive");
        assert_eq!(TransactionType::Fee.as_str(), "fee");
    }

    #[test]
    fn test_transaction_type_from_str() {
        assert_eq!(TransactionType::from_str("buy"), Some(TransactionType::Buy));
        assert_eq!(
            TransactionType::from_str("sell"),
            Some(TransactionType::Sell)
        );
        assert_eq!(
            TransactionType::from_str("transfer_in"),
            Some(TransactionType::TransferIn)
        );
        assert_eq!(
            TransactionType::from_str("transfer_out"),
            Some(TransactionType::TransferOut)
        );
        assert_eq!(
            TransactionType::from_str("transfer_internal"),
            Some(TransactionType::TransferInternal)
        );
        assert_eq!(
            TransactionType::from_str("swap"),
            Some(TransactionType::Swap)
        );
        assert_eq!(
            TransactionType::from_str("receive"),
            Some(TransactionType::Receive)
        );
        assert_eq!(TransactionType::from_str("fee"), Some(TransactionType::Fee));
        assert_eq!(TransactionType::from_str("invalid"), None);
    }

    #[test]
    fn test_transaction_type_from_str_aliases() {
        // Test aliases
        assert_eq!(
            TransactionType::from_str("deposit"),
            Some(TransactionType::TransferIn)
        );
        assert_eq!(
            TransactionType::from_str("withdrawal"),
            Some(TransactionType::TransferOut)
        );
        assert_eq!(
            TransactionType::from_str("transfer"),
            Some(TransactionType::TransferInternal)
        );
        assert_eq!(
            TransactionType::from_str("trade"),
            Some(TransactionType::Swap)
        );
        assert_eq!(
            TransactionType::from_str("airdrop"),
            Some(TransactionType::Receive)
        );
        assert_eq!(
            TransactionType::from_str("reward"),
            Some(TransactionType::Receive)
        );

        // Test case insensitivity
        assert_eq!(TransactionType::from_str("BUY"), Some(TransactionType::Buy));
        assert_eq!(
            TransactionType::from_str("Sell"),
            Some(TransactionType::Sell)
        );
        assert_eq!(
            TransactionType::from_str("DEPOSIT"),
            Some(TransactionType::TransferIn)
        );
    }

    #[test]
    fn test_transaction_type_display_name() {
        assert_eq!(TransactionType::Buy.display_name(), "Buy");
        assert_eq!(TransactionType::Sell.display_name(), "Sell");
        assert_eq!(TransactionType::TransferIn.display_name(), "Transfer In");
        assert_eq!(TransactionType::TransferOut.display_name(), "Transfer Out");
        assert_eq!(
            TransactionType::TransferInternal.display_name(),
            "Internal Transfer"
        );
        assert_eq!(TransactionType::Swap.display_name(), "Swap");
        assert_eq!(TransactionType::Receive.display_name(), "Receive");
        assert_eq!(TransactionType::Fee.display_name(), "Fee");
    }

    #[test]
    fn test_transaction_new_buy() {
        let quantity = Decimal::from_str("1.5").unwrap();
        let price = Decimal::from_str("50000").unwrap();
        let timestamp = Utc::now();

        let tx = Transaction::new_buy("test-account", "BTC", quantity, price, timestamp);

        assert_eq!(tx.tx_type, TransactionType::Buy);
        assert_eq!(tx.to_account_id, Some("test-account".to_string()));
        assert_eq!(tx.to_asset, Some("BTC".to_string()));
        assert_eq!(tx.to_quantity, Some(quantity));
        assert_eq!(tx.price_usd, Some(price));
        assert_eq!(tx.from_account_id, None);
        assert_eq!(tx.from_asset, None);
        assert_eq!(tx.from_quantity, None);
        assert_eq!(tx.timestamp, timestamp);
    }

    #[test]
    fn test_transaction_new_sell() {
        let quantity = Decimal::from_str("0.5").unwrap();
        let price = Decimal::from_str("60000").unwrap();
        let timestamp = Utc::now();

        let tx = Transaction::new_sell("test-account", "BTC", quantity, price, timestamp);

        assert_eq!(tx.tx_type, TransactionType::Sell);
        assert_eq!(tx.from_account_id, Some("test-account".to_string()));
        assert_eq!(tx.from_asset, Some("BTC".to_string()));
        assert_eq!(tx.from_quantity, Some(quantity));
        assert_eq!(tx.price_usd, Some(price));
        assert_eq!(tx.to_account_id, None);
        assert_eq!(tx.to_asset, None);
        assert_eq!(tx.to_quantity, None);
        assert_eq!(tx.timestamp, timestamp);
    }

    #[test]
    fn test_transaction_new_transfer() {
        let quantity = Decimal::from_str("2.0").unwrap();
        let timestamp = Utc::now();

        let tx =
            Transaction::new_transfer("from-account", "to-account", "ETH", quantity, timestamp);

        assert_eq!(tx.tx_type, TransactionType::TransferInternal);
        assert_eq!(tx.from_account_id, Some("from-account".to_string()));
        assert_eq!(tx.from_asset, Some("ETH".to_string()));
        assert_eq!(tx.from_quantity, Some(quantity));
        assert_eq!(tx.to_account_id, Some("to-account".to_string()));
        assert_eq!(tx.to_asset, Some("ETH".to_string()));
        assert_eq!(tx.to_quantity, Some(quantity));
        assert_eq!(tx.price_usd, None);
        assert_eq!(tx.timestamp, timestamp);
    }

    #[test]
    fn test_transaction_new_swap() {
        let from_qty = Decimal::from_str("1.0").unwrap();
        let to_qty = Decimal::from_str("30.0").unwrap();
        let timestamp = Utc::now();

        let tx = Transaction::new_swap("test-account", "BTC", from_qty, "ETH", to_qty, timestamp);

        assert_eq!(tx.tx_type, TransactionType::Swap);
        assert_eq!(tx.from_account_id, Some("test-account".to_string()));
        assert_eq!(tx.from_asset, Some("BTC".to_string()));
        assert_eq!(tx.from_quantity, Some(from_qty));
        assert_eq!(tx.to_account_id, Some("test-account".to_string()));
        assert_eq!(tx.to_asset, Some("ETH".to_string()));
        assert_eq!(tx.to_quantity, Some(to_qty));
        assert_eq!(tx.price_usd, None);
        assert_eq!(tx.timestamp, timestamp);
    }

    #[test]
    fn test_transaction_type_serialization() {
        // Test that TransactionType can be serialized/deserialized correctly
        let buy = TransactionType::Buy;
        let json = serde_json::to_string(&buy).unwrap();
        assert_eq!(json, "\"buy\"");

        let deserialized: TransactionType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TransactionType::Buy);
    }
}
