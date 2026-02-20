use cryptofolio::core::currency::{AssetType, Currency, ExchangeRate};
use cryptofolio::db::currencies;
use cryptofolio::error::Result;
use rust_decimal::Decimal;
use std::str::FromStr;
use chrono::Utc;

mod common;

#[tokio::test]
async fn test_currency_list_returns_seeded_currencies() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let currencies = currencies::list_currencies(&pool).await?;

    // Should have 9 seeded currencies from migration
    assert_eq!(currencies.len(), 9);

    // Check for specific currencies
    let currency_codes: Vec<&str> = currencies.iter().map(|c| c.code.as_str()).collect();
    assert!(currency_codes.contains(&"USD"));
    assert!(currency_codes.contains(&"CRC"));
    assert!(currency_codes.contains(&"BTC"));
    assert!(currency_codes.contains(&"USDT"));

    Ok(())
}

#[tokio::test]
async fn test_currency_get_by_code() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let usd = currencies::get_currency(&pool, "USD").await?;
    assert!(usd.is_some());

    let usd = usd.unwrap();
    assert_eq!(usd.code, "USD");
    assert_eq!(usd.name, "US Dollar");
    assert_eq!(usd.symbol, "$");
    assert_eq!(usd.decimals, 2);
    assert_eq!(usd.asset_type, AssetType::Fiat);
    assert!(usd.enabled);

    Ok(())
}

#[tokio::test]
async fn test_currency_get_nonexistent_returns_none() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let result = currencies::get_currency(&pool, "NONEXISTENT").await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_add_custom_currency() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let custom_currency = Currency {
        code: "GBP".to_string(),
        name: "British Pound".to_string(),
        symbol: "£".to_string(),
        decimals: 2,
        asset_type: AssetType::Fiat,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    currencies::add_currency(&pool, &custom_currency).await?;

    let retrieved = currencies::get_currency(&pool, "GBP").await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.code, "GBP");
    assert_eq!(retrieved.name, "British Pound");
    assert_eq!(retrieved.symbol, "£");
    assert_eq!(retrieved.decimals, 2);

    Ok(())
}

#[tokio::test]
async fn test_update_currency() -> Result<()> {
    let pool = common::setup_test_db().await?;

    // Get an existing currency
    let mut usd = currencies::get_currency(&pool, "USD").await?.unwrap();

    // Update it
    usd.enabled = false;
    usd.symbol = "US$".to_string();

    currencies::update_currency(&pool, &usd).await?;

    // Verify update
    let updated = currencies::get_currency(&pool, "USD").await?.unwrap();
    assert!(!updated.enabled);
    assert_eq!(updated.symbol, "US$");

    Ok(())
}

#[tokio::test]
async fn test_remove_currency() -> Result<()> {
    let pool = common::setup_test_db().await?;

    // Add a test currency
    let test_currency = Currency {
        code: "TEST".to_string(),
        name: "Test Currency".to_string(),
        symbol: "T".to_string(),
        decimals: 2,
        asset_type: AssetType::Crypto,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    currencies::add_currency(&pool, &test_currency).await?;

    // Verify it exists
    assert!(currencies::get_currency(&pool, "TEST").await?.is_some());

    // Remove it
    currencies::remove_currency(&pool, "TEST").await?;

    // Verify it's gone
    assert!(currencies::get_currency(&pool, "TEST").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_currency_exists() -> Result<()> {
    let pool = common::setup_test_db().await?;

    assert!(currencies::currency_exists(&pool, "USD").await?);
    assert!(currencies::currency_exists(&pool, "BTC").await?);
    assert!(!currencies::currency_exists(&pool, "NONEXISTENT").await?);

    Ok(())
}

#[tokio::test]
async fn test_add_exchange_rate() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let rate = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00181818").unwrap(),
        timestamp: Utc::now(),
        source: "manual".to_string(),
        notes: Some("Test rate".to_string()),
        created_at: Utc::now(),
    };

    let id = currencies::add_exchange_rate(&pool, &rate).await?;
    assert!(id > 0);

    Ok(())
}

#[tokio::test]
async fn test_get_latest_exchange_rate() -> Result<()> {
    let pool = common::setup_test_db().await?;

    // Add two rates with different timestamps
    let rate1 = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00181818").unwrap(),
        timestamp: Utc::now() - chrono::Duration::hours(2),
        source: "manual".to_string(),
        notes: Some("Old rate".to_string()),
        created_at: Utc::now(),
    };

    let rate2 = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00182000").unwrap(),
        timestamp: Utc::now(),
        source: "manual".to_string(),
        notes: Some("Latest rate".to_string()),
        created_at: Utc::now(),
    };

    currencies::add_exchange_rate(&pool, &rate1).await?;
    currencies::add_exchange_rate(&pool, &rate2).await?;

    let latest = currencies::get_latest_exchange_rate(&pool, "CRC", "USD").await?;
    assert!(latest.is_some());

    let latest = latest.unwrap();
    assert_eq!(latest.rate, Decimal::from_str("0.00182000").unwrap());
    assert_eq!(latest.notes.as_deref(), Some("Latest rate"));

    Ok(())
}

