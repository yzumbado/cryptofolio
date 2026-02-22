# Test 60: Keychain Migration Workflow

**Estimated Time:** 15 minutes
**Platform:** macOS (keychain features)
**Phase:** Phase 2 (Keychain Security)

---

## Overview

Verify the complete TOML → Keychain migration workflow, including secret discovery, backup creation, migration execution, and rollback capabilities.

---

## Prerequisites

- ✅ macOS system with keychain access
- ✅ Binary compiled with Phase 2 features
- ✅ Database initialized
- ✅ Config basics working (Test 10 passed)
- ✅ Keychain integration working (Test 11 passed)

---

## Test Steps

### 1. Setup - Create Test Secrets in TOML

```bash
# Create test secrets in config.toml for migration
./target/release/cryptofolio config set test_exchange.api_key "test-key-123"
./target/release/cryptofolio config set test_exchange.api_secret "test-secret-456"
./target/release/cryptofolio config set another_service.token "token-789"

# Verify secrets in TOML
cat ~/Library/Application\ Support/cryptofolio/config.toml | grep -E "api_key|api_secret|token"
```

**Expected in config.toml:**
```toml
[test_exchange]
api_key = "test-key-123"
api_secret = "test-secret-456"

[another_service]
token = "token-789"
```

**Validation:**
- ✅ Secrets visible in plaintext TOML
- ✅ 3 test secrets created
- ✅ Ready for migration

---

### 2. Check Pre-Migration Keychain Status

```bash
# Verify keychain is empty (or has known state)
./target/release/cryptofolio config keychain-status
```

**Expected Output:**
```
No secrets currently tracked in keychain.
```
*Or list of existing secrets not including test_exchange.* or another_service.*

**Validation:**
- ✅ Migration starting from known state
- ✅ No conflicts with existing secrets

---

### 3. Run Migration Discovery (Dry Run)

```bash
# The migration command will show what it finds before prompting
# We'll run it and cancel to see discovery
./target/release/cryptofolio config migrate-to-keychain
# Press 'n' when prompted to cancel
```

**Expected Output:**
```
🔍 Scanning config.toml for secrets...

Found 3 secret(s) to migrate:
  • test_exchange.api_key
  • test_exchange.api_secret
  • another_service.token

🔐 Security Level Selection:
  [1] Standard (unlocked with Mac)
      - Accessible when Mac is unlocked
      - Suitable for automation, background jobs

  [2] Touch ID Protected (require Touch ID or password) ← Recommended
      - Requires Touch ID or password for access
      - Balance of security and usability
      - Note: Full Touch ID prompts coming in v0.3.1

  [3] Touch ID Only (biometric only, no fallback)
      - Requires Touch ID (no password fallback)
      - Maximum security
      - Note: Full Touch ID prompts coming in v0.3.1

Choose security level [1-3] (default: 2):
```

**At this point, press Ctrl+C or type 'n' to cancel and verify no changes made.**

**Validation:**
- ✅ Discovery finds all 3 secrets
- ✅ Secret names displayed correctly
- ✅ Security level options explained
- ✅ Recommended option highlighted
- ✅ Can cancel without changes

---

### 4. Verify No Changes After Cancellation

```bash
# Check TOML still has secrets
cat ~/Library/Application\ Support/cryptofolio/config.toml | grep -E "api_key|api_secret|token"

# Check keychain still empty
./target/release/cryptofolio config keychain-status

# Check no backup created
ls ~/Library/Application\ Support/cryptofolio/*.backup 2>&1
```

**Expected Results:**
- ✅ TOML unchanged (secrets still present)
- ✅ Keychain empty (no migration occurred)
- ✅ No backup file created
- ✅ No side effects from cancelled migration

---

### 5. Execute Migration with Standard Security Level

```bash
# Run migration and select Standard (option 1)
./target/release/cryptofolio config migrate-to-keychain

# When prompted for security level, enter: 1
# When asked to confirm, enter: y
```

**Expected Workflow:**
```
🔍 Scanning config.toml for secrets...

Found 3 secret(s) to migrate:
  • test_exchange.api_key
  • test_exchange.api_secret
  • another_service.token

🔐 Security Level Selection:
  [1] Standard (unlocked with Mac)
  [2] Touch ID Protected (require Touch ID or password) ← Recommended
  [3] Touch ID Only (biometric only, no fallback)

Choose security level [1-3] (default: 2): 1

You selected: Standard

📋 Migration Plan:
  • Create backup: config.toml.backup.YYYYMMDD-HHMMSS
  • Migrate 3 secrets to macOS Keychain (Standard security)
  • Clear secrets from config.toml
  • Update database metadata

Proceed with migration? [y/N]: y

✓ Created backup: config.toml.backup.20260221-235959
✓ Migrated 'test_exchange.api_key' to keychain (Standard)
✓ Migrated 'test_exchange.api_secret' to keychain (Standard)
✓ Migrated 'another_service.token' to keychain (Standard)
✓ Cleared secrets from config.toml
✓ Updated database metadata

🎉 Migration complete!

  3 secret(s) migrated to macOS Keychain
  Backup saved: config.toml.backup.20260221-235959
  Security level: Standard

Verify with: cryptofolio config keychain-status
```

