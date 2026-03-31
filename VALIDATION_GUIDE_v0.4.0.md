# Cryptofolio v0.4.0 — Manual Validation Guide

**Purpose:** Step-by-step validation of the new `sync-history` command against a real Binance account.

**Prerequisites:** API credentials ready, Binance account exists, Xcode Command Line Tools installed.

---

## 0. Build and Verify

```bash
# Navigate to project root
cd /Users/yzumbado/projects/cryptofolio

# Build debug binary (unsigned - works without code signing!)
cargo build

# Verify version
./target/debug/cryptofolio --version
# cryptofolio 0.4.0

# Run full test suite
cargo test
# test result: ok. 341 passed; 0 failed
```

> **✅ Good News:** As of v0.4.0, Cryptofolio uses the macOS `security` command for keychain
> access, which works perfectly without code signing! The unsigned debug binary has full
> keychain functionality using the system's built-in security tools.

---

## 1. Credential Setup (Secure Keychain Storage)

```bash
# Store API key securely in macOS Keychain
./target/debug/cryptofolio config set-secret binance.api_key
# Enter secret (hidden): <paste your Binance API key>
# ✓ Secret stored in macOS Keychain (Standard)

./target/debug/cryptofolio config set-secret binance.api_secret
# Enter secret (hidden): <paste your Binance API secret>
# ✓ Secret stored in macOS Keychain (Standard)

# Verify credentials are stored
./target/debug/cryptofolio config keychain-status
# Should show: binance.api_key ✓ Active, binance.api_secret ✓ Active
```

**Alternative: Environment Variables (for CI/automation)**

```bash
# If you prefer environment variables instead
export BINANCE_API_KEY="your-binance-api-key"
export BINANCE_API_SECRET="your-binance-api-secret"
```

> **✅ Secure:** Credentials stored in macOS Keychain are encrypted and require your Mac
> password or Touch ID to access. The app uses the system `security` command, so no code
> signing is required!

---

## 2. Account Setup

```bash
# Create Binance exchange account (skip if already exists)
./target/debug/cryptofolio account add "Binance" --type exchange --category trading --sync

# Verify account exists
./target/debug/cryptofolio account list
# Should show Binance in the list

# Sync current balances to confirm API connectivity
./target/debug/cryptofolio sync --account Binance
# ✓ Synced N assets from 'Binance'
```

**Expected:** At least one holding shown, no authentication errors.

---

## 3. Dry-Run (Safe Preview — No Writes)

This is always the right first step.

```bash
./target/debug/cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT \
  --full-history \
  --dry-run
```

**Expected output:**
```
[DRY RUN] Would sync-history for account 'Binance'
Fetching BTCUSDT trades...
Fetching ETHUSDT trades...
Fetching deposit history...
Fetching withdrawal history...
Fetching fiat orders...
Fetching internal transfers...

=== Dry-Run Report ===
Trades:      N would be created, M skipped
Deposits:    N would be created
Withdrawals: N would be created
Fiat orders: N would be created
Transfers:   N would be created

No changes written (dry run)
```

**Checks:**
- [ ] No crash or authentication error
- [ ] Numbers seem plausible for your account history
- [ ] No unhandled errors shown

---

## 4. Full History Import

```bash
./target/debug/cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT \
  --full-history
```

**Expected output:**
```
Syncing Binance transaction history...
  Trades (BTCUSDT)...     ✓ N created, M skipped
  Trades (ETHUSDT)...     ✓ N created, M skipped
  Deposits...             ✓ N created
  Withdrawals...          ✓ N created
  Fiat orders...          ✓ N created
  Transfers...            ✓ N created

=== Sync Report ===
Trades:      N created, M skipped
Deposits:    N created
Withdrawals: N created
Fiat orders: N created
Transfers:   N created
Total:       N transactions imported
```

**Checks:**
- [ ] No panic or unhandled error
- [ ] Created count > 0 (assuming you have history)
- [ ] Command exits cleanly

---

## 5. Verify Transactions Were Created

```bash
# List recent transactions
./target/debug/cryptofolio tx list --limit 20

# List only buy transactions
./target/debug/cryptofolio tx list --limit 20 --type buy

# Verify an imported trade shows external ID
./target/debug/cryptofolio tx list --limit 5 --json | jq '.[].external_id'
# Should show: "binance-trade-12345", "binance-deposit-abc", etc.
```

**Checks:**
- [ ] Transactions shown with correct dates/amounts
- [ ] `external_id` is set (e.g. `binance-trade-28457`)
- [ ] Transaction types are correct (Buy, Sell, Transfer In, Transfer Out)

---

## 6. Verify Holdings Were Updated

```bash
./target/debug/cryptofolio holdings list
```

**Checks:**
- [ ] BTC and ETH holdings exist (if you traded those pairs)
- [ ] Quantities are non-negative
- [ ] Holdings roughly match your known Binance balance

> **Note:** Holdings from `sync-history` may not exactly match the current Binance
> balance because the `sync` command (balance snapshot) and `sync-history` (trade
> history) use different data sources. The balance sync is the authoritative source
> for current holdings.

---

## 7. Verify P&L Tracking

```bash
# Check if P&L was calculated for trades
./target/debug/cryptofolio pnl summary

# Detailed realized gains
./target/debug/cryptofolio pnl realized --limit 10

# Check tax lots created
./target/debug/cryptofolio pnl unrealized
```

**Checks:**
- [ ] `pnl summary` shows non-zero realized P&L (if you have sold any crypto)
- [ ] Realized P&L entries match your known sell transactions
- [ ] Unrealized P&L shows for remaining holdings

---

## 8. Duplicate Detection (Idempotency Test)

Run the same command again immediately after the first import:

```bash
./target/debug/cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT \
  --full-history
```

