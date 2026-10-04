# Cryptofolio Data Model

**Version:** 0.6.0  
**Last Updated:** October 2026

This document explains the key data model concepts and the relationships between
them. The authoritative SQL schema is `src/db/schema.rs`; every table and column
below is transcribed from it.

---

## Schema Management

The schema is defined once in `src/db/schema.rs` and applied by
`schema::create(&pool)` at startup. Every statement uses `CREATE TABLE IF NOT
EXISTS`, so calling it against an existing database is safe.

- **No migration system.** There is no `migrations.rs` and no `_migrations`
  table. Versioned migrations are planned for v1.0.
- **To reset during development:** delete the database file and restart, or point
  the `CRYPTOFOLIO_DB` environment variable at a throwaway file.
- `PRAGMA journal_mode=WAL` and `PRAGMA foreign_keys=ON` are enabled.
- The default path is `<config dir>/database.sqlite`
  (`AppConfig::database_path()`); `CRYPTOFOLIO_DB` overrides it. A stale
  `~/.config/cryptofolio` copy can coexist on a machine — confirm which file is
  live before resetting.

All monetary values are stored as `TEXT` in `rust_decimal::Decimal` string form —
never `REAL`.

---

## Core Entities

### Category

Account groupings, seeded with `banking`, `trading`, `cold-storage`,
`hot-wallets`, and `on-ramp`.

```
categories
  id          TEXT      PK
  name        TEXT      NOT NULL UNIQUE
  sort_order  INTEGER   DEFAULT 0
  created_at  DATETIME  DEFAULT CURRENT_TIMESTAMP
```

### Account

A place where assets are held. Accounts are never hard-deleted; removal archives
them (`archived` flag) so the immutable ledger is preserved.

```
accounts
  id            TEXT      PK
  name          TEXT      NOT NULL UNIQUE
  category_id   TEXT      → categories.id
  account_type  TEXT      NOT NULL
  config        TEXT
  sync_enabled  BOOLEAN   DEFAULT FALSE
  archived      BOOLEAN   NOT NULL DEFAULT 0
  created_at    DATETIME  DEFAULT CURRENT_TIMESTAMP
```

`account_type` is one of `exchange`, `hardware_wallet`, `software_wallet`,
`custodial_service`, `bank`.

| Type | Sync mode | Holdings managed by |
|---|---|---|
| `exchange` | Exchange API (Binance) | `sync` |
| `hardware_wallet` | Blockchain API | `wallet sync` |
| `software_wallet` | Blockchain API | `wallet sync` |
| `custodial_service` | Manual | Transaction history |
| `bank` | Manual | Transaction history |

### Wallet Address

A blockchain address or HD wallet (xpub) attached to an account. One account can
hold multiple addresses across chains.

```
wallet_addresses
  id              INTEGER   PK AUTOINCREMENT
  account_id      TEXT      → accounts.id ON DELETE CASCADE
  blockchain      TEXT      NOT NULL
  address         TEXT      NOT NULL
  label           TEXT
  address_type    TEXT      legacy | segwit | native_segwit | taproot | erc20 | ...
  xpub            TEXT
  derivation_path TEXT
  network         TEXT      DEFAULT 'mainnet'
  last_synced_at  DATETIME
  created_at      DATETIME  DEFAULT CURRENT_TIMESTAMP
  UNIQUE(account_id, blockchain, address)
```

### Currency

Asset definitions (fiat, crypto, stablecoin), seeded with USD, CRC, EUR, BTC,
ETH, SOL, ADA, BNB, TAO, USDT, USDC.

```
currencies
  code        TEXT      PK
  name        TEXT      NOT NULL
  symbol      TEXT      NOT NULL
  decimals    INTEGER   NOT NULL DEFAULT 2
  asset_type  TEXT      NOT NULL CHECK IN ('fiat', 'crypto', 'stablecoin')
  enabled     BOOLEAN   NOT NULL DEFAULT 1
  created_at  DATETIME  DEFAULT CURRENT_TIMESTAMP
  updated_at  DATETIME  DEFAULT CURRENT_TIMESTAMP
```

