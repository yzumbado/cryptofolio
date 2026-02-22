# Test 80: Backward Compatibility

**Estimated Time:** 20 minutes
**Platform:** macOS, Linux, Windows
**Phase:** Phase 2 Validation (Backward Compatibility)

---

## Overview

Verify that all Phase 2 (Keychain) features maintain full backward compatibility with v0.2.0 functionality. Ensure existing commands, data, and workflows continue to work without breaking changes.

---

## Prerequisites

- ✅ Binary compiled with Phase 2 features
- ✅ Database initialized
- ✅ Existing v0.2.0 data (if available)
- ✅ Config file exists

---

## Test Steps

### 1. Verify All v0.2.0 Commands Still Work

```bash
# Test each core command from v0.2.0

# Price commands
./target/release/cryptofolio price BTC
./target/release/cryptofolio price BTC ETH
./target/release/cryptofolio market BTC

# Account commands
./target/release/cryptofolio account list
./target/release/cryptofolio account add "Legacy Account" --type wallet --category "Cold Storage"
./target/release/cryptofolio account show "Legacy Account"
./target/release/cryptofolio account update "Legacy Account" --category "Trading"

# Category commands
./target/release/cryptofolio category list
./target/release/cryptofolio category add "Legacy Category"

# Holdings commands
./target/release/cryptofolio holdings add "Legacy Account" BTC 0.5
./target/release/cryptofolio holdings list
./target/release/cryptofolio holdings update "Legacy Account" BTC 0.6
./target/release/cryptofolio holdings remove "Legacy Account" BTC --confirm

# Transaction commands
./target/release/cryptofolio tx buy BTC 0.1 --price 50000 --account "Legacy Account"
./target/release/cryptofolio tx list
./target/release/cryptofolio tx list --account "Legacy Account"

# Portfolio commands
./target/release/cryptofolio portfolio
./target/release/cryptofolio portfolio --account "Legacy Account"

# Config commands (non-keychain)
./target/release/cryptofolio config show
./target/release/cryptofolio config get general.default_currency
./target/release/cryptofolio config set general.default_currency USD
```

**Expected:**
- ✅ All commands execute without errors
- ✅ Same behavior as v0.2.0
- ✅ No warnings about keychain
- ✅ No breaking changes

**Validation:**
- ✅ Command syntax unchanged
- ✅ Output format unchanged
- ✅ Data operations work correctly
- ✅ No new required parameters

---

### 2. TOML Configuration Still Works

```bash
# Verify TOML config can still be used alongside keychain
cat ~/Library/Application\ Support/cryptofolio/config.toml

# Set non-secret value in TOML
./target/release/cryptofolio config set display.decimals 8

# Verify it's in TOML
grep decimals ~/Library/Application\ Support/cryptofolio/config.toml

# Verify config show still works
./target/release/cryptofolio config show
```

**Expected:**
- ✅ TOML file still readable
- ✅ Non-secret values stored in TOML
- ✅ Config show displays TOML values
- ✅ No forced migration to keychain

**Validation:**
- ✅ TOML remains primary config format
- ✅ Keychain is opt-in, not mandatory
- ✅ Mixed storage (TOML + Keychain) works

---

### 3. Database Schema Compatibility

```bash
# Check that old tables still exist and work
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.tables
EOF
```

**Expected Tables:**
```
_migrations
accounts
categories
holdings
keychain_keys    ← NEW (Phase 2)
transactions
```

**Verify Old Tables Unchanged:**
```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.schema accounts
.schema categories
.schema holdings
.schema transactions
EOF
```

