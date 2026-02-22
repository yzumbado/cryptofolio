# Test 11: Configuration - Keychain Integration

**Estimated Time:** 15 minutes
**Platform:** macOS (keychain features), Linux/Windows (fallback testing)
**Phase:** Phase 2 (Keychain Security)

---

## Overview

Verify keychain integration for secure credential storage, including Touch ID security levels, session caching, and TOML fallback.

---

## Prerequisites

- ✅ macOS system (for keychain testing)
- ✅ Binary compiled with security-framework
- ✅ Database initialized (Test 01 passed)
- ✅ Config basics working (Test 10 passed)
- ✅ Touch ID available (optional - will test fallback)

---

## Test Steps

### 1. Check Keychain Status (Empty State)

```bash
./target/release/cryptofolio config keychain-status
```

**Expected Output (if no secrets migrated yet):**
```
No secrets currently tracked in keychain.

Use 'config set-secret <key>' to store secrets in macOS Keychain.
Use 'config migrate-to-keychain' to migrate existing secrets from config.toml.
```

**Validation:**
- ✅ Command runs without errors
- ✅ Helpful message displayed
- ✅ Clear instructions for next steps

---

### 2. Set Secret with Default Security Level

```bash
# Set a test secret
echo "test-secret-12345" | ./target/release/cryptofolio config set-secret test.api_key
```

**Expected Output:**
```
✓ Stored 'test.api_key' in macOS Keychain
  Security Level: Standard

ℹ Note: Secret stored in macOS Keychain (encrypted by your system).
       Will be available to this application when your Mac is unlocked.
```

**Validation:**
- ✅ Success message displayed
- ✅ Security level shown (Standard by default)
- ✅ No plaintext in output
- ✅ No errors

---

### 3. Verify Secret in Keychain Status

```bash
./target/release/cryptofolio config keychain-status
```

**Expected Output:**
```
┌────────────────────────┬──────────────────┬────────────┐
│ Key                    │ Security Level   │ Status     │
├────────────────────────┼──────────────────┼────────────┤
│ test.api_key           │ Standard         │ ✓ Active   │
└────────────────────────┴──────────────────┴────────────┘

Total: 1 secret(s) in keychain
```

**Validation:**
- ✅ Secret appears in table
- ✅ Security level: Standard
- ✅ Status: Active
- ✅ Count correct (1 secret)

---

### 4. Verify Secret in macOS Keychain (System Level)

```bash
# Use macOS security command to verify
security find-generic-password -s "com.cryptofolio.api-keys" -a "test.api_key" -w
```

**Expected Output:**
```
test-secret-12345
```

**Validation:**
- ✅ Secret retrievable via macOS security tool
- ✅ Service name: `com.cryptofolio.api-keys`
- ✅ Account name matches key: `test.api_key`
- ✅ Value matches what was set

---

### 5. Verify Database Metadata

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
SELECT key_name, storage_type, security_level, created_at
FROM keychain_keys
WHERE key_name = 'test.api_key';
EOF
```

**Expected Output:**
```
test.api_key|keychain|standard|2026-02-21 23:XX:XX
```

**Validation:**
- ✅ Metadata exists in database
- ✅ storage_type = 'keychain'
- ✅ security_level = 'standard'
- ✅ created_at timestamp valid

---

### 6. Set Secret with Touch ID Security Level

```bash
# Set secret with Touch ID protection
echo "secure-secret-789" | ./target/release/cryptofolio config set-secret production.api_key --security-level touchid
```

**Expected Output:**
```
✓ Stored 'production.api_key' in macOS Keychain
  Security Level: Touch ID Protected

ℹ Note: Touch ID protection requested but not yet fully implemented.
       Secret stored in standard keychain (still encrypted by macOS).

       Full Touch ID prompts planned for v0.3.1 using FFI bindings.
