# Database Specification

Engine: **SQLite** via `sqlx` with async runtime, WAL journal mode.

Location: `~/.config/cryptofolio/database.sqlite` (production)  
Test: `sqlite::memory:` (in-memory, created fresh per test)

## Schema management

Schema is defined in `src/db/schema.rs`. Call `schema::create(&pool)` once on startup — every statement is `CREATE TABLE IF NOT EXISTS`, so it is safe to call on an existing database.

**To reset the schema during development:** delete the database file and restart.  
Versioned migrations will be introduced at v1.0.

## Tables

| Table | Purpose |
|---|---|
| `categories` | Account groupings (trading, cold-storage, hot-wallets, on-ramp, banking) |
| `accounts` | Exchanges, wallets, DeFi positions, external sources |
| `wallet_addresses` | Blockchain addresses per account |
| `currencies` | Fiat and crypto asset definitions |
| `exchange_rates` | Point-in-time prices (immutable — needed for cost basis replay) |
| `transactions` | Immutable event ledger (buy/sell/transfer/swap/stake/earn/fee/airdrop/correction) |
| `transfer_links` | Links the two sides of a cross-wallet transfer for verification |
| `holdings` | Current balances per account/asset (maintained by sync engine) |
| `tax_lots` | FIFO/LIFO cost basis lots per acquisition event |
| `realized_pnl` | Computed disposal events with gain/loss |
| `portfolio_snapshots` | Daily balance snapshots per account/asset (powers Timeline chart) |
| `address_registry` | Every address ever seen, classified as mine/exchange/defi/external/unknown |
| `discovery_queue` | Addresses found during sync, pending classification and wallet sync |
| `reconciliation_log` | Per-wallet balance proof: computed vs live on-chain balance |
| `keychain_keys` | Secret storage metadata (actual secrets live in macOS Keychain) |
| `binance_sync_state` | Incremental sync watermarks per Binance account |
| `wallet_sync_state` | Block-height watermarks per wallet address |
| `blockchain_nodes` | Custom node configs (stores a Keychain key reference, never the key itself) |
| `sync_audit_log` | Tamper-evident record of every sync operation |

## Key patterns

### Deduplication

Transactions have two dedup keys enforced at the database level:

- `tx_hash TEXT UNIQUE` — canonical identifier for on-chain events (same hash regardless of which API returned it)
- `external_id TEXT UNIQUE` — identifier for exchange-only events (Binance order ID, etc.)

Both constraints are schema-level `UNIQUE` — a duplicate insert is rejected by SQLite itself, not by application logic.

### Immutable ledger

`transactions` rows are never updated or deleted. Corrections are new rows with `tx_type = 'correction'`. The `source` column records which system produced each row; `trust_level` records how it was verified.

### Decimal storage

All monetary values stored as `TEXT` using `rust_decimal::Decimal` string form. Never use `REAL` for financial data.

### Timestamps

All timestamps stored as ISO 8601 UTC strings. `chrono::DateTime<Utc>` on the Rust side.

### API key security

`blockchain_nodes.api_key_ref` stores the **name** of a Keychain entry, not the key itself. Retrieve the actual key via `config::keychain` at runtime.
