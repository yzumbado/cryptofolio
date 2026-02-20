use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Fiat,
    Crypto,
    Stablecoin,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Fiat => "fiat",
            AssetType::Crypto => "crypto",
            AssetType::Stablecoin => "stablecoin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fiat" => Some(AssetType::Fiat),
            "crypto" | "cryptocurrency" => Some(AssetType::Crypto),
            "stablecoin" | "stable" => Some(AssetType::Stablecoin),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AssetType::Fiat => "Fiat",
            AssetType::Crypto => "Cryptocurrency",
            AssetType::Stablecoin => "Stablecoin",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    pub code: String,          // "USD", "CRC", "BTC"
    pub name: String,          // "US Dollar"
    pub symbol: String,        // "$", "₡", "₿"
    pub decimals: u8,          // 2 for fiat, 8 for BTC, etc.
    pub asset_type: AssetType, // fiat | crypto | stablecoin
    pub enabled: bool,         // Can be disabled without deleting
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Currency {
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        symbol: impl Into<String>,
        decimals: u8,
        asset_type: AssetType,
    ) -> Self {
        Self {
            code: code.into().to_uppercase(),
            name: name.into(),
            symbol: symbol.into(),
            decimals,
            asset_type,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn is_fiat(&self) -> bool {
        matches!(self.asset_type, AssetType::Fiat)
    }

    pub fn is_crypto(&self) -> bool {
        matches!(self.asset_type, AssetType::Crypto | AssetType::Stablecoin)
    }

    pub fn is_stablecoin(&self) -> bool {
        matches!(self.asset_type, AssetType::Stablecoin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub id: i64,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: rust_decimal::Decimal,
    pub timestamp: DateTime<Utc>,
    pub source: String, // "manual" or "api"
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ExchangeRate {
    pub fn new_manual(
        from_currency: impl Into<String>,
        to_currency: impl Into<String>,
        rate: rust_decimal::Decimal,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0,
            from_currency: from_currency.into().to_uppercase(),
            to_currency: to_currency.into().to_uppercase(),
            rate,
            timestamp,
            source: "manual".to_string(),
            notes: None,
            created_at: Utc::now(),
        }
    }

    pub fn new_with_notes(
        from_currency: impl Into<String>,
        to_currency: impl Into<String>,
        rate: rust_decimal::Decimal,
        timestamp: DateTime<Utc>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            id: 0,
            from_currency: from_currency.into().to_uppercase(),
            to_currency: to_currency.into().to_uppercase(),
            rate,
            timestamp,
            source: "manual".to_string(),
            notes: Some(notes.into()),
            created_at: Utc::now(),
        }
    }

    pub fn inverse(&self) -> Self {
        use rust_decimal::Decimal;
        Self {
            id: 0,
            from_currency: self.to_currency.clone(),
            to_currency: self.from_currency.clone(),
            rate: Decimal::ONE / self.rate,
            timestamp: self.timestamp,
            source: "calculated".to_string(),
            notes: Some(format!("Inverse of {}/{}", self.from_currency, self.to_currency)),
            created_at: Utc::now(),
        }
    }

    pub fn pair(&self) -> String {
        format!("{}/{}", self.from_currency, self.to_currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_asset_type_from_str() {
        assert_eq!(AssetType::from_str("fiat"), Some(AssetType::Fiat));
        assert_eq!(AssetType::from_str("crypto"), Some(AssetType::Crypto));
        assert_eq!(AssetType::from_str("stablecoin"), Some(AssetType::Stablecoin));
        assert_eq!(AssetType::from_str("FIAT"), Some(AssetType::Fiat));
        assert_eq!(AssetType::from_str("Crypto"), Some(AssetType::Crypto));
        assert_eq!(AssetType::from_str("invalid"), None);
    }

    #[test]
    fn test_asset_type_as_str() {
        assert_eq!(AssetType::Fiat.as_str(), "fiat");
        assert_eq!(AssetType::Crypto.as_str(), "crypto");
        assert_eq!(AssetType::Stablecoin.as_str(), "stablecoin");
    }

    #[test]
    fn test_asset_type_is_methods() {
        assert!(matches!(AssetType::Fiat, AssetType::Fiat));
        assert!(matches!(AssetType::Crypto, AssetType::Crypto));
        assert!(matches!(AssetType::Stablecoin, AssetType::Stablecoin));
    }

    #[test]
    fn test_currency_is_fiat() {
        let usd = Currency::new("USD", "US Dollar", "$", 2, AssetType::Fiat);
        let btc = Currency::new("BTC", "Bitcoin", "₿", 8, AssetType::Crypto);

        assert!(usd.is_fiat());
        assert!(!btc.is_fiat());
    }

    #[test]
    fn test_currency_is_crypto() {
        let usd = Currency::new("USD", "US Dollar", "$", 2, AssetType::Fiat);
        let btc = Currency::new("BTC", "Bitcoin", "₿", 8, AssetType::Crypto);
        let usdt = Currency::new("USDT", "Tether", "₮", 6, AssetType::Stablecoin);

        assert!(!usd.is_crypto());
        assert!(btc.is_crypto());
        assert!(usdt.is_crypto()); // Stablecoins are considered crypto
    }

    #[test]
    fn test_currency_is_stablecoin() {
        let usdt = Currency::new("USDT", "Tether", "₮", 6, AssetType::Stablecoin);
        let btc = Currency::new("BTC", "Bitcoin", "₿", 8, AssetType::Crypto);

        assert!(usdt.is_stablecoin());
        assert!(!btc.is_stablecoin());
    }

    #[test]
    fn test_exchange_rate_new_manual() {
        let from = "CRC";
        let to = "USD";
        let rate = Decimal::from_str("0.00181818").unwrap();
        let timestamp = Utc::now();

        let exchange_rate = ExchangeRate::new_manual(from, to, rate, timestamp);

        assert_eq!(exchange_rate.from_currency, "CRC");
        assert_eq!(exchange_rate.to_currency, "USD");
        assert_eq!(exchange_rate.rate, rate);
        assert_eq!(exchange_rate.timestamp, timestamp);
        assert_eq!(exchange_rate.source, "manual");
        assert_eq!(exchange_rate.notes, None);
    }

    #[test]
    fn test_exchange_rate_new_with_notes() {
        let from = "CRC";
        let to = "USD";
        let rate = Decimal::from_str("0.00181818").unwrap();
        let timestamp = Utc::now();
        let notes = "Costa Rica bank rate";

        let exchange_rate = ExchangeRate::new_with_notes(from, to, rate, timestamp, notes);

        assert_eq!(exchange_rate.from_currency, "CRC");
        assert_eq!(exchange_rate.to_currency, "USD");
        assert_eq!(exchange_rate.rate, rate);
        assert_eq!(exchange_rate.timestamp, timestamp);
        assert_eq!(exchange_rate.source, "manual");
        assert_eq!(exchange_rate.notes, Some("Costa Rica bank rate".to_string()));
    }

    #[test]
    fn test_exchange_rate_inverse() {
        let rate = ExchangeRate::new_manual(
            "CRC",
            "USD",
            Decimal::from_str("0.00181818").unwrap(),
            Utc::now(),
        );

        let inverse = rate.inverse();

        assert_eq!(inverse.from_currency, "USD");
        assert_eq!(inverse.to_currency, "CRC");

        // USD/CRC should be 1/0.00181818 ≈ 550
        let expected_rate = Decimal::ONE / Decimal::from_str("0.00181818").unwrap();
        assert!((inverse.rate - expected_rate).abs() < Decimal::from_str("0.01").unwrap());

        assert_eq!(inverse.source, "calculated");
        assert_eq!(inverse.notes, Some("Inverse of CRC/USD".to_string()));
    }

    #[test]
    fn test_exchange_rate_pair() {
        let rate = ExchangeRate::new_manual(
            "CRC",
            "USD",
            Decimal::from_str("0.00181818").unwrap(),
            Utc::now(),
        );

        assert_eq!(rate.pair(), "CRC/USD");
    }

    #[test]
    fn test_currency_decimals_for_different_types() {
        let usd = Currency::new("USD", "US Dollar", "$", 2, AssetType::Fiat);
        let btc = Currency::new("BTC", "Bitcoin", "₿", 8, AssetType::Crypto);
        let usdt = Currency::new("USDT", "Tether", "₮", 6, AssetType::Stablecoin);

        assert_eq!(usd.decimals, 2);
        assert_eq!(btc.decimals, 8);
        assert_eq!(usdt.decimals, 6);
    }

    #[test]
    fn test_currency_code_uppercase() {
        let currency = Currency::new("btc", "Bitcoin", "₿", 8, AssetType::Crypto);
        assert_eq!(currency.code, "BTC");
    }
}
