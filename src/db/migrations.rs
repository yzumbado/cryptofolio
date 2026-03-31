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

const MIGRATION_003: &str = r#"
-- Tax lots for FIFO/LIFO cost basis tracking
CREATE TABLE IF NOT EXISTS tax_lots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    quantity TEXT NOT NULL,
    remaining_quantity TEXT NOT NULL,
    acquisition_price TEXT NOT NULL,
    acquisition_date DATETIME NOT NULL,
    acquisition_tx_id INTEGER REFERENCES transactions(id),
    cost_basis_method TEXT NOT NULL CHECK(cost_basis_method IN ('fifo', 'lifo', 'average')),
    fully_disposed BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_tax_lots_account_asset ON tax_lots(account_id, asset);
CREATE INDEX IF NOT EXISTS idx_tax_lots_fully_disposed ON tax_lots(fully_disposed);
CREATE INDEX IF NOT EXISTS idx_tax_lots_acquisition_date ON tax_lots(acquisition_date);

-- Realized P&L from disposals
CREATE TABLE IF NOT EXISTS realized_pnl (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    disposal_date DATETIME NOT NULL,
    disposal_tx_id INTEGER REFERENCES transactions(id),
    quantity TEXT NOT NULL,
    proceeds TEXT NOT NULL,
    cost_basis TEXT NOT NULL,
    realized_gain TEXT NOT NULL,
    holding_period_days INTEGER,
    tax_lot_id INTEGER REFERENCES tax_lots(id),
    cost_basis_method TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_realized_pnl_account ON realized_pnl(account_id);
CREATE INDEX IF NOT EXISTS idx_realized_pnl_asset ON realized_pnl(asset);
CREATE INDEX IF NOT EXISTS idx_realized_pnl_date ON realized_pnl(disposal_date);
CREATE INDEX IF NOT EXISTS idx_realized_pnl_tax_lot ON realized_pnl(tax_lot_id);
"#;

const MIGRATION_005: &str = r#"
-- Keychain metadata tracking
CREATE TABLE IF NOT EXISTS keychain_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_name TEXT NOT NULL UNIQUE,
    storage_type TEXT NOT NULL CHECK(storage_type IN ('keychain', 'toml', 'env')),
    security_level TEXT CHECK(security_level IN ('standard', 'touchid', 'touchid-only')),
    last_accessed DATETIME,
    migrated_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_keychain_keys_name ON keychain_keys(key_name);
CREATE INDEX IF NOT EXISTS idx_keychain_keys_storage ON keychain_keys(storage_type);
"#;

const MIGRATION_006: &str = r#"
-- Binance sync state: tracks last successful sync timestamp per endpoint/account.
-- One row per exchange account. Used for incremental syncs.
CREATE TABLE IF NOT EXISTS binance_sync_state (
    account_id           TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    last_trade_sync      DATETIME,
    last_deposit_sync    DATETIME,
    last_withdrawal_sync DATETIME,
    last_fiat_sync       DATETIME,
    last_transfer_sync   DATETIME,
    last_trade_id        INTEGER,
    last_sync_symbol     TEXT,
    created_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_binance_sync_state_account ON binance_sync_state(account_id);
"#;

const MIGRATION_007: &str = r#"
-- Extend wallet_addresses for blockchain sync support
-- Add columns for xpub (HD wallets), address type, and sync tracking
ALTER TABLE wallet_addresses ADD COLUMN address_type TEXT;
ALTER TABLE wallet_addresses ADD COLUMN xpub TEXT;
ALTER TABLE wallet_addresses ADD COLUMN derivation_path TEXT;
ALTER TABLE wallet_addresses ADD COLUMN last_synced_at DATETIME;

-- Blockchain sync state: tracks sync progress per wallet address
CREATE TABLE IF NOT EXISTS blockchain_sync_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_address_id INTEGER NOT NULL REFERENCES wallet_addresses(id) ON DELETE CASCADE,
    last_block_height INTEGER,
    last_tx_hash TEXT,
    last_synced_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_address_id)
);

-- Blockchain node configurations (local nodes vs public APIs)
CREATE TABLE IF NOT EXISTS blockchain_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blockchain TEXT NOT NULL UNIQUE,
    node_type TEXT NOT NULL CHECK(node_type IN ('local', 'public_api', 'custom')),
    rpc_url TEXT,
    api_key TEXT,
    is_default BOOLEAN DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_blockchain_sync_state_wallet ON blockchain_sync_state(wallet_address_id);
CREATE INDEX IF NOT EXISTS idx_blockchain_nodes_blockchain ON blockchain_nodes(blockchain);
"#;

const MIGRATION_008: &str = r#"
-- Add network column to wallet_addresses for mainnet/testnet support
ALTER TABLE wallet_addresses ADD COLUMN network TEXT DEFAULT 'mainnet';
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

    // Check if migration 3 has been applied
    let migration_3_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 3"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_3_exists.is_none() {
        // Apply migration 3
        sqlx::raw_sql(MIGRATION_003).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (3)")
            .execute(pool)
            .await?;
    }

    // Check if migration 5 has been applied
    let migration_5_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 5"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_5_exists.is_none() {
        // Apply migration 5
        sqlx::raw_sql(MIGRATION_005).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (5)")
            .execute(pool)
            .await?;
    }

    // Check if migration 6 has been applied
    let migration_6_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 6"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_6_exists.is_none() {
        // Apply migration 6
        sqlx::raw_sql(MIGRATION_006).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (6)")
            .execute(pool)
            .await?;
    }

    // Check if migration 7 has been applied
    let migration_7_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 7"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_7_exists.is_none() {
        // Apply migration 7
        sqlx::raw_sql(MIGRATION_007).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (7)")
            .execute(pool)
            .await?;
    }

    // Check if migration 8 has been applied
    let migration_8_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM _migrations WHERE id = 8"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if migration_8_exists.is_none() {
        // Apply migration 8
        sqlx::raw_sql(MIGRATION_008).execute(pool).await?;

        // Mark migration as applied
        sqlx::query("INSERT OR IGNORE INTO _migrations (id) VALUES (8)")
            .execute(pool)
            .await?;
    }

    Ok(())
}
