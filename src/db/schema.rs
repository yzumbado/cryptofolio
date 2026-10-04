use sqlx::SqlitePool;

use crate::error::Result;

/// Create all tables. Safe to call on an existing database — every statement
/// uses IF NOT EXISTS. Drop the database file and restart to reset the schema.
pub async fn create(pool: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(SCHEMA).execute(pool).await?;
    Ok(())
}

const SCHEMA: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- Account categories
CREATE TABLE IF NOT EXISTS categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    sort_order INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO categories (id, name, sort_order) VALUES
    ('banking',      'Banking',      0),
    ('trading',      'Trading',      1),
    ('cold-storage', 'Cold Storage', 2),
    ('hot-wallets',  'Hot Wallets',  3),
    ('on-ramp',      'On-Ramp',      4);

-- Accounts (exchanges, wallets, DeFi positions, external sources)
CREATE TABLE IF NOT EXISTS accounts (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL UNIQUE,
    category_id  TEXT REFERENCES categories(id),
    account_type TEXT NOT NULL,
    config       TEXT,
    sync_enabled BOOLEAN DEFAULT FALSE,
    -- Soft-delete flag. Accounts are never hard-deleted (that would orphan or
    -- require purging immutable transactions). Removal archives instead.
    archived     BOOLEAN NOT NULL DEFAULT 0,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Wallet addresses (one account can have multiple addresses across chains)
CREATE TABLE IF NOT EXISTS wallet_addresses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id      TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    blockchain      TEXT NOT NULL,
    address         TEXT NOT NULL,
    label           TEXT,
    address_type    TEXT,
    xpub            TEXT,
    derivation_path TEXT,
    network         TEXT DEFAULT 'mainnet',
    last_synced_at  DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id, blockchain, address)
);

-- Asset definitions
CREATE TABLE IF NOT EXISTS currencies (
    code       TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    symbol     TEXT NOT NULL,
    decimals   INTEGER NOT NULL DEFAULT 2,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('fiat', 'crypto', 'stablecoin')),
    enabled    BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO currencies (code, name, symbol, decimals, asset_type) VALUES
    ('USD',  'US Dollar',         '$',    2, 'fiat'),
    ('CRC',  'Costa Rican Colón', '₡',    2, 'fiat'),
    ('EUR',  'Euro',              '€',    2, 'fiat'),
    ('BTC',  'Bitcoin',           '₿',    8, 'crypto'),
    ('ETH',  'Ethereum',          'Ξ',   18, 'crypto'),
    ('SOL',  'Solana',            'SOL',  9, 'crypto'),
    ('ADA',  'Cardano',           'ADA',  6, 'crypto'),
    ('BNB',  'Binance Coin',      'BNB',  8, 'crypto'),
    ('TAO',  'Bittensor',         'TAO',  9, 'crypto'),
    ('USDT', 'Tether USD',        'USDT', 6, 'stablecoin'),
    ('USDC', 'USD Coin',          'USDC', 6, 'stablecoin');

