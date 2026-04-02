# Database Specification

Engine: **SQLite** via `sqlx` with async runtime.

Location: `~/.config/cryptofolio/database.sqlite` (production)
Test: `sqlite::memory:` (in-memory, created fresh per test)

## Migration Convention

File: `src/db/migrations.rs`

- Each migration is a `const MIGRATION_NNN: &str` SQL string
- The `_migrations` table tracks applied migration IDs
- Migrations are additive only — no DROP TABLE, no ALTER COLUMN (until v1.0)
- New tables, indexes, and columns are always `CREATE/ADD ... IF NOT EXISTS`
- No data migration scripts until first public release (dev mode: schema-only)

## Table Overview

| Table | Purpose |
|-------|---------|
| `categories` | Account groupings (trading, cold-storage, hot-wallets) |
| `accounts` | Exchanges, wallets, and other portfolio sources |
| `wallet_addresses` | Blockchain addresses per account |
| `holdings` | Current asset quantities per account |
| `transactions` | Full transaction ledger (buy/sell/transfer/swap) |
| `tax_lots` | FIFO cost basis lots for P&L |
| `realized_pnl` | Computed disposal events |
| `currencies` | Fiat and crypto currency definitions |
| `exchange_rates` | Point-in-time exchange rates |
| `binance_sync_state` | Incremental sync watermarks for Binance |
| `blockchain_nodes` | Custom node configurations |
| `blockchain_sync_state` | Legacy sync state (superseded by wallet_sync_state) |
| `wallet_sync_state` | Block-height watermarks per wallet address |
| `sync_audit_log` | Tamper-evident record of every sync operation |

## Key Patterns

### External IDs

All synced records use `external_id` (e.g. `blockstream-{txid}`, `etherscan-{hash}`)
for idempotent upserts. Duplicate syncs skip existing records.

### Decimal Storage

All monetary values stored as `TEXT` using `rust_decimal::Decimal` string form.
Never use `REAL` for financial data.

### Timestamps

All timestamps stored as ISO 8601 UTC strings (`DATETIME` column type in SQLite).
`chrono::DateTime<Utc>` on the Rust side.

### LIMIT Parameterization

sqlx does not support binding `LIMIT` as a query parameter. Where dynamic limits
are needed, they are formatted as integers (type-safe — never user strings).

## sync_audit_log Schema

Every wallet sync operation writes two rows: `sync_start` and `sync_complete`.
Errors write a `sync_complete` row with the `error` column populated.

```sql
CREATE TABLE sync_audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    account_id  TEXT NOT NULL REFERENCES accounts(id),
    address     TEXT NOT NULL,
    chain       TEXT NOT NULL,
    provider    TEXT NOT NULL,
    action      TEXT NOT NULL,   -- "sync_start" | "sync_complete"
    records_in  INTEGER,         -- rows returned by provider
    records_new INTEGER,         -- new rows inserted
    error       TEXT,            -- NULL on success
    duration_ms INTEGER
);
```
