# Test 62: Keychain Security Level Management

**Estimated Time:** 10 minutes
**Platform:** macOS (keychain features)
**Phase:** Phase 2 (Keychain Security)

---

## Overview

Verify security level upgrade/downgrade operations, including validation, warnings, confirmations, and proper keychain re-storage.

---

## Prerequisites

- ✅ macOS system with keychain access
- ✅ Binary compiled with Phase 2 features
- ✅ Keychain operations working (Test 61 passed)
- ✅ At least one secret in keychain

---

## Test Steps

### 1. Setup - Create Test Secret with Standard Level

```bash
# Create secret with Standard security level
echo "upgrade-test-secret-123" | ./target/release/cryptofolio config set-secret security.test_key --security-level standard

# Verify initial state
./target/release/cryptofolio config keychain-status | grep security.test_key
```

**Expected Output:**
```
│ security.test_key       │ Standard         │ ✓ Active   │
```

**Validation:**
- ✅ Secret created with Standard level
- ✅ Ready for upgrade testing

---

### 2. Upgrade Security Level - Standard to Touch ID

```bash
# Upgrade to Touch ID Protected
./target/release/cryptofolio config upgrade-security security.test_key --to touchid
```

**Expected Output:**
```
🔐 Upgrading security level for 'security.test_key'
   Current: Standard
   Target:  Touch ID Protected

✓ Security level upgraded successfully

ℹ Note: Touch ID protection requested but not yet fully implemented.
       Secret stored in standard keychain (still encrypted by macOS).

       Full Touch ID prompts planned for v0.3.1 using FFI bindings.
```

**Validation:**
- ✅ Clear confirmation message
- ✅ Shows current and target levels
- ✅ Success message displayed
- ✅ Limitation note shown (expected)
- ✅ No errors

---

### 3. Verify Upgrade in Status

```bash
# Check keychain status
./target/release/cryptofolio config keychain-status | grep security.test_key
```

**Expected Output:**
```
│ security.test_key       │ Touch ID Protected │ ✓ Active   │
```

**Validation:**
- ✅ Security level updated to Touch ID Protected
- ✅ Status still Active
- ✅ Change persisted

---

### 4. Verify Upgrade in Database

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
SELECT key_name, security_level
FROM keychain_keys
WHERE key_name = 'security.test_key';
EOF
```

**Expected Output:**
```
security.test_key|touchid
```

**Validation:**
- ✅ Database updated
- ✅ security_level = 'touchid'
- ✅ Metadata consistent with keychain

---

### 5. Verify Secret Still Retrievable After Upgrade

```bash
# Retrieve via macOS security tool
security find-generic-password -s "com.cryptofolio.api-keys" -a "security.test_key" -w
```

**Expected Output:**
```
upgrade-test-secret-123
```

**Validation:**
- ✅ Secret value unchanged
- ✅ Still retrievable
- ✅ Upgrade didn't corrupt secret

---

### 6. Upgrade to Maximum Security Level

```bash
# Upgrade to Touch ID Only
./target/release/cryptofolio config upgrade-security security.test_key --to touchid-only
```

**Expected Output:**
```
🔐 Upgrading security level for 'security.test_key'
   Current: Touch ID Protected
   Target:  Touch ID Only

✓ Security level upgraded successfully

ℹ Note: Touch ID Only protection requested.
       This is the highest security level.
       Full Touch ID prompts planned for v0.3.1 using FFI bindings.
```

**Validation:**
- ✅ Upgrade to maximum level works
- ✅ Shows progression (Touch ID Protected → Touch ID Only)
- ✅ Success message
- ✅ Appropriate note for highest level

---

### 7. Verify Maximum Security Level

```bash
./target/release/cryptofolio config keychain-status | grep security.test_key
```

**Expected Output:**
```
│ security.test_key       │ Touch ID Only    │ ✓ Active   │
```

**Validation:**
- ✅ Security level: Touch ID Only
- ✅ Highest level achieved

---

### 8. Downgrade Security Level with Warning

```bash
# Attempt to downgrade to Standard
./target/release/cryptofolio config downgrade-security security.test_key --to standard
```

**Expected Output:**
```
⚠️  WARNING: Downgrading security level
   Current: Touch ID Only
   Target:  Standard

   Standard level doesn't require Touch ID for access.
   This reduces the security protection for this secret.

