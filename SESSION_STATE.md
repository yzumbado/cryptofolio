# Cryptofolio - Session State & Resume Guide

**Last Updated:** March 4, 2026
**Current Version:** v0.3.1 (Released)
**Next Version:** v0.4.0 (In Progress — Phase 4 complete, awaiting manual validation)

---

## 🎯 Current State

### Project Status
- ✅ **v0.3.1 RELEASED** — Production-ready with keychain security, P&L engine
- 🔧 **v0.4.0 IN PROGRESS** — Binance Deep Integration (Phase 4 CLI done, testing remains)
- 🧪 **Test Suite:** 341 tests (203 unit + 138 integration), 100% passing
- 📊 **Coverage:** 95-100% on critical business logic

### Git Status
- **Branch:** claude/nice-hermann
- **Last Commit:** ab5a654 — "test: Add integration tests for Binance history sync (Task #61)"
- **Status:** Clean (all changes committed, not yet pushed to remote)

---

## 📝 What Was Accomplished This Session

### Tasks #56–#61 (v0.4.0 Phase 4, fully implemented & tested)

| Task | File(s) | Description |
|------|---------|-------------|
| #56 ✅ | `src/exchange/binance/client.rs` | 6 new history methods + `get_signed_with_params` helper |
| #57 ✅ | `src/db/sync_state.rs`, `migrations.rs` | `binance_sync_state` table + `SyncStateRepository` |
| #58 ✅ | `src/exchange/binance/import.rs` | `TransactionImporter` for 5 Binance record types |
| #59 ✅ | `src/exchange/binance/sync.rs` | `SyncOrchestrator` with paginated fetching + `SyncReport` |
| #60 ✅ | `src/cli/commands/sync.rs`, `cli/mod.rs`, `main.rs`, `shell/mod.rs` | `sync-history` CLI command |
| #61 ✅ | `tests/binance_sync.rs` | 44 integration tests covering all import paths + sync state |

---

## 🚀 Remaining Tasks (Phase 4)

### Task #61 — Integration tests for history sync
**Status:** ✅ COMPLETE (44 tests, all passing)

Tests in `tests/binance_sync.rs` cover:
- `parse_symbol`, `is_usd_equivalent`, `ms_to_datetime` unit tests
- Buy/sell trade import (transaction, holding, tax lot creation)
- Duplicate detection for all record types
- Dry-run has zero side effects
- Deposit/withdrawal/fiat order/transfer happy paths and edge cases
- `SyncStateRepository` watermarks (update, reset, idempotent create)
- `SyncReport` (totals, errors)
- Mixed import sequence

### Task #62 — Manual testing with real Binance account
**Status:** NOT STARTED (requires user to run against live account)

```bash
# 1. Make sure credentials are set
cryptofolio config set-secret binance.api_key
cryptofolio config set-secret binance.api_secret

# 2. Ensure account exists
cryptofolio account add "Binance" --type exchange --category trading --sync

# 3. First dry-run
cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT,ADAUSDT \
  --full-history \
  --dry-run

# 4. Real import
cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT,ADAUSDT \
  --full-history

# 5. Verify results
cryptofolio tx list --limit 20
cryptofolio pnl summary
```

---

## 🗂️ Key Files Reference

### Phase 4 New Files
```
src/exchange/binance/
├── client.rs       # 6 new history methods (get_my_trades, get_deposit_history, etc.)
├── import.rs       # NEW: TransactionImporter (5 record types)
└── sync.rs         # NEW: SyncOrchestrator + SyncOptions + SyncReport

src/db/
├── sync_state.rs   # NEW: SyncStateRepository + SyncState struct
└── migrations.rs   # MIGRATION_006 added (binance_sync_state table)

src/cli/
├── mod.rs          # SyncHistory command variant added
└── commands/sync.rs # handle_sync_history_command added

src/main.rs         # SyncHistory dispatch arm
src/shell/mod.rs    # SyncHistory arm in run_cli_command
```

### Architecture
```
User: cryptofolio sync-history --account Binance --symbols BTCUSDT

    ↓  handle_sync_history_command

1. SyncOrchestrator::sync()
   - get_or_create sync state (SyncStateRepository)
   - compute start_time per endpoint from watermarks

    ↓

2. BinanceClient  (per endpoint, with pagination)
   - get_my_trades (fromId pagination, 1000/page per symbol)
   - get_deposit_history (offset pagination, 1000/page)
   - get_withdrawal_history (offset pagination, 1000/page)
   - get_fiat_orders (page pagination, 500/page)
   - get_transfer_history (page pagination, 100/page, per transfer type)

    ↓

3. TransactionImporter  (per record)
   - is_duplicate check (external_id in transactions table)
   - insert Transaction
   - update HoldingRepository (add/remove quantity)
   - PnLCalculator (process_acquisition for buys, process_disposal for sells)

    ↓

4. SyncStateRepository
   - update watermarks for next incremental run
```

---

## 🔧 Development Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test
cargo test binance::import  # just importer tests

# Check compilation
cargo check

# Run the new command (help)
cargo run -- sync-history --help

# Dry-run example (requires credentials)
cargo run -- sync-history --account Binance --symbols BTCUSDT --dry-run
```

---

## ⚠️ Known Limitations / Future Work

1. **Symbol parsing** — `parse_symbol()` in `import.rs` uses a hardcoded list
   of known quote assets. Unknown pairs (e.g. custom tokens) will error.
   Could be extended to call `get_all_coins_info()` for authoritative list.

2. **Holdings from withdrawals** — Silently ignores `InsufficientBalance` errors
   when importing withdrawals for records that predate the sync window.
   Holdings will be inconsistent for pre-sync data (expected, not a bug).

3. **BNB fees** — When Binance deducts trading fees in BNB (via BNB fee discount),
   the fee amount is in BNB but the `commission_asset` reflects this correctly.
   No automatic BNB holding adjustment is made for fees.

4. **Transfer sub-wallets** — Binance internal transfers (Spot ↔ Earn) are
   recorded as same-account transactions. The tool doesn't track sub-wallet
   balances separately (by design — Earn is not a separate "account").

---

## 🎯 Success Criteria (Phase 4 Complete)

- ✅ `cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT`
- ✅ Progress shown to user
- ✅ Imports trades, deposits, withdrawals, fiat orders, internal transfers
- ✅ Duplicate detection (re-running is safe)
- ✅ Sync watermarks persisted for incremental updates
- ✅ `--full-history` for complete re-import
- ✅ `--dry-run` for safe preview
- ✅ Fees tracked
- ✅ Integration tests (Task #61) — 44 tests, all passing
- ⬜ Manual validation with real account (Task #62)

---

## 🚦 Quick Start (Next Session)

```bash
cd /Users/yzumbado/projects/cryptofolio/.claude/worktrees/nice-hermann
git status    # should be clean
cargo test    # should pass 341 tests

# Only remaining task: Task #62 — manual validation with real Binance account
# See "Remaining Tasks" section above for the test commands
```

---

*Last updated: March 4, 2026*
*Phase 4 complete — only manual validation with real Binance account remains (Task #62)*
