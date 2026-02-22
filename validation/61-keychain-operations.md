# Test 61: Keychain Operations

**Estimated Time:** 10 minutes
**Platform:** macOS (keychain features)
**Phase:** Phase 2 (Keychain Security)

---

## Overview

Verify day-to-day keychain operations including secret retrieval, session caching, error handling, and interaction with existing commands.

---

## Prerequisites

- ✅ macOS system with keychain access
- ✅ Binary compiled with Phase 2 features
- ✅ Keychain basics working (Test 11 passed)
- ✅ At least one secret in keychain

---

## Test Steps

### 1. Setup - Create Test Secrets

```bash
# Create test secrets with different security levels
echo "standard-secret-123" | ./target/release/cryptofolio config set-secret ops.standard --security-level standard

echo "touchid-secret-456" | ./target/release/cryptofolio config set-secret ops.touchid --security-level touchid

# Verify created
./target/release/cryptofolio config keychain-status
```

**Expected Output:**
```
┌────────────────────────┬──────────────────┬────────────┐
│ Key                    │ Security Level   │ Status     │
├────────────────────────┼──────────────────┼────────────┤
│ ops.standard           │ Standard         │ ✓ Active   │
│ ops.touchid            │ Touch ID Protected │ ✓ Active   │
└────────────────────────┴──────────────────┴────────────┘

Total: 2 secret(s) in keychain
```

**Validation:**
- ✅ Both secrets created
- ✅ Different security levels
- ✅ Ready for operations testing

---

### 2. Retrieve Secret via macOS Security Tool

```bash
# Retrieve using system command
security find-generic-password -s "com.cryptofolio.api-keys" -a "ops.standard" -w
```

**Expected Output:**
```
standard-secret-123
```

**Validation:**
- ✅ Secret retrievable
- ✅ Value correct
- ✅ macOS Keychain integration working

---

### 3. Test Session Caching

```bash
# First access (cold cache)
time ./target/release/cryptofolio config keychain-status

# Immediate second access (warm cache)
time ./target/release/cryptofolio config keychain-status

# Third access (still cached)
time ./target/release/cryptofolio config keychain-status
```

**Expected:**
- ✅ All commands complete quickly (<100ms)
- ✅ No performance degradation
- ✅ Session cache working
- ✅ No repeated keychain prompts

**Note:** Cache TTL is 15 minutes, so all accesses within this test should hit cache.

---

### 4. Rapid Sequential Access

```bash
# Simulate rapid command execution (automation scenario)
for i in {1..10}; do
  ./target/release/cryptofolio config keychain-status > /dev/null
  echo "Access $i completed"
done
```

**Expected:**
- ✅ All 10 accesses succeed
- ✅ No errors
- ✅ Consistent performance
- ✅ Cache prevents repeated keychain access

---

### 5. Check Last Accessed Timestamp

```bash
# Trigger access
./target/release/cryptofolio config keychain-status > /dev/null

# Check database for last_accessed timestamp
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
SELECT key_name,
       datetime(last_accessed) as last_access,
       datetime(created_at) as created
FROM keychain_keys
WHERE key_name LIKE 'ops.%'
ORDER BY key_name;
EOF
```

**Expected Output:**
```
key_name      last_access          created
------------  -------------------  -------------------
ops.standard  2026-02-21 23:XX:XX  2026-02-21 23:XX:XX
ops.touchid   2026-02-21 23:XX:XX  2026-02-21 23:XX:XX
```

**Validation:**
- ✅ last_accessed timestamp updated
- ✅ Timestamp recent (within last minute)
- ✅ Timestamp NOT null (was updated)

---

### 6. JSON Output with Timestamps

```bash
./target/release/cryptofolio config keychain-status --json | jq '.'
```