Continue? [y/N]:
```

**At this point, type 'y' and press Enter.**

**After Confirmation:**
```
✓ Security level downgraded to Standard

ℹ Secret 'security.test_key' now uses Standard security level.
  Accessible when your Mac is unlocked.
```

**Validation:**
- ✅ Warning displayed before downgrade
- ✅ Confirmation prompt shown
- ✅ Clear explanation of security implications
- ✅ Success message after confirmation

---

### 9. Verify Downgrade Applied

```bash
./target/release/cryptofolio config keychain-status | grep security.test_key
```

**Expected Output:**
```
│ security.test_key       │ Standard         │ ✓ Active   │
```

**Validation:**
- ✅ Security level downgraded to Standard
- ✅ Change persisted
- ✅ Secret still active

---

### 10. Downgrade Cancellation

```bash
# Upgrade back to Touch ID
./target/release/cryptofolio config upgrade-security security.test_key --to touchid

# Try to downgrade but cancel
./target/release/cryptofolio config downgrade-security security.test_key --to standard
# When prompted "Continue? [y/N]:", type 'n' or just press Enter
```

**Expected Output:**
```
⚠️  WARNING: Downgrading security level
   Current: Touch ID Protected
   Target:  Standard

   Standard level doesn't require Touch ID for access.
   This reduces the security protection for this secret.

Continue? [y/N]: n

❌ Cancelled. Security level unchanged.
```

**Verify:**
```bash
./target/release/cryptofolio config keychain-status | grep security.test_key
```

**Expected:**
```
│ security.test_key       │ Touch ID Protected │ ✓ Active   │
```

**Validation:**
- ✅ Cancellation works
- ✅ Security level unchanged
- ✅ Clear cancellation message
- ✅ No side effects

---

### 11. Error Handling - Non-Existent Key

```bash
# Try to upgrade non-existent key
./target/release/cryptofolio config upgrade-security nonexistent.key --to touchid 2>&1
```

**Expected Output:**
```
[ERROR] Keychain error: Secret 'nonexistent.key' not found in keychain.
        Use 'config set-secret nonexistent.key' first.
```

**Validation:**
- ✅ Clear error message
- ✅ Helpful suggestion (use set-secret)
- ✅ No crash
- ✅ Exit code non-zero

---

### 12. Error Handling - Invalid Security Level

```bash
# Try to upgrade to invalid level
./target/release/cryptofolio config upgrade-security security.test_key --to invalid 2>&1
```

**Expected Output:**
```
error: invalid value 'invalid' for '--to <TO>'
  [possible values: touchid, touchid-only]

For more information, try '--help'.
```

**Validation:**
- ✅ Caught by CLI parser (clap)
- ✅ Shows valid options
- ✅ Suggests --help
- ✅ Exit code 2 (CLI error)

---

### 13. Error Handling - Downgrade Invalid Level

```bash
# Try to downgrade to invalid level
./target/release/cryptofolio config downgrade-security security.test_key --to invalid 2>&1
```

**Expected Output:**
```
error: invalid value 'invalid' for '--to <TO>'
  [possible values: standard, touchid]

For more information, try '--help'.
```

**Validation:**
- ✅ Invalid values rejected
- ✅ Shows valid downgrade options
- ✅ Different valid values than upgrade (no touchid-only for downgrade)

---

### 14. Upgrade Already at Target Level

```bash
# Upgrade to current level (should be no-op or informational)
./target/release/cryptofolio config upgrade-security security.test_key --to touchid
```

**Expected:**
- ✅ Either: Success message (no-op)
- ✅ Or: Informational message ("already at this level")
- ✅ No error
- ✅ Idempotent operation

---

### 15. Multiple Security Level Changes

```bash
# Cycle through all levels
echo "cycle-test" | ./target/release/cryptofolio config set-secret security.cycle --security-level standard