### Exchange Rate

Point-in-time rates. Rows are never deleted — they are needed for cost basis
replay.

```
exchange_rates
  id             INTEGER   PK AUTOINCREMENT
  from_currency  TEXT      NOT NULL
  to_currency    TEXT      NOT NULL
  rate           TEXT      NOT NULL
  timestamp      DATETIME  NOT NULL
  source         TEXT      DEFAULT 'manual'
  notes          TEXT
  created_at     DATETIME  DEFAULT CURRENT_TIMESTAMP
  UNIQUE(from_currency, to_currency, timestamp)
```

---

## Transaction

Immutable record of a financial event. The timestamp is the actual event date,
not insertion time.

```
transactions
  id                 INTEGER   PK AUTOINCREMENT
  tx_type            TEXT      NOT NULL CHECK IN (13 values below)
  from_account_id    TEXT      → accounts.id
  from_asset         TEXT
  from_quantity      TEXT
  to_account_id      TEXT      → accounts.id
  to_asset           TEXT
  to_quantity        TEXT
  price_usd          TEXT
  price_currency     TEXT      currency for price_amount
  price_amount       TEXT
  exchange_rate      TEXT      e.g. 550 CRC per 1 USD
  exchange_rate_pair TEXT      e.g. "CRC/USD"
  fee                TEXT
  fee_asset          TEXT
  tx_hash            TEXT      UNIQUE
  external_id        TEXT      UNIQUE
  source             TEXT      NOT NULL DEFAULT 'manual' CHECK IN (7 values below)
  trust_level        TEXT      NOT NULL DEFAULT 'unverified' CHECK IN (4 values below)
  notes              TEXT
  timestamp          DATETIME  NOT NULL
  created_at         DATETIME  NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
```

**`tx_type` CHECK values (13):** `buy`, `sell`, `transfer_in`, `transfer_out`,
`transfer_internal`, `swap`, `stake`, `unstake`, `earn`, `receive`, `fee`,
`airdrop`, `correction`.

`TransactionType::from_str` also accepts aliases — `deposit` → `transfer_in`,
`withdrawal` / `send` → `transfer_out`, `transfer` → `transfer_internal`,
`trade` → `swap`, `reward` / `interest` → `earn`, `incoming` → `receive` — but
only the canonical 13 are ever stored.

**`source` CHECK values (7):** `binance_api`, `binance_csv`, `etherscan`,
`blockfrost`, `helius`, `blockstream`, `manual`.

**`trust_level` CHECK values (4):** `exchange_verified`, `chain_verified`,
`manual`, `unverified`.

### Type → ledger effects

| Type | Holdings effect | Tax lot effect |
|---|---|---|
| `buy` | +quantity | Creates acquisition lot |
| `sell` | -quantity | Consumes lots FIFO, records gain |
| `receive` | +quantity | Creates acquisition lot |
| `transfer_in` | +quantity | Creates acquisition lot |
| `transfer_out` | -quantity | Consumes lots FIFO |
| `transfer_internal` | move between own accounts | No P&L event |
| `swap` | -from, +to | Disposal of from + acquisition of to |
| `stake` / `earn` / `airdrop` | +quantity | Acquisition |
| `unstake` | -quantity | Disposal |
| `fee` | -quantity | Consumes lots FIFO |
| `correction` | ledger adjustment | Adjusts rebuilt lots in `pnl backfill` |

### Deduplication

Two schema-level `UNIQUE` keys reject duplicate inserts at the engine level
(no EXISTS pre-check in application code):

- `tx_hash` — canonical identifier for on-chain events.
- `external_id` — identifier for exchange events (e.g. Binance order ID).

### Append-only enforcement and corrections

Two `BEFORE UPDATE` / `BEFORE DELETE` triggers on `transactions` raise `ABORT`:

- `trg_transactions_no_update`
- `trg_transactions_no_delete`

