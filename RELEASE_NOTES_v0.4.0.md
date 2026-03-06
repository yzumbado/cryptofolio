# Cryptofolio v0.4.0 — Binance Deep Integration

**Release Date:** March 5, 2026

---

## 🎉 What's New

### 📊 Complete Binance Transaction History Sync

v0.4.0 delivers full Binance transaction history import — every trade, deposit,
withdrawal, fiat order, and internal transfer automatically flows into your
Cryptofolio portfolio with accurate P&L tracking.

---

## ✨ New Command: `sync-history`

```bash
# Sync all history for specific trading pairs
cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT,ADAUSDT

# Preview without writing anything
cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run

# Re-import everything from the beginning
cryptofolio sync-history --account Binance --symbols BTCUSDT --full-history

# Sync from a specific date
cryptofolio sync-history --account Binance --symbols BTCUSDT --from 2024-01-01

# Skip specific record types
cryptofolio sync-history --account Binance --symbols BTCUSDT --no-transfers --no-fiat
```

### What gets imported

| Record Type | Binance Source | Cryptofolio Transaction |
|---|---|---|
| Spot trades | `/api/v3/myTrades` | Buy / Sell |
| Crypto deposits | `/sapi/v1/capital/deposit/hisrec` | Transfer In |
| Crypto withdrawals | `/sapi/v1/capital/withdraw/history` | Transfer Out |
| Fiat on-ramp orders | `/sapi/v1/fiat/orders` | Buy |
| Internal transfers (Spot ↔ Earn) | `/sapi/v1/asset/transfer` | Transfer Internal |

---

## 🔄 Incremental Sync (Smart Watermarks)

Run `sync-history` as often as you like — it only fetches what's new:

```
First run:  Imports 3,247 trades + 12 deposits + 8 withdrawals
Second run: Imports 3 new trades (from last watermark)
```

Watermarks are stored per account in the `binance_sync_state` table. Use
`--full-history` to clear them and re-import everything.

---

## 💰 Automatic P&L Tracking

Every imported trade automatically creates a tax lot and calculates realized P&L
using your configured cost basis method (FIFO):

```bash
# After sync-history, P&L is ready:
cryptofolio pnl summary --account Binance

=== P&L Summary (Binance) ===
Realized P&L:   +$45,230.50
Unrealized P&L: +$12,450.00
──────────────────────────────
Net P&L:        +$57,680.50
```

---

## 🛡️ Duplicate-Safe

Re-running `sync-history` is always safe. Every imported record carries a unique
`external_id` (e.g. `binance-trade-28457`) that prevents duplicates, even across
`--full-history` re-runs.

---

## 🧪 Comprehensive Tests

44 new integration tests in `tests/binance_sync.rs` cover every import path,
duplicate detection, dry-run side effects, watermark updates, and edge cases.

**Test suite total: 341 tests — 100% passing.**

---

## 🚀 Quick Start

### Prerequisites

```bash
# Store API credentials securely in macOS Keychain
cryptofolio config set-secret binance.api_key
cryptofolio config set-secret binance.api_secret

# Create a Binance exchange account (if not already done)
cryptofolio account add "Binance" --type exchange --category trading --sync
```

### First Sync

```bash
# Dry run first — see what would be imported
cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT,ADAUSDT,BNBUSDT \
  --full-history \
  --dry-run

# Then run for real
cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT,ADAUSDT,BNBUSDT \
  --full-history
```

### Verify Results

```bash
# List recent transactions
cryptofolio tx list --account Binance --limit 20

# Check portfolio holdings
cryptofolio portfolio --account Binance

# View realized P&L
cryptofolio pnl summary --account Binance
```

---

## 📝 Technical Notes

### Symbol Parsing
Trading pairs are parsed by trying known quote assets longest-first:
`USDT`, `BUSD`, `USDC`, `TUSD`, `USDP`, `DAI`, `BTC`, `ETH`, `BNB`.
Unknown pairs will return an error — open an issue to add support.

### Fee Tracking
Fees are recorded as `fee` + `fee_asset` on each transaction. BNB fee discounts
are captured when `commission_asset = BNB`. No automatic BNB holding deduction
is made for fees (fee amounts are typically tiny).

### Pre-Sync Holdings Gap
If you import withdrawal history that predates your first deposit/buy in
Cryptofolio, the holding balance reduction is silently skipped rather than
erroring. Holdings will be accurate from the point you first have recorded
acquisitions.

### Transfer Sub-Wallets
Binance Spot ↔ Earn transfers are recorded as same-account internal transfers.
Earn wallet balances are not tracked separately (they appear in the main Binance
account balance sync).

---

## 🔧 Upgrade Notes

v0.4.0 adds a new database migration (MIGRATION_006) for the `binance_sync_state`
table. The migration is applied automatically on first run — no manual steps needed.

```
Previous: v0.3.1 — 5 DB migrations
Now:      v0.4.0 — 6 DB migrations (adds binance_sync_state)
```

---

## What's Coming Next

- CSV / Excel report export
- CoinGecko price history import
- Per-year P&L breakdown for tax reporting
- Multi-exchange unified portfolio view

---

*Cryptofolio v0.4.0 — Built with Rust 🦀*