**Expected Output:**
```json
[
  {
    "key_name": "ops.standard",
    "storage_type": "keychain",
    "security_level": "standard",
    "last_accessed": "2026-02-21T23:XX:XXZ",
    "migrated_at": null
  },
  {
    "key_name": "ops.touchid",
    "storage_type": "keychain",
    "security_level": "touchid",
    "last_accessed": "2026-02-21T23:XX:XXZ",
    "migrated_at": null
  }
]
```

**Validation:**
- ✅ Valid JSON
- ✅ last_accessed populated
- ✅ Timestamps in ISO 8601 format
- ✅ migrated_at null (not migrated, created directly)

---

### 7. Error Handling - Delete Secret and Try to Access

```bash
# Delete secret from keychain (but leave metadata)
security delete-generic-password -s "com.cryptofolio.api-keys" -a "ops.standard"

# Try to access via status command
./target/release/cryptofolio config keychain-status 2>&1
```

**Expected:**
- ⚠️ May show secret in status (metadata exists in DB)
- ⚠️ But actual retrieval would fail

**Note:** This tests edge case where metadata and keychain are out of sync.

**Cleanup:**
```bash
# Re-create the secret
echo "standard-secret-123" | ./target/release/cryptofolio config set-secret ops.standard --security-level standard
```

---

### 8. Test with Existing Commands (Integration)

```bash
# Create a test account that might use API keys
./target/release/cryptofolio account add "Test Exchange" --type exchange --category Trading

# Set fake API credentials in keychain
echo "test-api-key-789" | ./target/release/cryptofolio config set-secret test_exchange.api_key
echo "test-api-secret-000" | ./target/release/cryptofolio config set-secret test_exchange.api_secret

# Verify stored
./target/release/cryptofolio config keychain-status | grep test_exchange
```

**Expected:**
```
│ test_exchange.api_key    │ Standard         │ ✓ Active   │
│ test_exchange.api_secret │ Standard         │ ✓ Active   │
```

**Validation:**
- ✅ Keychain integrates with existing commands
- ✅ Account creation unaffected
- ✅ Credentials stored securely
- ✅ No breaking changes

---

### 9. Concurrent Access Test

```bash
# Simulate concurrent access (multiple terminal sessions)
./target/release/cryptofolio config keychain-status &
./target/release/cryptofolio config keychain-status &
./target/release/cryptofolio config keychain-status &
wait
```

**Expected:**
- ✅ All processes complete successfully
- ✅ No database locks
- ✅ No keychain conflicts
- ✅ All return correct data

---

### 10. SSH Session Detection

```bash
# Simulate SSH session (set environment variables)
export SSH_CONNECTION="fake_ssh_connection"
export SSH_CLIENT="fake"

# Try keychain operation
./target/release/cryptofolio config keychain-status

# Clean up
unset SSH_CONNECTION
unset SSH_CLIENT
```

**Expected:**
- ✅ Command still works (keychain available locally)
- ℹ️ May show different behavior if Touch ID detection is active
- ✅ Graceful handling of SSH context

**Note:** Current implementation doesn't prevent access in SSH, but tracks the context.

---

### 11. Empty Key Name Handling

```bash
# Try to set secret with empty key name
echo "test" | ./target/release/cryptofolio config set-secret "" 2>&1
```

**Expected Output:**
```
error: the following required arguments were not provided:
  <KEY>

Usage: cryptofolio config set-secret <KEY> [OPTIONS]

For more information, try '--help'.
```

**Validation:**
- ✅ Rejected by CLI parser
- ✅ Clear error message
- ✅ No crash
- ✅ Exit code non-zero

---

### 12. Special Characters in Key Names

```bash
# Try key with dots (valid)
echo "test" | ./target/release/cryptofolio config set-secret special.test.key

# Try key with spaces (should be handled)
echo "test" | ./target/release/cryptofolio config set-secret "special key" 2>&1

# Try key with special characters
echo "test" | ./target/release/cryptofolio config set-secret "special@key" 2>&1
```

