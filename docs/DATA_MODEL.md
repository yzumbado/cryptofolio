# Cryptofolio Data Model

**Version:** 0.5.0  
**Last Updated:** May 2026

This document explains the key data model concepts and the relationships between them. For the full SQL schema, see `src/db/migrations.rs`.

---

## Core Entities

### Account

An account represents a place where assets are held — an exchange, a hardware wallet, a software wallet, a custodial service, or a bank.

```
accounts
  id            TEXT  PK
  name          TEXT  UNIQUE
  category_id   TEXT  → categories.id
  account_type  TEXT  exchange | hardware_wallet | software_wallet | custodial_service | bank
  sync_enabled  BOOL
  created_at    DATETIME
```

**Account types and their sync behavior:**

| Type | Sync mode | Holdings managed by |
|---|---|---|
| `exchange` | Exchange API (Binance) | `exchange sync` |
| `hardware_wallet` | Blockchain API | `wallet sync` |
| `software_wallet` | Blockchain API | `wallet sync` |
| `custodial_service` | Manual | Transaction history |
| `bank` | Manual | Transaction history |

---

### Wallet Address

A blockchain address or HD wallet (xpub) attached to an account. One account can have multiple wallet addresses across different chains.

```
wallet_addresses
  id            INTEGER  PK
  account_id    TEXT     → accounts.id ON DELETE CASCADE
  blockchain    TEXT     bitcoin | ethereum | cardano | solana
  address       TEXT     on-chain address or xpub/ypub/zpub
  address_type  TEXT     legacy | segwit | native_segwit | taproot | erc20 | ...
  label         TEXT?
```

---

### Holding

One row per `(account, asset)` pair. Represents the current balance and average cost basis.

```
holdings
  id              INTEGER  PK
  account_id      TEXT     → accounts.id ON DELETE CASCADE
  asset           TEXT     BTC | ETH | ADA | USDT | ...
  quantity        TEXT     (stored as string for decimal precision)
  avg_cost_basis  TEXT?    average USD cost per unit
  updated_at      DATETIME
  UNIQUE(account_id, asset)
```

**Two sources of truth:**

```
Synced account (has wallet address):
  quantity = set by wallet sync (get_address_summary → holdings.set_quantity)
  cost basis = set by manual tx buy --cost-basis-only

Manual account (no wallet address):
  quantity = derived from transaction history (add_quantity / remove_quantity)
  cost basis = derived from transaction history
```

⚠️ **Double-count risk:** Recording `tx buy` on a synced account WITHOUT `--cost-basis-only` calls `add_quantity`, which adds on top of the synced balance. Always use `--cost-basis-only` for historical cost basis entry on synced accounts.

---

### Transaction

Immutable record of a financial event. Created at the actual event date, not insertion time.

```
transactions
  id              INTEGER  PK
  tx_type         TEXT     buy | sell | transfer_in | transfer_out |
                           transfer_internal | swap | receive | fee | send
  from_account_id TEXT?    → accounts.id
  from_asset      TEXT?
  from_quantity   TEXT?
  to_account_id   TEXT?    → accounts.id
  to_asset        TEXT?
  to_quantity     TEXT?
  price_usd       TEXT?    price per unit in USD
  fee             TEXT?
  fee_asset       TEXT?
  external_id     TEXT?    exchange trade ID or on-chain tx hash
  notes           TEXT?
  timestamp       DATETIME actual event date (not created_at)
  created_at      DATETIME insertion time
```

**Transaction type mapping:**

| Type | Holdings effect | Tax lot effect |
|---|---|---|
| `buy` | +quantity to account | Creates acquisition lot |
| `sell` | -quantity from account | Consumes lots FIFO, records gain |
| `receive` | +quantity to account | Creates acquisition lot |
| `transfer_in` | +quantity to account | Creates acquisition lot (inherits cost basis) |
| `transfer_out` | -quantity from account | Consumes lots FIFO |
| `transfer_internal` | move between own accounts | No P&L event |
| `swap` | -from, +to | Disposal of from + acquisition of to |
| `fee` | -quantity | Consumes lots FIFO |
| `send` | -quantity (on-chain import) | Consumes lots FIFO |

---

### Tax Lot

Created for every acquisition event. Tracks the remaining quantity for FIFO disposal.

```
tax_lots
  id                INTEGER  PK
  tx_id             INTEGER  → transactions.id
  account_id        TEXT     → accounts.id ON DELETE CASCADE
  asset             TEXT
  quantity          TEXT     original acquired quantity
  cost_per_unit     TEXT     acquisition price in USD
  acquisition_date  DATETIME actual purchase/receive date
  remaining_quantity TEXT    decremented on disposal
  fully_disposed    BOOL     true when remaining_quantity = 0
  method            TEXT     fifo | lifo | wacb
```