#[tokio::test]
async fn test_get_exchange_rate_at_time() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);
    let two_hours_ago = now - chrono::Duration::hours(2);

    // Add rates at different times
    let rate1 = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00181").unwrap(),
        timestamp: two_hours_ago,
        source: "manual".to_string(),
        notes: None,
        created_at: Utc::now(),
    };

    let rate2 = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00182").unwrap(),
        timestamp: one_hour_ago,
        source: "manual".to_string(),
        notes: None,
        created_at: Utc::now(),
    };

    currencies::add_exchange_rate(&pool, &rate1).await?;
    currencies::add_exchange_rate(&pool, &rate2).await?;

    // Get rate at 90 minutes ago (should return the 2-hour-old rate)
    let ninety_minutes_ago = now - chrono::Duration::minutes(90);
    let historical = currencies::get_exchange_rate_at_time(&pool, "CRC", "USD", ninety_minutes_ago).await?;

    assert!(historical.is_some());
    let historical = historical.unwrap();
    assert_eq!(historical.rate, Decimal::from_str("0.00181").unwrap());

    // Get rate at 30 minutes ago (should return the 1-hour-old rate)
    let thirty_minutes_ago = now - chrono::Duration::minutes(30);
    let recent = currencies::get_exchange_rate_at_time(&pool, "CRC", "USD", thirty_minutes_ago).await?;

    assert!(recent.is_some());
    let recent = recent.unwrap();
    assert_eq!(recent.rate, Decimal::from_str("0.00182").unwrap());

    Ok(())
}

#[tokio::test]
async fn test_list_exchange_rates() -> Result<()> {
    let pool = common::setup_test_db().await?;

    // Add multiple rates for the same pair
    for i in 0..5 {
        let rate = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str(&format!("0.0018{}", i)).unwrap(),
            timestamp: Utc::now() - chrono::Duration::hours(i),
            source: "manual".to_string(),
            notes: None,
            created_at: Utc::now(),
        };
        currencies::add_exchange_rate(&pool, &rate).await?;
    }

    let rates = currencies::list_exchange_rates(&pool, "CRC", "USD").await?;
    assert_eq!(rates.len(), 5);

    // Should be ordered by timestamp DESC
    assert!(rates[0].timestamp > rates[1].timestamp);

    Ok(())
}

#[tokio::test]
async fn test_exchange_rate_upsert_on_conflict() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let timestamp = Utc::now();

    // Add initial rate
    let rate1 = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00181").unwrap(),
        timestamp,
        source: "manual".to_string(),
        notes: Some("Original".to_string()),
        created_at: Utc::now(),
    };

    currencies::add_exchange_rate(&pool, &rate1).await?;

    // Add same pair + timestamp with different rate (should update)
    let rate2 = ExchangeRate {
        id: 0,
        from_currency: "CRC".to_string(),
        to_currency: "USD".to_string(),
        rate: Decimal::from_str("0.00182").unwrap(),
        timestamp,
        source: "manual".to_string(),
        notes: Some("Updated".to_string()),
        created_at: Utc::now(),
    };

    currencies::add_exchange_rate(&pool, &rate2).await?;

    // Should only have one rate for this timestamp
    let rates = currencies::list_exchange_rates(&pool, "CRC", "USD").await?;
    assert_eq!(rates.len(), 1);

    // Should have the updated value
    assert_eq!(rates[0].rate, Decimal::from_str("0.00182").unwrap());
    assert_eq!(rates[0].notes.as_deref(), Some("Updated"));

    Ok(())
}

#[tokio::test]
async fn test_currency_asset_types() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let currencies = currencies::list_currencies(&pool).await?;

    // Check fiat currencies
    let usd = currencies.iter().find(|c| c.code == "USD").unwrap();
    assert_eq!(usd.asset_type, AssetType::Fiat);

    let crc = currencies.iter().find(|c| c.code == "CRC").unwrap();
    assert_eq!(crc.asset_type, AssetType::Fiat);

    // Check stablecoins
    let usdt = currencies.iter().find(|c| c.code == "USDT").unwrap();
    assert_eq!(usdt.asset_type, AssetType::Stablecoin);

    let usdc = currencies.iter().find(|c| c.code == "USDC").unwrap();
    assert_eq!(usdc.asset_type, AssetType::Stablecoin);

    // Check crypto
    let btc = currencies.iter().find(|c| c.code == "BTC").unwrap();
    assert_eq!(btc.asset_type, AssetType::Crypto);

    let eth = currencies.iter().find(|c| c.code == "ETH").unwrap();
    assert_eq!(eth.asset_type, AssetType::Crypto);

    Ok(())
}

#[tokio::test]
async fn test_currency_ordering() -> Result<()> {
    let pool = common::setup_test_db().await?;

    let currencies = currencies::list_currencies(&pool).await?;

    // Should be ordered: fiat, stablecoin, crypto, then by code
    let types: Vec<AssetType> = currencies.iter().map(|c| c.asset_type.clone()).collect();

    // Find the positions of first occurrence of each type
    let first_fiat = types.iter().position(|t| t == &AssetType::Fiat);
    let first_stablecoin = types.iter().position(|t| t == &AssetType::Stablecoin);
    let first_crypto = types.iter().position(|t| t == &AssetType::Crypto);

    // Verify ordering
    assert!(first_fiat.is_some());
    assert!(first_stablecoin.is_some());
    assert!(first_crypto.is_some());

    assert!(first_fiat.unwrap() < first_stablecoin.unwrap());
    assert!(first_stablecoin.unwrap() < first_crypto.unwrap());

    Ok(())
}