**Validation:**
- ✅ All steps executed in order
- ✅ Progress shown for each secret
- ✅ Backup created with timestamp
- ✅ Success message displayed

---

### 6. Verify Backup Created

```bash
# Check backup exists
ls -lh ~/Library/Application\ Support/cryptofolio/config.toml.backup.*

# Compare backup to see it has original secrets
grep -E "api_key|api_secret|token" ~/Library/Application\ Support/cryptofolio/config.toml.backup.*
```

**Expected:**
- ✅ Backup file exists
- ✅ Backup has timestamp in filename
- ✅ Backup contains original secrets
- ✅ Backup is valid TOML

---

### 7. Verify Secrets Removed from TOML

```bash
# Check current config.toml
cat ~/Library/Application\ Support/cryptofolio/config.toml

# Specifically look for secrets
grep -E "api_key|api_secret|token" ~/Library/Application\ Support/cryptofolio/config.toml || echo "No secrets found (expected)"
```

**Expected Result:**
```
No secrets found (expected)
```

**In config.toml:**
```toml
# Secrets should be removed
# Only non-secret config remains

[test_exchange]
# api_key removed
# api_secret removed

[another_service]
# token removed
```

**Validation:**
- ✅ Secrets cleared from TOML
- ✅ Section structure may remain (but empty)
- ✅ No plaintext secrets visible

---

### 8. Verify Secrets in Keychain

```bash
# Check keychain status
./target/release/cryptofolio config keychain-status
```

**Expected Output:**
```
┌────────────────────────┬──────────────────┬────────────┐
│ Key                    │ Security Level   │ Status     │
├────────────────────────┼──────────────────┼────────────┤
│ another_service.token  │ Standard         │ ✓ Active   │
│ test_exchange.api_key  │ Standard         │ ✓ Active   │
│ test_exchange.api_secret│ Standard         │ ✓ Active   │
└────────────────────────┴──────────────────┴────────────┘

Total: 3 secret(s) in keychain
```

**Validation:**
- ✅ All 3 secrets present
- ✅ Security level: Standard
- ✅ Status: Active
- ✅ Sorted alphabetically

---

### 9. Verify Secrets in macOS Keychain

```bash
# Use macOS security tool to verify each secret
security find-generic-password -s "com.cryptofolio.api-keys" -a "test_exchange.api_key" -w
security find-generic-password -s "com.cryptofolio.api-keys" -a "test_exchange.api_secret" -w
security find-generic-password -s "com.cryptofolio.api-keys" -a "another_service.token" -w
```

**Expected Output:**
```
test-key-123
test-secret-456
token-789
```

**Validation:**
- ✅ All secrets retrievable
- ✅ Values match original TOML values
- ✅ Service name correct
- ✅ Account names match key names

---

### 10. Verify Database Metadata

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.headers on
.mode column
SELECT key_name, storage_type, security_level,
       datetime(migrated_at) as migration_date
FROM keychain_keys
WHERE key_name LIKE 'test_exchange.%' OR key_name LIKE 'another_service.%'
ORDER BY key_name;
EOF
```

**Expected Output:**
```
key_name                  storage_type  security_level  migration_date
------------------------  ------------  --------------  -------------------
another_service.token     keychain      standard        2026-02-21 23:59:59
test_exchange.api_key     keychain      standard        2026-02-21 23:59:59
test_exchange.api_secret  keychain      standard        2026-02-21 23:59:59
```

**Validation:**
- ✅ All 3 secrets in database
- ✅ storage_type = 'keychain'
- ✅ security_level = 'standard'
- ✅ migrated_at timestamp set (not NULL)
- ✅ Timestamps reasonable (within last few minutes)

---

### 11. Test Migration Idempotency

```bash
# Try to run migration again (should find no secrets)
./target/release/cryptofolio config migrate-to-keychain
```

**Expected Output:**
```
🔍 Scanning config.toml for secrets...

No secrets found in config.toml to migrate.

All secrets are already in the keychain.
Use 'config keychain-status' to view stored secrets.
```

**Validation:**
- ✅ Detects no secrets to migrate
- ✅ No errors
- ✅ Helpful message
- ✅ Safe to run multiple times

---

### 12. Test Rollback (Manual)

```bash
# Simulate rollback by restoring backup
cp ~/Library/Application\ Support/cryptofolio/config.toml.backup.* \
   ~/Library/Application\ Support/cryptofolio/config.toml.restored

# Verify backup is valid
./target/release/cryptofolio config show

