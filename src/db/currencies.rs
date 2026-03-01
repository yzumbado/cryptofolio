use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::core::currency::{AssetType, Currency, ExchangeRate};
use crate::error::Result;

/// Get all currencies
pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<Currency>> {
    let rows = sqlx::query!(
        r#"
        SELECT code as "code!", name as "name!", symbol as "symbol!",
               decimals as "decimals!", asset_type as "asset_type!",
               enabled as "enabled!",
               created_at as "created_at: String",
               updated_at as "updated_at: String"
        FROM currencies
        ORDER BY
            CASE asset_type
                WHEN 'fiat' THEN 1
                WHEN 'stablecoin' THEN 2
                WHEN 'crypto' THEN 3
            END,
            code
        "#
    )
    .fetch_all(pool)
    .await?;

    let currencies = rows
        .into_iter()
        .map(|row| Currency {
            code: row.code,
            name: row.name,
            symbol: row.symbol,
            decimals: row.decimals as u8,
            asset_type: AssetType::from_str(&row.asset_type).unwrap_or(AssetType::Crypto),
            enabled: row.enabled,
            created_at: row.created_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            updated_at: row.updated_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        })
        .collect();

    Ok(currencies)
}

/// Get a currency by code
pub async fn get_currency(pool: &SqlitePool, code: &str) -> Result<Option<Currency>> {
    let row = sqlx::query!(
        r#"
        SELECT code as "code!", name as "name!", symbol as "symbol!",
               decimals as "decimals!", asset_type as "asset_type!",
               enabled as "enabled!",
               created_at as "created_at: String",
               updated_at as "updated_at: String"
        FROM currencies
        WHERE code = ?
        "#,
        code
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Currency {
        code: row.code,
        name: row.name,
        symbol: row.symbol,
        decimals: row.decimals as u8,
        asset_type: AssetType::from_str(&row.asset_type).unwrap_or(AssetType::Crypto),
        enabled: row.enabled,
        created_at: row.created_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now),
        updated_at: row.updated_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now),
    }))
}