-- Historical prices at point-in-time (never delete — needed for cost basis replay)
CREATE TABLE IF NOT EXISTS exchange_rates (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    from_currency TEXT NOT NULL,
    to_currency   TEXT NOT NULL,
    rate          TEXT NOT NULL,
    timestamp     DATETIME NOT NULL,
    source        TEXT DEFAULT 'manual',
    notes         TEXT,
    created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_currency, to_currency, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_exchange_rates_lookup
    ON exchange_rates(from_currency, to_currency, timestamp DESC);

-- Immutable transaction ledger.
-- Never UPDATE or DELETE rows. Corrections are new rows with tx_type = 'correction'.
-- tx_hash is the canonical dedup key for on-chain events.
-- external_id is the dedup key for exchange events (Binance order ID, etc.).
CREATE TABLE IF NOT EXISTS transactions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_type         TEXT NOT NULL CHECK(tx_type IN (
                        'buy', 'sell',
                        'transfer_in', 'transfer_out', 'transfer_internal',
                        'swap', 'stake', 'unstake',
                        'earn', 'receive', 'fee', 'airdrop', 'correction')),

    from_account_id TEXT REFERENCES accounts(id),
    from_asset      TEXT,
    from_quantity   TEXT,

    to_account_id   TEXT REFERENCES accounts(id),
    to_asset        TEXT,
    to_quantity     TEXT,

    price_usd          TEXT,
    price_currency     TEXT,
    price_amount       TEXT,
    exchange_rate      TEXT,
    exchange_rate_pair TEXT,

    fee       TEXT,
    fee_asset TEXT,

    tx_hash     TEXT UNIQUE,
    external_id TEXT UNIQUE,
    source      TEXT NOT NULL DEFAULT 'manual'
                    CHECK(source IN ('binance_api', 'binance_csv', 'etherscan', 'blockfrost',
                                     'helius', 'blockstream', 'manual')),
    trust_level TEXT NOT NULL DEFAULT 'unverified'
                    CHECK(trust_level IN ('exchange_verified', 'chain_verified', 'manual', 'unverified')),

    notes      TEXT,
    timestamp  DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_transactions_timestamp     ON transactions(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_transactions_from_account  ON transactions(from_account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_to_account    ON transactions(to_account_id);

-- Immutability enforcement. The ledger is append-only: the ONLY sanctioned
-- mutation is inserting a new row with tx_type = 'correction'. These triggers
-- make the guarantee real at the engine level — any UPDATE or DELETE aborts.
-- Accounts are archived rather than deleted, so no legitimate path purges rows.
CREATE TRIGGER IF NOT EXISTS trg_transactions_no_update
BEFORE UPDATE ON transactions
BEGIN
    SELECT RAISE(ABORT, 'transactions are immutable: record a new row with tx_type = ''correction'' instead of updating');
END;

CREATE TRIGGER IF NOT EXISTS trg_transactions_no_delete
BEFORE DELETE ON transactions
BEGIN
    SELECT RAISE(ABORT, 'transactions are immutable: rows cannot be deleted (archive the account instead)');
END;

-- Links the two sides of a cross-wallet transfer.
-- Connects the exchange withdrawal row to the on-chain inflow row.
-- fee_delta = out_quantity - in_quantity (should equal the network fee, nothing more).
CREATE TABLE IF NOT EXISTS transfer_links (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    out_tx_id  INTEGER NOT NULL REFERENCES transactions(id),
    in_tx_id   INTEGER NOT NULL REFERENCES transactions(id),
    tx_hash    TEXT NOT NULL,
    fee_delta  TEXT,
    verified   BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(out_tx_id, in_tx_id)
);

-- Current holdings per account (maintained by sync engine).
-- Should always reconcile with SUM of transactions — see reconciliation_log.
CREATE TABLE IF NOT EXISTS holdings (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id          TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    asset               TEXT NOT NULL,
    quantity            TEXT NOT NULL,
    avg_cost_basis      TEXT,
    avg_cost_basis_base TEXT,
    cost_basis_currency TEXT DEFAULT 'USD',
    updated_at          DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id, asset)
);

CREATE INDEX IF NOT EXISTS idx_holdings_account ON holdings(account_id);
CREATE INDEX IF NOT EXISTS idx_holdings_asset   ON holdings(asset);

-- FIFO/LIFO cost basis lots
CREATE TABLE IF NOT EXISTS tax_lots (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id         TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    asset              TEXT NOT NULL,
    quantity           TEXT NOT NULL,
    remaining_quantity TEXT NOT NULL,
    acquisition_price  TEXT NOT NULL,
    acquisition_date   DATETIME NOT NULL,
    acquisition_tx_id  INTEGER REFERENCES transactions(id),
    cost_basis_method  TEXT NOT NULL CHECK(cost_basis_method IN ('fifo', 'lifo', 'average')),
    fully_disposed     BOOLEAN DEFAULT FALSE,
    created_at         DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at         DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_tax_lots_account_asset    ON tax_lots(account_id, asset);
CREATE INDEX IF NOT EXISTS idx_tax_lots_acquisition_date ON tax_lots(acquisition_date);
CREATE INDEX IF NOT EXISTS idx_tax_lots_disposed         ON tax_lots(fully_disposed);

-- Realized P&L from disposals
CREATE TABLE IF NOT EXISTS realized_pnl (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id          TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    asset               TEXT NOT NULL,
    disposal_date       DATETIME NOT NULL,
    disposal_tx_id      INTEGER REFERENCES transactions(id),
    quantity            TEXT NOT NULL,
    proceeds            TEXT NOT NULL,
    cost_basis          TEXT NOT NULL,
    realized_gain       TEXT NOT NULL,
    holding_period_days INTEGER,
    tax_lot_id          INTEGER REFERENCES tax_lots(id),
    cost_basis_method   TEXT NOT NULL,
    created_at          DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_realized_pnl_account ON realized_pnl(account_id);
CREATE INDEX IF NOT EXISTS idx_realized_pnl_asset   ON realized_pnl(asset);
CREATE INDEX IF NOT EXISTS idx_realized_pnl_date    ON realized_pnl(disposal_date);

-- Daily balance snapshots per account/asset (powers the Timeline chart)
CREATE TABLE IF NOT EXISTS portfolio_snapshots (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_date DATE NOT NULL,
    account_id    TEXT REFERENCES accounts(id),
    asset         TEXT NOT NULL,
    quantity      TEXT NOT NULL,
    price_usd     TEXT,
    value_usd     TEXT,
    created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(snapshot_date, account_id, asset)
);

CREATE INDEX IF NOT EXISTS idx_portfolio_snapshots_date ON portfolio_snapshots(snapshot_date DESC);

-- Every address ever seen, classified so we know if it is ours or external
CREATE TABLE IF NOT EXISTS address_registry (
    address          TEXT PRIMARY KEY,
    chain            TEXT NOT NULL,
    classification   TEXT NOT NULL DEFAULT 'unknown'
                         CHECK(classification IN ('mine', 'exchange', 'defi_contract', 'external', 'unknown')),
    label            TEXT,
    account_id       TEXT REFERENCES accounts(id),
    first_seen_tx_id INTEGER REFERENCES transactions(id),
    classified_by    TEXT DEFAULT 'auto' CHECK(classified_by IN ('auto', 'manual')),
    created_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Addresses discovered during sync that are pending classification / wallet sync
CREATE TABLE IF NOT EXISTS discovery_queue (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    address            TEXT NOT NULL,
    chain              TEXT NOT NULL,
    discovered_from_tx INTEGER REFERENCES transactions(id),
    status             TEXT NOT NULL DEFAULT 'pending'
                           CHECK(status IN ('pending', 'classified', 'synced', 'skipped')),
    queued_at          DATETIME DEFAULT CURRENT_TIMESTAMP,
    processed_at       DATETIME,
    UNIQUE(address, chain)
);

-- Per-wallet balance verification: computed balance vs live on-chain balance
CREATE TABLE IF NOT EXISTS reconciliation_log (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id       TEXT NOT NULL REFERENCES accounts(id),
    asset            TEXT NOT NULL,
    onchain_balance  TEXT NOT NULL,
    computed_balance TEXT NOT NULL,
    delta            TEXT NOT NULL,
    status           TEXT NOT NULL CHECK(status IN ('verified', 'partial', 'unreconciled')),
    block_height     INTEGER,
    checked_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reconciliation_account
    ON reconciliation_log(account_id, checked_at DESC);

-- Secret storage metadata. Actual secrets live in macOS Keychain — never stored here.
CREATE TABLE IF NOT EXISTS keychain_keys (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    key_name       TEXT NOT NULL UNIQUE,
    storage_type   TEXT NOT NULL CHECK(storage_type IN ('keychain', 'toml', 'env')),
    security_level TEXT CHECK(security_level IN ('standard', 'touchid', 'touchid-only')),
    last_accessed  DATETIME,
    migrated_at    DATETIME,
    created_at     DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Binance incremental sync watermarks (one row per Binance account)
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

-- On-chain sync watermarks (block height per address)
CREATE TABLE IF NOT EXISTS wallet_sync_state (
    address      TEXT PRIMARY KEY,
    chain        TEXT NOT NULL,
    last_block   INTEGER,
    last_sync_at DATETIME,
    updated_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Blockchain node configs. api_key_ref points to a keychain_keys entry — never store the key itself here.
CREATE TABLE IF NOT EXISTS blockchain_nodes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    blockchain  TEXT NOT NULL UNIQUE,
    node_type   TEXT NOT NULL CHECK(node_type IN ('local', 'public_api', 'custom')),
    rpc_url     TEXT,
    api_key_ref TEXT,
    is_default  BOOLEAN DEFAULT TRUE,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Tamper-evident log of every sync operation
CREATE TABLE IF NOT EXISTS sync_audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    account_id  TEXT NOT NULL REFERENCES accounts(id),
    address     TEXT NOT NULL,
    chain       TEXT NOT NULL,
    provider    TEXT NOT NULL,
    action      TEXT NOT NULL,
    records_in  INTEGER,
    records_new INTEGER,
    error       TEXT,
    duration_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_sync_audit_log
    ON sync_audit_log(account_id, timestamp DESC);
"#;