# Check that secrets would be visible
grep -E "api_key|api_secret|token" ~/Library/Application\ Support/cryptofolio/config.toml.restored
```

**Expected:**
- ✅ Backup is valid TOML
- ✅ Contains original secrets
- ✅ Could be restored if needed

**Note:** Don't actually restore for this test, just verify backup is usable.

---

### 13. Migration with Different Security Level

```bash
# Add a new test secret
./target/release/cryptofolio config set new_test.secret "new-secret-999"

# Migrate with Touch ID Protected level
./target/release/cryptofolio config migrate-to-keychain
# Select option 2 (Touch ID Protected)
# Confirm with 'y'
```

**Expected:**
```
Found 1 secret(s) to migrate:
  • new_test.secret

Choose security level [1-3] (default: 2): 2

You selected: Touch ID Protected

[... migration proceeds ...]

✓ Migrated 'new_test.secret' to keychain (Touch ID Protected)
```

**Verify:**
```bash
./target/release/cryptofolio config keychain-status | grep new_test
```

**Expected:**
```
│ new_test.secret         │ Touch ID Protected │ ✓ Active   │
```

**Validation:**
- ✅ Different security level can be chosen
- ✅ Migration respects security level selection
- ✅ Mixed security levels supported

---

### 14. Cleanup All Test Data

```bash
# Remove all test secrets from keychain
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test_exchange.api_key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test_exchange.api_secret"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "another_service.token"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "new_test.secret"

# Remove from database
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
DELETE FROM keychain_keys
WHERE key_name LIKE 'test_exchange.%'
   OR key_name LIKE 'another_service.%'
   OR key_name LIKE 'new_test.%';
EOF

# Remove backup files
rm ~/Library/Application\ Support/cryptofolio/config.toml.backup.*

# Remove test TOML sections (if empty)
# May need to manually edit config.toml

# Verify cleanup
./target/release/cryptofolio config keychain-status
```

**Expected:**
```
No secrets currently tracked in keychain.
```

**Validation:**
- ✅ All test data removed
- ✅ Clean state restored

---

## Validation Checklist

### Migration Discovery
- [ ] Scans config.toml correctly
- [ ] Finds all secrets
- [ ] Displays secret names
- [ ] Shows count accurately
- [ ] Can be cancelled without changes

### Security Level Selection
- [ ] 3 options presented
- [ ] Clear descriptions for each level
- [ ] Recommended option highlighted
- [ ] User input validated
- [ ] Default value works (if just pressed Enter)

### Migration Execution
- [ ] Creates backup with timestamp
- [ ] Migrates all secrets to keychain
- [ ] Clears secrets from TOML
- [ ] Updates database metadata
- [ ] Shows progress for each step
- [ ] Displays success message

### Backup System
- [ ] Backup created before migration
- [ ] Backup has timestamp in filename
- [ ] Backup contains original secrets
- [ ] Backup is valid TOML
- [ ] Can be restored if needed

### Data Integrity
- [ ] All secrets in macOS Keychain
- [ ] Values match original TOML
- [ ] Database metadata correct
- [ ] migrated_at timestamps set
- [ ] storage_type = 'keychain'

### Idempotency
- [ ] Re-running migration safe
- [ ] No duplicate secrets created
- [ ] Clear message when no secrets found

### Error Handling
- [ ] Cancellation works without changes
- [ ] Invalid input rejected
- [ ] Clear error messages
- [ ] No data loss on errors

---

## Common Issues

### Issue 1: Backup Not Created
**Symptoms:** Migration proceeds but no backup file
**Solution:** Check write permissions
**Action:**
```bash
ls -ld ~/Library/Application\ Support/cryptofolio/
chmod 755 ~/Library/Application\ Support/cryptofolio/
```

### Issue 2: Secrets Not Cleared from TOML
**Symptoms:** Migration completes but TOML still has secrets
**Solution:** Check TOML write permissions
**Action:**
```bash
chmod 600 ~/Library/Application\ Support/cryptofolio/config.toml
```

### Issue 3: Keychain Access Denied
**Symptoms:** "User interaction is not allowed" during migration
**Solution:** Unlock keychain, ensure access allowed
**Action:**
```bash
security unlock-keychain ~/Library/Keychains/login.keychain-db
```

### Issue 4: Partial Migration
**Symptoms:** Some secrets migrated, some failed
**Solution:** Check error messages, retry failed secrets
**Action:**
```bash
# Restore backup if needed
cp config.toml.backup.* config.toml
# Fix issue (permissions, keychain access)
# Re-run migration
```

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**Secrets Migrated:** _______________
**Security Level Used:** _______________

**Tests Passed:**
- [ ] Migration discovery
- [ ] Security level selection
- [ ] Backup creation
- [ ] Migration execution
- [ ] Secrets in keychain
- [ ] Secrets cleared from TOML
- [ ] Database metadata
- [ ] Idempotency
- [ ] Rollback capability
- [ ] Cleanup

**Notes:**
```
_____________________________________________
_____________________________________________
_____________________________________________
```

**Issues Found:**
```
_____________________________________________
_____________________________________________
```

---

**Next Test:** [61-keychain-operations.md](61-keychain-operations.md)