**Expected:**
- ✅ All v0.2.0 tables present
- ✅ Old table schemas unchanged
- ✅ New keychain_keys table added (doesn't affect old tables)
- ✅ Foreign keys still valid

---

### 4. Existing Data Not Affected

```bash
# If you have existing data, verify it's intact

# Check account count
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite \
  "SELECT COUNT(*) FROM accounts"

# Check transaction count
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite \
  "SELECT COUNT(*) FROM transactions"

# List all accounts (should include pre-Phase 2 accounts)
./target/release/cryptofolio account list
```

**Expected:**
- ✅ All existing accounts present
- ✅ All existing transactions intact
- ✅ No data loss
- ✅ No data corruption

**Validation:**
- ✅ Counts match pre-Phase 2 state
- ✅ Data displays correctly
- ✅ Can still operate on old data

---

### 5. Mixed Storage: TOML and Keychain Coexist

```bash
# Set a regular config value in TOML
./target/release/cryptofolio config set test.non_secret "public_value"

# Set a secret in keychain
echo "secret_value" | ./target/release/cryptofolio config set-secret test.secret_key

# Verify both exist
./target/release/cryptofolio config show
./target/release/cryptofolio config keychain-status

# Check TOML has non-secret
grep non_secret ~/Library/Application\ Support/cryptofolio/config.toml

# Check TOML doesn't have secret
grep secret_key ~/Library/Application\ Support/cryptofolio/config.toml || echo "Not in TOML (correct)"
```

**Expected:**
- ✅ Non-secret in TOML: `test.non_secret = "public_value"`
- ✅ Secret in keychain only (not in TOML)
- ✅ config show displays both sources
- ✅ No conflicts between storage types

**Validation:**
- ✅ Dual storage model works
- ✅ Secrets automatically go to keychain (on macOS)
- ✅ Regular config stays in TOML
- ✅ Seamless integration

---

### 6. Export Functionality Unchanged

```bash
# Test v0.2.0 export features

# CSV export
./target/release/cryptofolio tx export --format csv > /tmp/test_export.csv

# Verify CSV is valid
head -5 /tmp/test_export.csv

# JSON export (if available)
./target/release/cryptofolio tx list --json > /tmp/test_export.json

# Verify JSON is valid
cat /tmp/test_export.json | jq '.' > /dev/null && echo "Valid JSON"
```

**Expected:**
- ✅ Export commands work
- ✅ CSV format unchanged
- ✅ JSON format valid
- ✅ All data exported correctly

**Cleanup:**
```bash
rm /tmp/test_export.csv /tmp/test_export.json
```

---

### 7. Binance Sync Still Works (if configured)

```bash
# Only run if you have Binance credentials configured

# Check if Binance credentials exist (keychain or TOML)
./target/release/cryptofolio config show | grep binance || echo "No Binance config"

# If credentials exist, test sync
# ./target/release/cryptofolio sync --account Binance

# Note: This test requires real API credentials
# Skip if not available
```

**Expected (if configured):**
- ✅ Sync command works
- ✅ Retrieves credentials from keychain (if migrated) or TOML
- ✅ API calls succeed
- ✅ Holdings updated

**Validation:**
- ✅ Exchange integration unaffected
- ✅ API credential retrieval works from both sources

---

### 8. Help Text Unchanged (Existing Commands)

```bash
# Verify help text for v0.2.0 commands hasn't changed

./target/release/cryptofolio --help
./target/release/cryptofolio account --help
./target/release/cryptofolio tx --help
./target/release/cryptofolio portfolio --help
./target/release/cryptofolio config --help
```

**Expected:**
- ✅ All help text displays
- ✅ Command descriptions unchanged
- ✅ New keychain commands visible in config --help
- ✅ No breaking changes to existing command syntax

**Validation:**
- ✅ Documentation consistent
- ✅ Examples still valid
- ✅ New features additive, not destructive

---

### 9. Error Messages Unchanged

```bash
# Test that error messages from v0.2.0 commands are unchanged

# Non-existent account
./target/release/cryptofolio account show "NonExistent" 2>&1

# Invalid transaction
./target/release/cryptofolio tx buy INVALID_SYMBOL 1 --price 100 --account "Legacy Account" 2>&1

# Missing required parameter
./target/release/cryptofolio tx buy 2>&1
```

**Expected:**
- ✅ Error messages clear and helpful
- ✅ No new errors for old commands
- ✅ Error format consistent with v0.2.0

**Validation:**
- ✅ Error handling unchanged
- ✅ User experience consistent
- ✅ No regression in error reporting

---

### 10. Performance - No Degradation

```bash
# Test that Phase 2 additions don't slow down existing commands

# Time a command that doesn't use keychain
time ./target/release/cryptofolio account list

# Time a database query
time ./target/release/cryptofolio tx list --limit 100

# Time a config operation
time ./target/release/cryptofolio config show
```

**Expected:**
- ✅ account list: <100ms
- ✅ tx list: <200ms (depends on data size)
- ✅ config show: <100ms

**Validation:**
- ✅ No performance regression
- ✅ Keychain operations don't affect non-keychain commands
- ✅ Database queries remain fast

---

### 11. Config File Format Unchanged

```bash
# Verify config.toml format is still valid TOML

cat ~/Library/Application\ Support/cryptofolio/config.toml

# Try to parse it externally (if toml tools available)
# python3 -c "import toml; toml.load(open('config.toml'))" 2>&1
```

**Expected:**
- ✅ Valid TOML syntax
- ✅ All sections properly formatted
- ✅ No keychain-specific syntax in TOML
- ✅ Backward compatible with v0.2.0

**Validation:**
- ✅ Old config files work with new binary
- ✅ New config files compatible with format
- ✅ No forced schema changes

---

### 12. CLI Exit Codes Unchanged

```bash
# Verify exit codes for common scenarios

# Success
./target/release/cryptofolio account list
echo "Exit code: $?"  # Should be 0

# Command error (invalid command)
./target/release/cryptofolio invalid_command 2>&1
echo "Exit code: $?"  # Should be non-zero

# Missing required argument
./target/release/cryptofolio account add 2>&1
echo "Exit code: $?"  # Should be non-zero
```

**Expected:**
- ✅ Success: exit code 0
- ✅ Errors: exit code non-zero
- ✅ Consistent with v0.2.0 behavior

**Validation:**
- ✅ Scripts relying on exit codes still work
- ✅ No changes to error handling contract

---

### 13. JSON Output Format Unchanged (Existing Commands)

```bash
# Verify JSON output for v0.2.0 commands unchanged

./target/release/cryptofolio account list --json | jq '.'
./target/release/cryptofolio tx list --json | jq '.transactions | length'
./target/release/cryptofolio portfolio --json | jq '.accounts | length'
```

**Expected:**
- ✅ Valid JSON
- ✅ Same structure as v0.2.0
- ✅ No new required fields
- ✅ Parseable by existing tools

**Validation:**
- ✅ API contract unchanged
- ✅ Integrations with other tools still work
- ✅ Backward compatible JSON schema

---

### 14. Migration Doesn't Break Non-Migrated Secrets

```bash
# Set a secret in TOML (old way)
./target/release/cryptofolio config set legacy.api_key "old-style-key"

# Verify it's in TOML
grep legacy.api_key ~/Library/Application\ Support/cryptofolio/config.toml

# Set another secret in keychain (new way)
echo "new-style-key" | ./target/release/cryptofolio config set-secret modern.api_key

# Verify both accessible
./target/release/cryptofolio config show | grep legacy
./target/release/cryptofolio config keychain-status | grep modern

# Both should coexist
```

**Expected:**
- ✅ Legacy secret in TOML works
- ✅ Modern secret in keychain works
- ✅ No forced migration
- ✅ User chooses when to migrate

**Validation:**
- ✅ Gradual migration possible
- ✅ No breaking changes for users on older workflows
- ✅ Opt-in security features

---

### 15. Platform Independence Maintained

```bash
# Verify non-macOS behavior (simulate)

# On macOS, keychain commands work
./target/release/cryptofolio config keychain-status

# On Linux/Windows, keychain commands should:
# - Either: Not be available (compile-time)
# - Or: Show clear message about platform support

# Test conditional compilation worked
./target/release/cryptofolio --help | grep keychain || echo "Keychain features (macOS only)"
```

**Expected:**
- ✅ macOS: Keychain features available
- ✅ Linux/Windows: Graceful fallback or feature unavailable
- ✅ Core features work on all platforms
- ✅ Platform-specific features clearly marked

**Validation:**
- ✅ No platform regressions
- ✅ Conditional compilation working
- ✅ Clear messaging for platform-specific features

---

### 16. Cleanup Test Data

```bash
# Remove test accounts
./target/release/cryptofolio account delete "Legacy Account" --confirm

# Remove test categories
./target/release/cryptofolio category delete "Legacy Category" --confirm

# Remove test config values
# Manually edit TOML if needed to remove test.* and legacy.* sections

# Remove test keychain entries
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test.secret_key" 2>/dev/null
security delete-generic-password -s "com.cryptofolio.api-keys" -a "modern.api_key" 2>/dev/null

# Clean database
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
DELETE FROM keychain_keys WHERE key_name LIKE 'test.%' OR key_name LIKE 'modern.%';
EOF

# Verify cleanup
./target/release/cryptofolio account list
./target/release/cryptofolio config keychain-status
```

**Validation:**
- ✅ All test data removed
- ✅ System returned to pre-test state

---

## Validation Checklist

### Command Compatibility
- [ ] All v0.2.0 commands work unchanged
- [ ] Command syntax unchanged
- [ ] Output format unchanged
- [ ] No new required parameters
- [ ] Help text consistent

### Data Compatibility
- [ ] Existing data intact
- [ ] No data loss
- [ ] No data corruption
- [ ] Old database schema preserved
- [ ] New tables don't affect old tables

### Configuration
- [ ] TOML config still works
- [ ] Mixed storage (TOML + Keychain) works
- [ ] No forced migration
- [ ] Config format unchanged
- [ ] Backward compatible TOML

### Functionality
- [ ] Export commands work
- [ ] Exchange sync works (if configured)
- [ ] Portfolio views work
- [ ] Transaction operations work
- [ ] Account management works

### Performance
- [ ] No performance regression
- [ ] Response times unchanged
- [ ] Database queries fast
- [ ] No slowdown in non-keychain commands

### Error Handling
- [ ] Error messages unchanged
- [ ] Exit codes consistent
- [ ] Error format unchanged
- [ ] User experience consistent

### Integration
- [ ] JSON output format unchanged
- [ ] API contract maintained
- [ ] External tools still work
- [ ] Scripts using CLI still work

### Platform Support
- [ ] All platforms still supported
- [ ] Platform-specific features conditional
- [ ] Graceful fallback on unsupported platforms
- [ ] Clear messaging

---

## Common Issues

### Issue 1: Old Commands Broken
**Symptoms:** v0.2.0 commands fail with new binary
**Solution:** Check for breaking changes, review command parsing
**Action:** Rollback if necessary, investigate regression

### Issue 2: Data Inaccessible
**Symptoms:** Can't access old accounts/transactions
**Solution:** Check database migrations, verify schema
**Action:**
```bash
sqlite3 database.sqlite ".schema"
sqlite3 database.sqlite "SELECT * FROM _migrations"
```

### Issue 3: TOML Config Ignored
**Symptoms:** Settings in TOML not applied
**Solution:** Check config loading priority
**Action:** Verify settings.rs loads TOML before keychain

### Issue 4: Performance Degradation
**Symptoms:** Commands slower than v0.2.0
**Solution:** Profile queries, check for unnecessary keychain access
**Action:** Review code for performance regressions

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**Platform:** macOS / Linux / Windows
**Previous Version:** v0.2.0
**Current Version:** v0.3.0 (Phase 2)

**Tests Passed:**
- [ ] All v0.2.0 commands work
- [ ] TOML configuration works
- [ ] Database compatibility
- [ ] Existing data intact
- [ ] Mixed storage works
- [ ] Export functionality
- [ ] Exchange sync (if applicable)
- [ ] Help text consistent
- [ ] Error messages unchanged
- [ ] Performance maintained
- [ ] Config format unchanged
- [ ] Exit codes consistent
- [ ] JSON output unchanged
- [ ] No forced migration
- [ ] Platform independence

**Regression Issues Found:**
```
_____________________________________________
_____________________________________________
_____________________________________________
```

**Migration Notes:**
```
_____________________________________________
_____________________________________________
_____________________________________________
```

---

## Upgrade Path Verification

**From v0.2.0 to v0.3.0 (Phase 2):**

1. ✅ Install new binary (replace old one)
2. ✅ Run any command - database auto-migrates
3. ✅ All existing features work immediately
4. ✅ Keychain features available (opt-in)
5. ✅ Run `config migrate-to-keychain` when ready
6. ✅ No data loss, no downtime

**Rollback Path:**
1. ✅ Stop using new binary
2. ✅ Replace with v0.2.0 binary
3. ⚠️ Keychain secrets inaccessible (restore from backup if needed)
4. ✅ All other features work

---

**Validation Complete:** This test confirms Phase 2 maintains full backward compatibility with v0.2.0.

---

**End of Critical Test Suite**

**Next Steps:**
- Execute all 8 critical tests (00, 01, 10, 11, 60, 61, 62, 80)
- Document results
- Fix any issues found
- Commit Phase 2 code