The only sanctioned mutation is inserting a **new** row with
`tx_type = 'correction'`. `pnl backfill` treats corrections as lot adjustments
and reports the count it applied. Because accounts are archived rather than
deleted, no legitimate path purges ledger rows.

### Cross-wallet transfer links

```
transfer_links
  id         INTEGER   PK AUTOINCREMENT
  out_tx_id  INTEGER   NOT NULL → transactions.id
  in_tx_id   INTEGER   NOT NULL → transactions.id
  tx_hash    TEXT      NOT NULL
  fee_delta  TEXT      out_quantity - in_quantity
  verified   BOOLEAN   DEFAULT FALSE
  created_at DATETIME  DEFAULT CURRENT_TIMESTAMP
  UNIQUE(out_tx_id, in_tx_id)
```

---

## Holding

One row per `(account, asset)` pair.

```
holdings
  id                  INTEGER   PK AUTOINCREMENT
  account_id          TEXT      → accounts.id ON DELETE CASCADE
  asset               TEXT      NOT NULL
  quantity            TEXT      NOT NULL
  avg_cost_basis      TEXT
  avg_cost_basis_base TEXT
  cost_basis_currency TEXT      DEFAULT 'USD'
  updated_at          DATETIME  DEFAULT CURRENT_TIMESTAMP
  UNIQUE(account_id, asset)
```

**Two sources of truth:**

```
Synced account (has wallet address):
  quantity = set by wallet sync / exchange sync (set_quantity)
  cost basis = manual tx buy --cost-basis-only

Manual account (no wallet address):
  quantity = derived from transaction history (add_quantity / remove_quantity)
  cost basis = derived from transaction history
```

⚠️ **Double-count risk:** recording `tx buy` on a synced account without
`--cost-basis-only` calls `add_quantity`, which adds on top of the synced
balance. Always use `--cost-basis-only` for historical cost basis on synced
accounts.

---

## Tax Lot

Created for each acquisition event; holds the remaining quantity for FIFO
disposal.

```
tax_lots
  id                 INTEGER   PK AUTOINCREMENT
  account_id         TEXT      NOT NULL → accounts.id ON DELETE CASCADE
  asset              TEXT      NOT NULL
  quantity           TEXT      NOT NULL
  remaining_quantity TEXT      NOT NULL
  acquisition_price  TEXT      NOT NULL
  acquisition_date   DATETIME  NOT NULL
  acquisition_tx_id  INTEGER   → transactions.id
  cost_basis_method  TEXT      NOT NULL CHECK IN ('fifo', 'lifo', 'average')
  fully_disposed     BOOLEAN   DEFAULT FALSE
  created_at         DATETIME  DEFAULT CURRENT_TIMESTAMP
  updated_at         DATETIME  DEFAULT CURRENT_TIMESTAMP
```

`cost_basis_method` is stored **lowercase** (`'fifo'`, not `'FIFO'`).

**FIFO disposal walk:**

```
process_disposal(account, asset, qty, price, date):
  lots = SELECT * FROM tax_lots
         WHERE account_id = ? AND asset = ? AND NOT fully_disposed
         ORDER BY acquisition_date ASC
  for lot in lots:
    consume = min(remaining_quantity, qty_to_dispose)
    gain    = (disposal_price - acquisition_price) * consume
    realized_pnl.insert(...)
    lot.remaining_quantity -= consume
    if lot.remaining_quantity == 0: lot.fully_disposed = true
    qty_to_dispose -= consume
    if qty_to_dispose == 0: break
```

---

## Realized P&L

One row per lot consumed in a disposal.

