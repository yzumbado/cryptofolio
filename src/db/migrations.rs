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

    Ok(())
}