```

**Validation:**
- ✅ Success message displayed
- ✅ Security level: Touch ID Protected
- ✅ Documented limitation shown
- ✅ Secret still stored securely

---

### 7. Verify Multiple Secrets in Status

```bash
./target/release/cryptofolio config keychain-status
```

**Expected Output:**
```
┌────────────────────────┬──────────────────┬────────────┐
│ Key                    │ Security Level   │ Status     │
├────────────────────────┼──────────────────┼────────────┤
│ production.api_key     │ Touch ID Protected │ ✓ Active   │
│ test.api_key           │ Standard         │ ✓ Active   │
└────────────────────────┴──────────────────┴────────────┘

Total: 2 secret(s) in keychain
```

**Validation:**
- ✅ Both secrets listed
- ✅ Different security levels shown
- ✅ Count correct (2 secrets)
- ✅ Alphabetical or logical ordering

---

### 8. JSON Output Format

```bash
./target/release/cryptofolio config keychain-status --json
```

**Expected Output:**
```json
[
  {
    "key_name": "production.api_key",
    "storage_type": "keychain",
    "security_level": "touchid",
    "last_accessed": null,
    "migrated_at": null
  },
  {
    "key_name": "test.api_key",
    "storage_type": "keychain",
    "security_level": "standard",
    "last_accessed": null,
    "migrated_at": null
  }
]
```

**Validation:**
- ✅ Valid JSON format
- ✅ All fields present
- ✅ Correct data types
- ✅ Parseable by tools (jq, python, etc.)

**Verify with jq:**
```bash
./target/release/cryptofolio config keychain-status --json | jq '.[0].key_name'
# Should output: "production.api_key"
```

---

### 9. Retrieve Secret via Settings

```bash
# This tests internal retrieval (not a direct command)
# The secret should be retrievable by the application

# Verify in database that it exists
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite \
  "SELECT key_name FROM keychain_keys WHERE key_name = 'test.api_key'"
```

**Expected:**
```
test.api_key
```

**Validation:**
- ✅ Secret tracked in database
- ✅ Application can retrieve when needed
- ✅ No errors accessing keychain

---

### 10. Error Handling - Non-Existent Key

```bash
# Try to get a key that doesn't exist
security find-generic-password -s "com.cryptofolio.api-keys" -a "nonexistent.key" 2>&1
```

**Expected Output:**
```
security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain.
```

**In Application:**
If we had a command to retrieve (we don't expose this directly), it would show:
```
[ERROR] Secret 'nonexistent.key' not found in keychain
```

**Validation:**
- ✅ Appropriate error for missing keys
- ✅ No crash
- ✅ Clear error message

---

### 11. Set Secret from Environment Variable

```bash
# Set environment variable
export MY_SECRET="env-secret-456"

# Set secret from env var
./target/release/cryptofolio config set-secret env.test_key --from-env MY_SECRET

# Verify
./target/release/cryptofolio config keychain-status | grep env.test_key
```

**Expected:**
- ✅ Secret stored from environment variable
- ✅ Value not echoed to terminal
- ✅ Appears in keychain status

**Cleanup:**
```bash
unset MY_SECRET
```

---

### 12. Security Level Validation

```bash
# Try invalid security level
echo "test" | ./target/release/cryptofolio config set-secret test.invalid --security-level invalid 2>&1
```

**Expected Output:**
```
error: invalid value 'invalid' for '--security-level <SECURITY_LEVEL>'
  [possible values: standard, touchid, touchid-only]

For more information, try '--help'.
```

**Validation:**
- ✅ Invalid values rejected
- ✅ Shows valid options
- ✅ Suggests --help
- ✅ Exit code non-zero

---

### 13. Platform Detection (macOS)

```bash
# On macOS, keychain should be available
./target/release/cryptofolio config keychain-status
```

**Expected:**
- ✅ Command works (keychain available)
- ✅ No "platform not supported" errors
- ✅ macOS Keychain integration active

---

### 14. Session Caching Verification

```bash
# First access (may trigger keychain access)
./target/release/cryptofolio config keychain-status > /dev/null