**Expected:**
- ✅ Dots in key names allowed (standard format: section.key)
- ⚠️ Spaces may be accepted but not recommended
- ⚠️ Special chars (@, -, _) behavior varies

**Validation:**
- ✅ Valid key names work
- ✅ Invalid key names handled gracefully

---

### 13. Long Secret Values

```bash
# Create a very long secret (e.g., 1000 characters)
LONG_SECRET=$(python3 -c "print('a' * 1000)")
echo "$LONG_SECRET" | ./target/release/cryptofolio config set-secret ops.long_secret

# Verify stored
security find-generic-password -s "com.cryptofolio.api-keys" -a "ops.long_secret" -w | wc -c
```

**Expected:**
```
1000
```
(or 1001 with newline)

**Validation:**
- ✅ Long secrets handled
- ✅ No truncation
- ✅ Full value stored
- ✅ Keychain supports large values

---

### 14. Cleanup Test Data

```bash
# Remove all test secrets
security delete-generic-password -s "com.cryptofolio.api-keys" -a "ops.standard"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "ops.touchid"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test_exchange.api_key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test_exchange.api_secret"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "special.test.key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "ops.long_secret"

# Remove from database
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
DELETE FROM keychain_keys
WHERE key_name LIKE 'ops.%'
   OR key_name LIKE 'test_exchange.%'
   OR key_name LIKE 'special.%';
EOF

# Remove test account
./target/release/cryptofolio account delete "Test Exchange" --confirm

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

### Secret Retrieval
- [ ] Secrets retrievable via macOS security tool
- [ ] Values correct
- [ ] No errors during retrieval

### Session Caching
- [ ] First access works
- [ ] Subsequent accesses use cache
- [ ] Performance consistent (<100ms)
- [ ] No repeated keychain prompts within session

### Timestamp Tracking
- [ ] last_accessed updated on access
- [ ] Timestamps accurate (within minute)
- [ ] Visible in JSON output
- [ ] ISO 8601 format in JSON

### Error Handling
- [ ] Missing secrets handled gracefully
- [ ] Empty key names rejected
- [ ] Invalid input handled
- [ ] Clear error messages

### Integration
- [ ] Works with existing commands
- [ ] Account creation unaffected
- [ ] No breaking changes
- [ ] Credentials stored securely

### Edge Cases
- [ ] Concurrent access works
- [ ] SSH context detected
- [ ] Long secret values handled
- [ ] Special characters handled

### Performance
- [ ] Rapid sequential access works
- [ ] No database locks
- [ ] No keychain conflicts
- [ ] Cache effective

---

## Common Issues

### Issue 1: Cache Not Working
**Symptoms:** Every command triggers keychain access
**Solution:** Check cache implementation, TTL settings
**Action:** Review session cache logic, ensure mutex working

### Issue 2: Timestamp Not Updating
**Symptoms:** last_accessed remains NULL or stale
**Solution:** Check database write permissions
**Action:**
```bash
sqlite3 database.sqlite "UPDATE keychain_keys SET last_accessed = datetime('now')"
```

### Issue 3: Concurrent Access Fails
**Symptoms:** Database locked errors with concurrent access
**Solution:** SQLite WAL mode, or ensure IMMEDIATE transactions
**Action:**
```bash
sqlite3 database.sqlite "PRAGMA journal_mode=WAL"
```

### Issue 4: Long Secret Truncated
**Symptoms:** Secret value incomplete
**Solution:** Check keychain item size limits (macOS supports up to ~2GB)
**Action:** Should not occur with reasonable secret sizes

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**macOS Version:** _______________

**Tests Passed:**
- [ ] Secret retrieval
- [ ] Session caching
- [ ] Rapid sequential access
- [ ] Timestamp tracking
- [ ] JSON output
- [ ] Integration with existing commands
- [ ] Concurrent access
- [ ] SSH context handling
- [ ] Error handling
- [ ] Special characters
- [ ] Long secret values
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

**Next Test:** [62-keychain-security.md](62-keychain-security.md)