```
realized_pnl
  id                  INTEGER   PK AUTOINCREMENT
  account_id          TEXT      NOT NULL → accounts.id ON DELETE CASCADE
  asset               TEXT      NOT NULL
  disposal_date       DATETIME  NOT NULL
  disposal_tx_id      INTEGER   → transactions.id
  quantity            TEXT      NOT NULL
  proceeds            TEXT      NOT NULL
  cost_basis          TEXT      NOT NULL
  realized_gain       TEXT      NOT NULL
  holding_period_days INTEGER
  tax_lot_id          INTEGER   → tax_lots.id
  cost_basis_method   TEXT      NOT NULL
  created_at          DATETIME  DEFAULT CURRENT_TIMESTAMP
```

---

## Sync and Provenance

### Binance sync state

Incremental watermarks, one row per Binance account.

```
binance_sync_state
  account_id            TEXT      PK → accounts.id ON DELETE CASCADE
  last_trade_sync       DATETIME
  last_deposit_sync     DATETIME
  last_withdrawal_sync  DATETIME
  last_fiat_sync        DATETIME
  last_transfer_sync    DATETIME
  last_trade_id         INTEGER
  last_sync_symbol      TEXT
  created_at            DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP
  updated_at            DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP
```

### Wallet sync state

On-chain block-height watermark keyed by raw address string. This is the only
per-address sync cursor — there is no `blockchain_sync_state` table.

```
wallet_sync_state
  address      TEXT      PK
  chain        TEXT      NOT NULL
  last_block   INTEGER
  last_sync_at DATETIME
  updated_at   DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP
```

### Sync audit log

Tamper-evident record of every sync operation. `account_id` references
`accounts(id)`; rows are retained because accounts are archived, never deleted.

```
sync_audit_log
  id          INTEGER   PK AUTOINCREMENT
  timestamp   DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP
  account_id  TEXT      NOT NULL → accounts.id
  address     TEXT      NOT NULL
  chain       TEXT      NOT NULL
  provider    TEXT      NOT NULL
  action      TEXT      NOT NULL
  records_in  INTEGER
  records_new INTEGER
  error       TEXT
  duration_ms INTEGER
```

### Reconciliation log

Per-wallet balance proof: computed balance vs live on-chain balance.

```
reconciliation_log
  id               INTEGER   PK AUTOINCREMENT
  account_id       TEXT      NOT NULL → accounts.id
  asset            TEXT      NOT NULL
  onchain_balance  TEXT      NOT NULL
  computed_balance TEXT      NOT NULL
  delta            TEXT      NOT NULL
  status           TEXT      NOT NULL CHECK IN ('verified', 'partial', 'unreconciled')
  block_height     INTEGER
  checked_at       DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP
```

> ⚠️ **This table has no writer yet.** No code path inserts into
> `reconciliation_log`. Balances are therefore **not reconciled** — never claim a
> holding is verified. Wiring this table is backlog item T1.

---

## Other Tables

### Portfolio snapshots

Daily per-account/asset balance snapshots (powers the timeline chart).

```
portfolio_snapshots
  id            INTEGER   PK AUTOINCREMENT
  snapshot_date DATE      NOT NULL
  account_id    TEXT      → accounts.id
  asset         TEXT      NOT NULL
  quantity      TEXT      NOT NULL
  price_usd     TEXT
  value_usd     TEXT
  created_at    DATETIME  DEFAULT CURRENT_TIMESTAMP
  UNIQUE(snapshot_date, account_id, asset)
```

### Address registry and discovery queue

Addresses ever seen, classified as owned or external, plus a queue of addresses
discovered during sync that await classification.

```
address_registry
  address          TEXT      PK
  chain            TEXT      NOT NULL
  classification   TEXT      NOT NULL DEFAULT 'unknown'
                             CHECK IN ('mine', 'exchange', 'defi_contract', 'external', 'unknown')
  label            TEXT
  account_id       TEXT      → accounts.id
  first_seen_tx_id INTEGER   → transactions.id
  classified_by    TEXT      DEFAULT 'auto' CHECK IN ('auto', 'manual')
  created_at       DATETIME  DEFAULT CURRENT_TIMESTAMP

discovery_queue
  id                 INTEGER   PK AUTOINCREMENT
  address            TEXT      NOT NULL
  chain              TEXT      NOT NULL
  discovered_from_tx INTEGER   → transactions.id
  status             TEXT      NOT NULL DEFAULT 'pending'
                               CHECK IN ('pending', 'classified', 'synced', 'skipped')
  queued_at          DATETIME  DEFAULT CURRENT_TIMESTAMP
  processed_at       DATETIME
  UNIQUE(address, chain)
```