/// Add a new currency
pub async fn add_currency(pool: &SqlitePool, currency: &Currency) -> Result<()> {
    let asset_type = currency.asset_type.as_str();
    let created_at = currency.created_at.to_rfc3339();
    let updated_at = currency.updated_at.to_rfc3339();

    sqlx::query!(
        r#"
        INSERT INTO currencies (code, name, symbol, decimals, asset_type, enabled, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        currency.code,
        currency.name,
        currency.symbol,
        currency.decimals,
        asset_type,
        currency.enabled,
        created_at,
        updated_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a currency
pub async fn update_currency(pool: &SqlitePool, currency: &Currency) -> Result<()> {
    let asset_type = currency.asset_type.as_str();
    let updated_at = Utc::now().to_rfc3339();

    sqlx::query!(
        r#"
        UPDATE currencies
        SET name = ?, symbol = ?, decimals = ?, asset_type = ?, enabled = ?, updated_at = ?
        WHERE code = ?
        "#,
        currency.name,
        currency.symbol,
        currency.decimals,
        asset_type,
        currency.enabled,
        updated_at,
        currency.code
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a currency
pub async fn remove_currency(pool: &SqlitePool, code: &str) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM currencies WHERE code = ?
        "#,
        code
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if a currency exists
pub async fn currency_exists(pool: &SqlitePool, code: &str) -> Result<bool> {
    let row = sqlx::query!(
        r#"
        SELECT COUNT(*) as count FROM currencies WHERE code = ?
        "#,
        code
    )
    .fetch_one(pool)
    .await?;

    Ok(row.count > 0)
}

// ============================================================================
// Exchange Rates
// ============================================================================

/// Add or update an exchange rate
pub async fn add_exchange_rate(pool: &SqlitePool, rate: &ExchangeRate) -> Result<i64> {
    let rate_str = rate.rate.to_string();
    let timestamp = rate.timestamp.to_rfc3339();
    let created_at = rate.created_at.to_rfc3339();

    let result = sqlx::query!(
        r#"
        INSERT INTO exchange_rates (from_currency, to_currency, rate, timestamp, source, notes, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(from_currency, to_currency, timestamp) DO UPDATE SET
            rate = excluded.rate,
            source = excluded.source,
            notes = excluded.notes
        "#,
        rate.from_currency,
        rate.to_currency,
        rate_str,
        timestamp,
        rate.source,
        rate.notes,
        created_at
    )
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get the latest exchange rate for a currency pair
pub async fn get_latest_exchange_rate(
    pool: &SqlitePool,
    from_currency: &str,
    to_currency: &str,
) -> Result<Option<ExchangeRate>> {
    let row = sqlx::query!(
        r#"
        SELECT id as "id!", from_currency as "from_currency!",
               to_currency as "to_currency!", rate as "rate!",
               timestamp as "timestamp: String",
               source, notes,
               created_at as "created_at: String"
        FROM exchange_rates
        WHERE from_currency = ? AND to_currency = ?
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
        from_currency,
        to_currency
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ExchangeRate {
        id: row.id,
        from_currency: row.from_currency,
        to_currency: row.to_currency,
        rate: row.rate.parse().unwrap_or_else(|_| Decimal::ZERO),
        timestamp: DateTime::parse_from_rfc3339(&row.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        source: row.source.unwrap_or_else(|| "manual".to_string()),
        notes: row.notes,
        created_at: row.created_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now),
    }))
}

/// Get exchange rate at a specific time (or closest before)
pub async fn get_exchange_rate_at_time(
    pool: &SqlitePool,
    from_currency: &str,
    to_currency: &str,
    timestamp: DateTime<Utc>,
) -> Result<Option<ExchangeRate>> {
    let timestamp_str = timestamp.to_rfc3339();

    let row = sqlx::query!(
        r#"
        SELECT id as "id!", from_currency as "from_currency!",
               to_currency as "to_currency!", rate as "rate!",
               timestamp as "timestamp: String",
               source, notes,
               created_at as "created_at: String"
        FROM exchange_rates
        WHERE from_currency = ? AND to_currency = ? AND timestamp <= ?
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
        from_currency,
        to_currency,
        timestamp_str
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ExchangeRate {
        id: row.id,
        from_currency: row.from_currency,
        to_currency: row.to_currency,
        rate: row.rate.parse().unwrap_or_else(|_| Decimal::ZERO),
        timestamp: DateTime::parse_from_rfc3339(&row.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        source: row.source.unwrap_or_else(|| "manual".to_string()),
        notes: row.notes,
        created_at: row.created_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now),
    }))
}

/// List all exchange rates for a currency pair
pub async fn list_exchange_rates(
    pool: &SqlitePool,
    from_currency: &str,
    to_currency: &str,
) -> Result<Vec<ExchangeRate>> {
    let rows = sqlx::query!(
        r#"
        SELECT id as "id!", from_currency as "from_currency!",
               to_currency as "to_currency!", rate as "rate!",
               timestamp as "timestamp: String",
               source, notes,
               created_at as "created_at: String"
        FROM exchange_rates
        WHERE from_currency = ? AND to_currency = ?
        ORDER BY timestamp DESC
        "#,
        from_currency,
        to_currency
    )
    .fetch_all(pool)
    .await?;

    let rates = rows
        .into_iter()
        .map(|row| ExchangeRate {
            id: row.id,
            from_currency: row.from_currency,
            to_currency: row.to_currency,
            rate: row.rate.parse().unwrap_or_else(|_| Decimal::ZERO),
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            source: row.source.unwrap_or_else(|| "manual".to_string()),
            notes: row.notes,
            created_at: row.created_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        })
        .collect();

    Ok(rates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use chrono::TimeZone;
    use std::str::FromStr;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;
        migrations::run(&pool).await?;
        Ok(pool)
    }

    // ============ Currency Tests ============

    #[tokio::test]
    async fn test_list_currencies_returns_seeded_data() -> Result<()> {
        let pool = setup_test_db().await?;

        let currencies = list_currencies(&pool).await?;

        // Should have seeded currencies from migration
        assert!(currencies.len() >= 9);
        assert!(currencies.iter().any(|c| c.code == "USD"));
        assert!(currencies.iter().any(|c| c.code == "BTC"));
        assert!(currencies.iter().any(|c| c.code == "ETH"));

        Ok(())
    }

    #[tokio::test]
    async fn test_currency_ordering() -> Result<()> {
        let pool = setup_test_db().await?;

        let currencies = list_currencies(&pool).await?;

        // Should be ordered: fiat, stablecoin, crypto
        let first_fiat_idx = currencies.iter().position(|c| c.asset_type == AssetType::Fiat);
        let first_stable_idx = currencies.iter().position(|c| c.asset_type == AssetType::Stablecoin);
        let first_crypto_idx = currencies.iter().position(|c| c.asset_type == AssetType::Crypto);

        assert!(first_fiat_idx.is_some());
        assert!(first_stable_idx.is_some());
        assert!(first_crypto_idx.is_some());

        assert!(first_fiat_idx.unwrap() < first_stable_idx.unwrap());
        assert!(first_stable_idx.unwrap() < first_crypto_idx.unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_currency_by_code() -> Result<()> {
        let pool = setup_test_db().await?;

        let currency = get_currency(&pool, "USD").await?;
        assert!(currency.is_some());
        let usd = currency.unwrap();
        assert_eq!(usd.code, "USD");
        assert_eq!(usd.name, "US Dollar");
        assert_eq!(usd.symbol, "$");
        assert_eq!(usd.decimals, 2);
        assert_eq!(usd.asset_type, AssetType::Fiat);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_currency_nonexistent_returns_none() -> Result<()> {
        let pool = setup_test_db().await?;

        let currency = get_currency(&pool, "NONEXISTENT").await?;
        assert!(currency.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_add_custom_currency() -> Result<()> {
        let pool = setup_test_db().await?;

        let new_currency = Currency {
            code: "DOGE".to_string(),
            name: "Dogecoin".to_string(),
            symbol: "Ð".to_string(),
            decimals: 8,
            asset_type: AssetType::Crypto,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        add_currency(&pool, &new_currency).await?;

        let currency = get_currency(&pool, "DOGE").await?;
        assert!(currency.is_some());
        let doge = currency.unwrap();
        assert_eq!(doge.code, "DOGE");
        assert_eq!(doge.name, "Dogecoin");
        assert_eq!(doge.symbol, "Ð");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_currency() -> Result<()> {
        let pool = setup_test_db().await?;

        // Get existing currency
        let mut btc = get_currency(&pool, "BTC").await?.unwrap();

        // Modify it
        btc.name = "Bitcoin Updated".to_string();
        btc.enabled = false;

        update_currency(&pool, &btc).await?;

        // Verify update
        let updated = get_currency(&pool, "BTC").await?.unwrap();
        assert_eq!(updated.name, "Bitcoin Updated");
        assert_eq!(updated.enabled, false);

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_currency() -> Result<()> {
        let pool = setup_test_db().await?;

        // Add a custom currency
        let custom = Currency {
            code: "TEST".to_string(),
            name: "Test Coin".to_string(),
            symbol: "TST".to_string(),
            decimals: 8,
            asset_type: AssetType::Crypto,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        add_currency(&pool, &custom).await?;

        // Remove it
        remove_currency(&pool, "TEST").await?;

        // Verify removed
        let currency = get_currency(&pool, "TEST").await?;
        assert!(currency.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_currency_exists() -> Result<()> {
        let pool = setup_test_db().await?;

        assert!(currency_exists(&pool, "USD").await?);
        assert!(currency_exists(&pool, "BTC").await?);
        assert!(!currency_exists(&pool, "NONEXISTENT").await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_currency_asset_types() -> Result<()> {
        let pool = setup_test_db().await?;

        let usd = get_currency(&pool, "USD").await?.unwrap();
        assert_eq!(usd.asset_type, AssetType::Fiat);

        let usdt = get_currency(&pool, "USDT").await?.unwrap();
        assert_eq!(usdt.asset_type, AssetType::Stablecoin);

        let btc = get_currency(&pool, "BTC").await?.unwrap();
        assert_eq!(btc.asset_type, AssetType::Crypto);

        Ok(())
    }

    // ============ Exchange Rate Tests ============

    #[tokio::test]
    async fn test_add_exchange_rate() -> Result<()> {
        let pool = setup_test_db().await?;

        let rate = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0019").unwrap(),
            timestamp: Utc::now(),
            source: "manual".to_string(),
            notes: Some("Test rate".to_string()),
            created_at: Utc::now(),
        };

        let rate_id = add_exchange_rate(&pool, &rate).await?;
        assert!(rate_id > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_latest_exchange_rate() -> Result<()> {
        let pool = setup_test_db().await?;

        // Add multiple rates at different times
        let timestamp1 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let timestamp2 = Utc.with_ymd_and_hms(2024, 1, 2, 12, 0, 0).unwrap();

        let rate1 = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0019").unwrap(),
            timestamp: timestamp1,
            source: "manual".to_string(),
            notes: None,
            created_at: Utc::now(),
        };

        let rate2 = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0020").unwrap(),
            timestamp: timestamp2,
            source: "manual".to_string(),
            notes: None,
            created_at: Utc::now(),
        };

        add_exchange_rate(&pool, &rate1).await?;
        add_exchange_rate(&pool, &rate2).await?;

        // Get latest (should be rate2)
        let latest = get_latest_exchange_rate(&pool, "CRC", "USD").await?.unwrap();
        assert_eq!(latest.rate, Decimal::from_str("0.0020").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_exchange_rate_at_time() -> Result<()> {
        let pool = setup_test_db().await?;

        let timestamp1 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let timestamp2 = Utc.with_ymd_and_hms(2024, 1, 3, 12, 0, 0).unwrap();

        let rate1 = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0019").unwrap(),
            timestamp: timestamp1,
            source: "manual".to_string(),
            notes: None,
            created_at: Utc::now(),
        };

        let rate2 = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0020").unwrap(),
            timestamp: timestamp2,
            source: "manual".to_string(),
            notes: None,
            created_at: Utc::now(),
        };

        add_exchange_rate(&pool, &rate1).await?;
        add_exchange_rate(&pool, &rate2).await?;

        // Get rate at Jan 2 (should return Jan 1 rate, closest before)
        let query_time = Utc.with_ymd_and_hms(2024, 1, 2, 12, 0, 0).unwrap();
        let rate = get_exchange_rate_at_time(&pool, "CRC", "USD", query_time).await?.unwrap();
        assert_eq!(rate.rate, Decimal::from_str("0.0019").unwrap());

        // Get rate at Jan 4 (should return Jan 3 rate)
        let query_time = Utc.with_ymd_and_hms(2024, 1, 4, 12, 0, 0).unwrap();
        let rate = get_exchange_rate_at_time(&pool, "CRC", "USD", query_time).await?.unwrap();
        assert_eq!(rate.rate, Decimal::from_str("0.0020").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_exchange_rates() -> Result<()> {
        let pool = setup_test_db().await?;

        // Add 3 rates
        for i in 1..=3 {
            let timestamp = Utc.with_ymd_and_hms(2024, 1, i, 12, 0, 0).unwrap();
            let rate = ExchangeRate {
                id: 0,
                from_currency: "CRC".to_string(),
                to_currency: "USD".to_string(),
                rate: Decimal::from_str(&format!("0.00{}", 18 + i)).unwrap(),
                timestamp,
                source: "manual".to_string(),
                notes: None,
                created_at: Utc::now(),
            };
            add_exchange_rate(&pool, &rate).await?;
        }

        let rates = list_exchange_rates(&pool, "CRC", "USD").await?;
        assert_eq!(rates.len(), 3);

        // Should be ordered by timestamp DESC
        assert_eq!(rates[0].rate, Decimal::from_str("0.0021").unwrap());
        assert_eq!(rates[1].rate, Decimal::from_str("0.0020").unwrap());
        assert_eq!(rates[2].rate, Decimal::from_str("0.0019").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_exchange_rate_upsert_on_conflict() -> Result<()> {
        let pool = setup_test_db().await?;

        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // Add initial rate
        let rate1 = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0019").unwrap(),
            timestamp,
            source: "manual".to_string(),
            notes: Some("Initial".to_string()),
            created_at: Utc::now(),
        };
        add_exchange_rate(&pool, &rate1).await?;

        // Add same pair and timestamp with different rate (should update)
        let rate2 = ExchangeRate {
            id: 0,
            from_currency: "CRC".to_string(),
            to_currency: "USD".to_string(),
            rate: Decimal::from_str("0.0021").unwrap(),
            timestamp,
            source: "api".to_string(),
            notes: Some("Updated".to_string()),
            created_at: Utc::now(),
        };
        add_exchange_rate(&pool, &rate2).await?;

        // Should have only one rate (updated)
        let rates = list_exchange_rates(&pool, "CRC", "USD").await?;
        assert_eq!(rates.len(), 1);
        assert_eq!(rates[0].rate, Decimal::from_str("0.0021").unwrap());
        assert_eq!(rates[0].source, "api");
        assert_eq!(rates[0].notes, Some("Updated".to_string()));

        Ok(())
    }
}
