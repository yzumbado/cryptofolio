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