# Standard → Touch ID
./target/release/cryptofolio config upgrade-security security.cycle --to touchid

# Touch ID → Touch ID Only
./target/release/cryptofolio config upgrade-security security.cycle --to touchid-only

# Touch ID Only → Touch ID (downgrade)
./target/release/cryptofolio config downgrade-security security.cycle --to touchid <<< "y"

# Touch ID → Standard (downgrade)
./target/release/cryptofolio config downgrade-security security.cycle --to standard <<< "y"

# Verify final state
./target/release/cryptofolio config keychain-status | grep security.cycle
```

**Expected Final State:**
```
│ security.cycle          │ Standard         │ ✓ Active   │
```

**Validation:**
- ✅ All transitions work
- ✅ Secret value preserved through all changes
- ✅ No corruption
- ✅ State consistent

---

### 16. Cleanup Test Data

```bash
# Remove all test secrets
security delete-generic-password -s "com.cryptofolio.api-keys" -a "security.test_key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "security.cycle"

# Remove from database
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
DELETE FROM keychain_keys WHERE key_name LIKE 'security.%';
EOF

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

### Upgrade Operations
- [ ] Standard → Touch ID works
- [ ] Touch ID → Touch ID Only works
- [ ] Shows current and target levels
- [ ] Success message displayed
- [ ] Database updated
- [ ] Keychain updated
- [ ] Secret value preserved

### Downgrade Operations
- [ ] Warning displayed before downgrade
- [ ] Confirmation prompt shown
- [ ] Security implications explained
- [ ] Downgrade executes after confirmation
- [ ] Database updated
- [ ] Keychain updated
- [ ] Secret value preserved

### Cancellation
- [ ] Downgrade can be cancelled
- [ ] Upgrade can be interrupted (Ctrl+C)
- [ ] No changes on cancellation
- [ ] Clear cancellation message

### Error Handling
- [ ] Non-existent keys rejected
- [ ] Invalid security levels rejected
- [ ] Helpful error messages
- [ ] Suggestions provided
- [ ] No crashes
- [ ] Proper exit codes

### Data Integrity
- [ ] Secret values unchanged through operations
- [ ] Database metadata consistent
- [ ] Keychain storage consistent
- [ ] No data loss
- [ ] No corruption

### Edge Cases
- [ ] Upgrade to same level handled
- [ ] Multiple level changes work
- [ ] All transitions possible
- [ ] Idempotent operations

---

## Common Issues

### Issue 1: Downgrade Proceeds Without Confirmation
**Symptoms:** No prompt shown, downgrade happens immediately
**Solution:** Check implementation of confirmation prompt
**Action:** Verify stdin reading, ensure prompt logic correct

### Issue 2: Secret Value Changes After Level Change
**Symptoms:** Retrieved value differs after upgrade/downgrade
**Solution:** Check re-storage logic
**Action:** Verify value retrieved before deletion, re-stored correctly

### Issue 3: Database Out of Sync with Keychain
**Symptoms:** Status shows one level, but keychain has different access control
**Solution:** Ensure atomic update (database + keychain)
**Action:** Review transaction boundaries, ensure both update or none

### Issue 4: Keychain Access Denied During Operation
**Symptoms:** "User interaction is not allowed" during upgrade/downgrade
**Solution:** Unlock keychain, ensure access allowed
**Action:**
```bash
security unlock-keychain ~/Library/Keychains/login.keychain-db
```

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**macOS Version:** _______________
**Touch ID Available:** [ ] Yes [ ] No

**Tests Passed:**
- [ ] Upgrade Standard → Touch ID
- [ ] Upgrade Touch ID → Touch ID Only
- [ ] Downgrade with warning
- [ ] Downgrade cancellation
- [ ] Error handling (non-existent key)
- [ ] Error handling (invalid level)
- [ ] Multiple level changes
- [ ] Secret value preservation
- [ ] Database consistency
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

**Next Test:** [80-backward-compat.md](80-backward-compat.md)