**FIFO disposal walk:**
```
process_disposal(account, asset, qty, price, date):
  lots = SELECT * FROM tax_lots
         WHERE account_id = ? AND asset = ? AND NOT fully_disposed
         ORDER BY acquisition_date ASC

  for lot in lots:
    consume = min(remaining, qty_to_dispose)
    gain = (disposal_price - lot.cost_per_unit) * consume
    realized_pnl.insert(...)
    lot.remaining_quantity -= consume
    if lot.remaining_quantity == 0: lot.fully_disposed = true
    qty_to_dispose -= consume
    if qty_to_dispose == 0: break
```

---

### Realized P&L

One row per lot consumed in a disposal.

```
realized_pnl
  id                INTEGER  PK
  disposal_tx_id    INTEGER  → transactions.id
  lot_tx_id         INTEGER  → tax_lots.id (the acquisition)
  account_id        TEXT     → accounts.id ON DELETE CASCADE
  asset             TEXT
  quantity_disposed TEXT
  cost_per_unit     TEXT
  disposal_price    TEXT
  realized_gain     TEXT     (disposal_price - cost_per_unit) * qty
  acquisition_date  DATETIME for holding period classification
  disposal_date     DATETIME
  method            TEXT
```

---

### Sync Audit Log

Tamper-evident record of every wallet sync operation.

```
sync_audit_log
  id          INTEGER  PK
  timestamp   DATETIME
  account_id  TEXT     → accounts.id ON DELETE CASCADE
  address     TEXT     first 8 + last 4 chars (truncated for privacy)
  chain       TEXT
  provider    TEXT
  action      TEXT     balance_sync | tx_sync | health_check
  records_in  INTEGER  records returned by provider
  records_new INTEGER  new records inserted
  error       TEXT?    NULL on success
  duration_ms INTEGER
```

---

### Blockchain Sync State

Tracks the last synced block height per address, enabling incremental syncs.

```
blockchain_sync_state
  wallet_address_id  INTEGER  → wallet_addresses.id ON DELETE CASCADE
  chain              TEXT
  last_block_height  INTEGER
  last_sync_at       DATETIME
  PRIMARY KEY (wallet_address_id, chain)

wallet_sync_state
  address      TEXT  PK  (raw address string)
  chain        TEXT
  last_block   INTEGER
  last_sync_at DATETIME
```

---

## Entity Relationship Diagram

```
categories (1)────────────────────────────────────< (N) accounts
                                                          │
                        ┌─────────────────────────────────┤
                        │                                 │
                        ▼                                 ▼
              wallet_addresses (N)              holdings (N)
                        │                       (account_id, asset)
                        │                                 │
                        ▼                                 │
            blockchain_sync_state (N)                     │
                                                          │
               transactions (N) ──────────────────────────┤
                        │                                 │
                        ├──▶ tax_lots (N)                 │
                        │          │                      │
                        │          └──▶ realized_pnl (N)  │
                        │                                 │
                        └──▶ sync_audit_log (N) ──────────┘
```

---

## Common Pitfalls

### 1. Holdings double-count on synced accounts

**Problem:** `tx buy` always calls `holdings.add_quantity()`. On a synced account, the last sync already set the correct quantity. Recording a buy for cost basis purposes adds on top, inflating the balance.

**Fix:** Use `--cost-basis-only` flag on `tx buy` for synced accounts. This creates the tax lot without modifying holdings.

### 2. Cardano raw token amounts

**Problem:** Blockfrost returns native token quantities as raw integers (no decimal conversion). A NIGHT token with 6 decimals and raw value `100593639033` represents `100593.639033` tokens.

**Fix:** The Blockfrost client fetches `GET /assets/{unit}` metadata for each token to get `decimals`, then divides. If metadata fetch fails (no API key), the token is skipped with a warning rather than stored with the wrong value.

### 3. Historical transactions timestamped to now

**Problem:** `tx buy` without `--date` uses `Utc::now()` as the timestamp. All historical cost basis entries appear as purchased today, making FIFO ordering wrong.

**Fix:** Always pass `--date YYYY-MM-DD` (or ISO 8601) for historical entries. The MCP tool `record_transaction` accepts a `timestamp` parameter for this purpose.

### 4. Wallet remove FK failure

**Problem:** Removing a wallet that has been synced fails with a FOREIGN KEY constraint if dependent records exist in `blockchain_sync_state` or `sync_audit_log`.

**Fix:** `blockchain_sync_state` has `ON DELETE CASCADE`. `sync_audit_log` cascades via `account_id`. The `wallet remove` command explicitly deletes dependent rows before removing the wallet address row.

### 5. Transaction type "send" not recognized

**Problem:** On-chain sync (Etherscan, Blockfrost) imports transactions with type `send` (lowercase). The `tx list` command fails to deserialize these rows because `TransactionType::from_str` doesn't handle `"send"`.

**Fix:** `"send"` is mapped to `TransactionType::TransferOut` in `from_str`. Unknown types are shown as-is rather than crashing.