**Expected:**
```
Trades:      0 created, N skipped   ← all existing, none created
Deposits:    0 created, N skipped
Withdrawals: 0 created, N skipped
Fiat orders: 0 created, N skipped
Transfers:   0 created, N skipped
Total:       0 transactions imported
```

**Checks:**
- [ ] Zero new transactions created
- [ ] Total transaction count is unchanged (verify with `./target/debug/cryptofolio tx list | wc -l`)
- [ ] No errors

---

## 9. Incremental Sync Test

After the full import, run without `--full-history` to test watermarks:

```bash
./target/debug/cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT
```

**Expected:**
```
Syncing from last watermark...
Trades (BTCUSDT): 0 created, N skipped (all already imported)
...
```

**Checks:**
- [ ] Command completes quickly (watermarks correctly skip old data)
- [ ] 0 new transactions created (no new Binance activity since the full import)
- [ ] No authentication errors

---

## 10. Partial Symbol Test

Test importing for a single pair:

```bash
# Only sync BNBUSDT trades
./target/debug/cryptofolio sync-history --account Binance --symbols BNBUSDT
```

**Checks:**
- [ ] Only BNBUSDT trades are imported (not BTCUSDT/ETHUSDT)
- [ ] Deposits and withdrawals are still fetched (they're not symbol-filtered)

---

## 11. Date Range Test

```bash
./target/debug/cryptofolio sync-history --account Binance \
  --symbols BTCUSDT \
  --from 2024-01-01 \
  --dry-run
```

**Checks:**
- [ ] Only shows records from 2024 onwards in dry-run output
- [ ] Records before 2024 are not included

---

## 12. Skip Flags Test

```bash
# Only import trades, skip everything else
./target/debug/cryptofolio sync-history --account Binance \
  --symbols BTCUSDT \
  --no-deposits \
  --no-withdrawals \
  --no-fiat \
  --no-transfers \
  --dry-run
```

**Expected output:**
```
[DRY RUN]
Trades (BTCUSDT): N would be created
(deposits skipped)
(withdrawals skipped)
(fiat orders skipped)
(transfers skipped)
```

**Checks:**
- [ ] Only trade records shown in output
- [ ] No deposit/withdrawal/fiat/transfer counts shown

---

## 13. Edge Case: API Error Handling

Temporarily corrupt one API credential to test graceful failure:

```bash
# Set a bad API key temporarily
./target/debug/cryptofolio config set-secret binance.api_key
# Enter: BADKEY123

# Try to sync
./target/debug/cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run
```

**Expected:**
```
Error: Authentication failed (invalid API key)
```
or
```
Error fetching BTCUSDT trades: API error 401
```

**Checks:**
- [ ] Clear error message (not a panic/stack trace)
- [ ] Command exits with non-zero status code

```bash
# Restore correct API key
./target/debug/cryptofolio config set-secret binance.api_key
```

---

## 14. Post-Import Verification

After all tests:

```bash
# Check final transaction count
./target/debug/cryptofolio tx list --json | jq 'length'

# Check holdings summary
./target/debug/cryptofolio holdings list

# Final P&L
./target/debug/cryptofolio pnl summary

# Spot check a specific transaction
./target/debug/cryptofolio tx list --limit 5 --json | jq '.[0]'
# Should show: id, external_id, type, asset, quantity, price, date, notes
```

---

## Checklist Summary

| Test | Status |
|------|--------|
| Build, sign, and version check | ⬜ |
| Credential setup (Keychain) | ⬜ |
| Account + balance sync | ⬜ |
| Dry-run (no writes) | ⬜ |
| Full history import | ⬜ |
| Transactions created correctly | ⬜ |
| Holdings updated | ⬜ |
| P&L calculated | ⬜ |
| Duplicate detection (zero on re-run) | ⬜ |
| Incremental sync (watermarks work) | ⬜ |
| Partial symbol sync | ⬜ |
| Date range filter | ⬜ |
| Skip flags work | ⬜ |
| API error handled gracefully | ⬜ |
| Post-import state verified | ⬜ |

---

## Known Limitations (Not Bugs)

1. **Holdings gap for pre-sync withdrawals** — If you import withdrawal history
   that predates your first recorded acquisition, the holding reduction is silently
   skipped. Holdings will be consistent from the point of first recorded acquisition.

2. **BNB fees** — Trading fees paid in BNB are recorded in the fee fields but
   no BNB holding adjustment is made automatically.

3. **Symbol parsing** — Only known quote assets are supported (USDT, BUSD, USDC,
   TUSD, USDP, DAI, BTC, ETH, BNB). If you traded an unsupported pair, the
   import will error on that specific trade. Open an issue to add support.

4. **Spot ↔ Earn transfers** — Recorded as same-account internal transfers.
   Earn sub-wallet balance is not tracked separately.

5. **Testnet** — `sync-history` works with testnet API too. Use
   `./target/debug/cryptofolio config use-testnet` first.

6. **Binary signing** — The release binary must be re-signed with `./sign.sh` after
   every `cargo build --release`. Ad-hoc signing is machine-specific and will not
   work if the binary is copied to another Mac.

---

## Reporting Issues

If a test fails or produces unexpected results, capture:

```bash
# Full command output
./target/debug/cryptofolio sync-history --account Binance --symbols BTCUSDT 2>&1 | tee /Users/yzumbado/projects/cryptofolio/sync-debug.log

# Transaction count before and after
./target/debug/cryptofolio tx list --json | jq 'length'

# Check binary is signed
codesign -dvvv ./target/debug/cryptofolio 2>&1 | grep -E "Signature|Identifier"
```

Then open an issue with:
- The command you ran
- The output / error message
- Your Binance account type (standard / sub-account / VIP)
- Whether testnet or mainnet
- Output of the `codesign` check above