### Keychain metadata

Storage metadata only — actual secrets live in macOS Keychain.

```
keychain_keys
  id             INTEGER   PK AUTOINCREMENT
  key_name       TEXT      NOT NULL UNIQUE
  storage_type   TEXT      NOT NULL CHECK IN ('keychain', 'toml', 'env')
  security_level TEXT      CHECK IN ('standard', 'touchid', 'touchid-only')
  last_accessed  DATETIME
  migrated_at    DATETIME
  created_at     DATETIME  DEFAULT CURRENT_TIMESTAMP
```

### Blockchain nodes

Node configs. `api_key_ref` names a keychain entry — never the key itself.

```
blockchain_nodes
  id          INTEGER   PK AUTOINCREMENT
  blockchain  TEXT      NOT NULL UNIQUE
  node_type   TEXT      NOT NULL CHECK IN ('local', 'public_api', 'custom')
  rpc_url     TEXT
  api_key_ref TEXT
  is_default  BOOLEAN   DEFAULT TRUE
  created_at  DATETIME  DEFAULT CURRENT_TIMESTAMP
```

---

## Entity Relationship Diagram

```
categories (1)────────────────────────────────< (N) accounts
                                                     │
                    ┌────────────────────────────────┼──────────────────┐
                    │                                │                  │
                    ▼                                ▼                  ▼
          wallet_addresses (N)             holdings (N)        sync_audit_log (N)
                    │                    (account_id, asset)   (account_id)
                    │
                    ▼
          wallet_sync_state (N)   (keyed by address, no FK)

   transactions (N)
          │
          ├──▶ tax_lots (N) ──▶ realized_pnl (N)
          ├──▶ transfer_links (N)
          └──▶ address_registry (N) / discovery_queue (N)

   accounts (1) ──▶ portfolio_snapshots (N) / reconciliation_log (N)
```

---

## Common Pitfalls

### 1. Holdings double-count on synced accounts

**Problem:** `tx buy` calls `holdings.add_quantity()`. On a synced account the
last sync already set the correct quantity, so recording a buy for cost basis
inflates the balance.

**Fix:** use `--cost-basis-only` on `tx buy` for synced accounts (the MCP
`record_transaction` tool exposes `cost_basis_only: true`). This creates the tax
lot without touching holdings.

### 2. Historical transactions timestamped to now

**Problem:** `tx buy` without `--date` uses the current time, so historical cost
basis entries appear as purchased today and FIFO ordering is wrong.

**Fix:** always pass `--date YYYY-MM-DD` (or ISO 8601) for historical entries.
Malformed timestamps now raise a parse error instead of silently falling back to
"now".

### 3. Reconciliation is not implemented

**Problem:** `reconciliation_log` exists but nothing writes to it, and
`holdings` is maintained independently by sync and by transaction recording.

**Fix:** none yet. Treat balances as unverified and say so; do not claim a
wallet is "reconciled". Wiring the writer is backlog item T1.

### 4. Cardano raw token amounts

**Problem:** Blockfrost returns native token quantities as raw integers (no
decimal conversion).

**Fix:** the Blockfrost client fetches `GET /assets/{unit}` metadata, divides by
`10^decimals`, and uses the ticker/name as the symbol. If the metadata fetch
fails, the token is skipped with a warning rather than stored with the wrong
value.

### 5. Removing a wallet

**Problem:** deleting dependent rows could orphan immutable history.

**Fix:** `wallet remove` (and `account remove`) **archives** the account instead
of deleting it. Transactions, holdings, wallet addresses, and `sync_audit_log`
rows are all retained; the account is hidden from the active set.
