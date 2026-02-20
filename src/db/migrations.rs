use sqlx::SqlitePool;

use crate::error::Result;

const MIGRATION_001: &str = r#"
-- Categories for grouping accounts
CREATE TABLE IF NOT EXISTS categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    sort_order  INTEGER DEFAULT 0,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Default categories
INSERT OR IGNORE INTO categories (id, name, sort_order) VALUES
    ('trading', 'Trading', 1),
    ('cold-storage', 'Cold Storage', 2),
    ('hot-wallets', 'Hot Wallets', 3);

-- Accounts (exchanges, wallets, etc.)
CREATE TABLE IF NOT EXISTS accounts (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    category_id     TEXT REFERENCES categories(id),
    account_type    TEXT NOT NULL,
    config          TEXT,
    sync_enabled    BOOLEAN DEFAULT FALSE,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Wallet addresses (for cold/hot wallets)
CREATE TABLE IF NOT EXISTS wallet_addresses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id      TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    blockchain      TEXT NOT NULL,
    address         TEXT NOT NULL,
    label           TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id, blockchain, address)
);

-- Holdings per account
CREATE TABLE IF NOT EXISTS holdings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id      TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    asset           TEXT NOT NULL,
    quantity        TEXT NOT NULL,
    avg_cost_basis  TEXT,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id, asset)
);

-- Transactions with source/destination
CREATE TABLE IF NOT EXISTS transactions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_type             TEXT NOT NULL,

    from_account_id     TEXT REFERENCES accounts(id),
    from_asset          TEXT,
    from_quantity       TEXT,

    to_account_id       TEXT REFERENCES accounts(id),
    to_asset            TEXT,
    to_quantity         TEXT,

    price_usd           TEXT,
    fee                 TEXT,
    fee_asset           TEXT,

    external_id         TEXT,
    notes               TEXT,
    timestamp           DATETIME NOT NULL,
    created_at          DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Portfolio snapshots
CREATE TABLE IF NOT EXISTS snapshots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    total_value_usd TEXT NOT NULL,
    snapshot_data   TEXT NOT NULL,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_holdings_account ON holdings(account_id);
CREATE INDEX IF NOT EXISTS idx_holdings_asset ON holdings(asset);
CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON transactions(timestamp);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(tx_type);
CREATE INDEX IF NOT EXISTS idx_wallet_addresses_account ON wallet_addresses(account_id);

-- Migration tracking table
CREATE TABLE IF NOT EXISTS _migrations (
    id          INTEGER PRIMARY KEY,
    applied_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);
"#;

const MIGRATION_002: &str = r#"
-- Currency metadata table
CREATE TABLE IF NOT EXISTS currencies (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 2,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('fiat', 'crypto', 'stablecoin')),
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Seed with common currencies
INSERT OR IGNORE INTO currencies (code, name, symbol, decimals, asset_type, enabled) VALUES
('USD', 'US Dollar', '$', 2, 'fiat', 1),
('CRC', 'Costa Rican Colón', '₡', 2, 'fiat', 1),
('EUR', 'Euro', '€', 2, 'fiat', 1),
('BTC', 'Bitcoin', '₿', 8, 'crypto', 1),
('ETH', 'Ethereum', 'Ξ', 18, 'crypto', 1),
('USDT', 'Tether USD', 'USDT', 6, 'stablecoin', 1),
('USDC', 'USD Coin', 'USDC', 6, 'stablecoin', 1),
('BNB', 'Binance Coin', 'BNB', 8, 'crypto', 1),
('SOL', 'Solana', 'SOL', 9, 'crypto', 1);

-- Exchange rates cache table (for manual entry and future API)
CREATE TABLE IF NOT EXISTS exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    source TEXT DEFAULT 'manual',
    notes TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_currency, to_currency, timestamp)
);

-- Index for quick lookups
CREATE INDEX IF NOT EXISTS idx_exchange_rates_lookup
ON exchange_rates(from_currency, to_currency, timestamp DESC);

-- Add cost_basis_currency to holdings
ALTER TABLE holdings ADD COLUMN cost_basis_currency TEXT DEFAULT 'USD';
ALTER TABLE holdings ADD COLUMN avg_cost_basis_base TEXT;

-- Add multi-currency fields to transactions
ALTER TABLE transactions ADD COLUMN price_currency TEXT;
ALTER TABLE transactions ADD COLUMN price_amount TEXT;
ALTER TABLE transactions ADD COLUMN exchange_rate TEXT;
ALTER TABLE transactions ADD COLUMN exchange_rate_pair TEXT;

-- Add 'banking' category for bank accounts
INSERT OR IGNORE INTO categories (id, name, sort_order) VALUES
    ('banking', 'Banking', 0),
    ('on-ramp', 'On-Ramp', 4);
"#;

pub async fn run(pool: &SqlitePool) -> Result<()> {
    // Check if migration 1 has been applied
    let migration_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 1"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_exists.is_none() {
        // Apply migration
        sqlx::raw_sql(MIGRATION_001).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (1)")
            .execute(pool)
            .await?;
    }

    // Check if migration 2 has been applied
    let migration_2_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 2"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_2_exists.is_none() {
        // Apply migration 2
        sqlx::raw_sql(MIGRATION_002).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (2)")
            .execute(pool)
            .await?;
    }

    Ok(())
}