# Immediate second access (should use cache)
time ./target/release/cryptofolio config keychain-status > /dev/null

# Multiple rapid accesses
for i in {1..5}; do
  ./target/release/cryptofolio config keychain-status > /dev/null
  echo "Access $i completed"
done
```

**Expected:**
- ✅ All commands execute quickly (<100ms)
- ✅ No repeated keychain prompts
- ✅ Session cache working (15-minute TTL)

---

### 15. Cleanup Test Secrets

```bash
# Remove test secrets from keychain
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test.api_key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "production.api_key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "env.test_key"

# Remove from database
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
DELETE FROM keychain_keys WHERE key_name IN ('test.api_key', 'production.api_key', 'env.test_key');
EOF

# Verify cleanup
./target/release/cryptofolio config keychain-status
```

**Expected:**
```
No secrets currently tracked in keychain.
```

**Validation:**
- ✅ All test secrets removed
- ✅ Keychain clean
- ✅ Database clean

---

## Validation Checklist

### Keychain Storage
- [ ] Set secret command works
- [ ] Secret stored in macOS Keychain
- [ ] Secret not in config.toml (plaintext)
- [ ] Database metadata created
- [ ] Keychain status shows secret

### Security Levels
- [ ] Standard level works
- [ ] Touch ID Protected level works
- [ ] Touch ID Only level works (if available)
- [ ] Invalid levels rejected
- [ ] Default level appropriate (Standard)

### Keychain Status Command
- [ ] Table format displays correctly
- [ ] JSON format valid and parseable
- [ ] Shows all secrets
- [ ] Shows security levels
- [ ] Shows active status
- [ ] Counts accurate

### System Integration
- [ ] macOS Keychain stores secrets correctly
- [ ] Service name: com.cryptofolio.api-keys
- [ ] Account names match key names
- [ ] Secrets retrievable via security command

### Database Metadata
- [ ] keychain_keys table populated
- [ ] storage_type = 'keychain'
- [ ] security_level correct
- [ ] Timestamps valid

### Error Handling
- [ ] Non-existent keys handled
- [ ] Invalid security levels rejected
- [ ] Clear error messages
- [ ] No crashes

### Performance
- [ ] Session caching works
- [ ] No repeated prompts within session
- [ ] Fast access (<100ms)

---

## Common Issues

### Issue 1: Keychain Access Denied
**Symptoms:** "User interaction is not allowed" error
**Solution:** Unlock Mac, ensure Keychain Access allowed
**Action:**
```bash
# Check if keychain is locked
security unlock-keychain ~/Library/Keychains/login.keychain-db
```

### Issue 2: Touch ID Not Available
**Symptoms:** Touch ID level set but prompts not appearing
**Solution:** Expected - full Touch ID prompts require FFI (v0.3.1)
**Action:** Use Standard level for now, or wait for v0.3.1

### Issue 3: Secret Not Found After Setting
**Symptoms:** Secret set successfully but not in keychain-status
**Solution:** Check database for metadata
**Action:**
```bash
sqlite3 database.sqlite "SELECT * FROM keychain_keys"
# If missing, database write failed - check permissions
```

### Issue 4: Multiple Keychains Conflict
**Symptoms:** Secret stored in wrong keychain
**Solution:** Ensure default keychain is login.keychain
**Action:**
```bash
security default-keychain
# Should show: login.keychain-db
```

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**macOS Version:** _______________
**Touch ID Available:** [ ] Yes [ ] No

**Tests Passed:**
- [ ] Set secret (default level)
- [ ] Set secret (Touch ID level)
- [ ] Set secret from environment variable
- [ ] Keychain status (table format)
- [ ] Keychain status (JSON format)
- [ ] macOS Keychain verification
- [ ] Database metadata tracking
- [ ] Session caching
- [ ] Error handling
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

**Next Test:** [60-keychain-migration.md](60-keychain-migration.md)
