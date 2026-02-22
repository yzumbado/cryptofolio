# Phase 2 Manual Testing Guide

## Test Environment
- **Platform:** macOS (required for keychain features)
- **Binary:** `./target/release/cryptofolio`
- **Database:** `~/.config/cryptofolio/database.sqlite`
- **Config:** `~/.config/cryptofolio/config.toml`

---

## Test Suite 1: Keychain Migration Workflow

### Test 1.1: Pre-Migration State Check
**Objective:** Verify current configuration state

```bash
# Check current config
./target/release/cryptofolio config show

# Check if any secrets exist in TOML
cat ~/.config/cryptofolio/config.toml | grep -i "secret\|key"

# Check keychain status (should be empty)
./target/release/cryptofolio config keychain-status
```

**Expected Results:**
- Config shows current settings
- May or may not have secrets in TOML
- Keychain status shows "No secrets tracked in keychain"

---

### Test 1.2: Add Test Secret to TOML
**Objective:** Create a secret to migrate

```bash
# Add a test secret to TOML (using interactive input)
./target/release/cryptofolio config set-secret test.api_key

# When prompted, enter: "test-secret-value-123"
# Type 'y' to confirm security warning
```

**Expected Results:**
- Secret saved to config.toml
- Warning about plaintext storage shown
- File permissions set to 0600

**Verification:**
```bash
# Verify secret is in TOML
cat ~/.config/cryptofolio/config.toml | grep test

# Should show something like:
# [test]
# api_key = "test-secret-value-123"
```

---

### Test 1.3: Run Keychain Migration
**Objective:** Migrate secrets from TOML to keychain

```bash
# Run migration wizard
./target/release/cryptofolio config migrate-to-keychain
```

**Expected Workflow:**
1. Shows found secrets (test.api_key)
2. Prompts for security level selection
3. Shows explanation of each level
4. Prompts for confirmation
5. Creates backup (config.toml.backup)
6. Migrates to keychain
7. Updates database
8. Clears from TOML
9. Shows success message

**Inputs to Provide:**
- Security level: Choose `2` (Touch ID Protected - Recommended)
- Confirmation: Type `y`

**Expected Results:**
- ✓ Created backup message
- ✓ Migrated secret to keychain message
- ✓ Cleared secrets from config.toml
- ✓ Migration complete message

---

### Test 1.4: Verify Migration Results
**Objective:** Confirm migration was successful

```bash
# 1. Check backup was created
ls -la ~/.config/cryptofolio/config.toml.backup

# 2. Verify secret removed from TOML
cat ~/.config/cryptofolio/config.toml | grep test
# Should NOT show the secret value

# 3. Check keychain status
./target/release/cryptofolio config keychain-status

# 4. Verify secret is in macOS Keychain (system command)
security find-generic-password -s "com.cryptofolio.api-keys" -a "test.api_key" -w
```

**Expected Results:**
- Backup file exists with timestamp
- TOML no longer contains secret value
- keychain-status shows the secret with "Touch ID Protected" level
- macOS security command retrieves the secret value

---

## Test Suite 2: Touch ID Behavior

### Test 2.1: Keychain Retrieval
**Objective:** Test secret retrieval from keychain

```bash
# The app should retrieve secrets from keychain automatically
# Let's test by trying to use a command that needs credentials
./target/release/cryptofolio config show
```

**Expected Results:**
- ⚠️ **Note:** Due to security-framework API limitations, native Touch ID prompt won't appear
- Secret should be retrieved from keychain silently
- Session caching should work (no repeated prompts within 15 minutes)

**Verification:**
```bash
# Check database for last_accessed timestamp
sqlite3 ~/.config/cryptofolio/database.sqlite \
  "SELECT key_name, security_level, last_accessed FROM keychain_keys;"
```

---

### Test 2.2: Session Caching
**Objective:** Verify session cache works (15-minute timeout)

```bash
# First access (should retrieve from keychain)
./target/release/cryptofolio config show

# Immediate second access (should use cache)
./target/release/cryptofolio config show

# Multiple rapid accesses (all should use cache)
for i in {1..5}; do
  ./target/release/cryptofolio config show > /dev/null
  echo "Access $i completed"
done
```

**Expected Results:**
- All commands execute quickly (cache hit)
- No performance degradation

---

### Test 2.3: SSH Session Detection
**Objective:** Test fallback when Touch ID unavailable

```bash
# Simulate SSH session
export SSH_CONNECTION="fake_ssh"
export SSH_CLIENT="fake"

# Try to use keychain
./target/release/cryptofolio config keychain-status

# Clean up
unset SSH_CONNECTION
unset SSH_CLIENT
```

**Expected Results:**
- Should still work (keychain access doesn't require SSH)
- May show different behavior if Touch ID detection is active

---

## Test Suite 3: Security Level Management

### Test 3.1: Check Current Security Levels
**Objective:** View all secrets and their security levels

```bash
# Table view
./target/release/cryptofolio config keychain-status

# JSON view
./target/release/cryptofolio config keychain-status --json
```

**Expected Results:**
- Shows test.api_key with "Touch ID Protected" level
- JSON output is valid and parseable
- Status shows "✓ Active" for keychain entries

---

### Test 3.2: Upgrade Security Level
**Objective:** Increase security to maximum level

```bash
# Upgrade to Touch ID Only
./target/release/cryptofolio config upgrade-security test.api_key --to touchid-only
```

**Expected Results:**
- Confirmation message about upgrade
- Success message
- May trigger keychain access (to retrieve current secret)

**Verification:**
```bash
# Check new security level
./target/release/cryptofolio config keychain-status

# Should show "Touch ID Only"
```

---

### Test 3.3: Downgrade Security Level
**Objective:** Decrease security level

```bash
# Downgrade to Standard
./target/release/cryptofolio config downgrade-security test.api_key --to standard
```

**Expected Results:**
- Warning about downgrading to Standard
- Confirmation prompt: "Continue? [y/N]"
- After typing 'y': Success message

**Verification:**
```bash
# Check new security level
./target/release/cryptofolio config keychain-status

# Should show "Standard"
```

---

### Test 3.4: Upgrade Back to Touch ID
**Objective:** Return to recommended security level

```bash
# Upgrade to Touch ID Protected
./target/release/cryptofolio config upgrade-security test.api_key --to touchid
```

**Expected Results:**
- Success message
- No warning (upgrading is safe)

---

## Test Suite 4: Set Secret with Security Level

### Test 4.1: Set New Secret with Standard Level
**Objective:** Add secret directly to keychain with specific security level

```bash
# Set secret with standard security (for automation)
echo "automation-secret-456" | \
  ./target/release/cryptofolio config set-secret automation.key --security-level standard
```

**Expected Results:**
- Secret stored in keychain
- Shows "Standard" security level
- No security warning (keychain is secure)

---

### Test 4.2: Set New Secret with Touch ID Level
**Objective:** Add secret with maximum security

```bash
# Set secret with Touch ID protection
echo "secure-secret-789" | \
  ./target/release/cryptofolio config set-secret production.key --security-level touchid
```

**Expected Results:**
- Secret stored in keychain
- Shows "Touch ID Protected" security level

---

### Test 4.3: Set Secret Without Security Level (Default)
**Objective:** Test default behavior

```bash
# Set secret without specifying level (should default to Touch ID if available)
echo "default-secret-000" | \
  ./target/release/cryptofolio config set-secret default.key
```

**Expected Results:**
- Secret stored in keychain
- Uses default security level (Touch ID Protected on macOS)

---

## Test Suite 5: Backward Compatibility

### Test 5.1: TOML Fallback
**Objective:** Verify TOML storage still works

```bash
# Temporarily break keychain access by using a non-macOS platform check
# Or just add a secret that explicitly goes to TOML

# Use regular set command (not set-secret)
./target/release/cryptofolio config set general.test_value "123"

# Verify it's in TOML
cat ~/.config/cryptofolio/config.toml | grep test_value
```

**Expected Results:**
- Non-secret values still stored in TOML
- Normal config operations unaffected

---

### Test 5.2: Mixed Storage
**Objective:** Verify keychain and TOML work together

```bash
# Check status showing both storage types
./target/release/cryptofolio config keychain-status

# Some keys in keychain, some in TOML
./target/release/cryptofolio config show
```

**Expected Results:**
- Both storage types work simultaneously
- No conflicts or errors

---

## Test Suite 6: Error Handling

### Test 6.1: Non-Existent Key
**Objective:** Test error handling for missing keys

```bash
# Try to upgrade non-existent key
./target/release/cryptofolio config upgrade-security nonexistent.key --to touchid
```

**Expected Results:**
- Clear error message: "Secret 'nonexistent.key' not found in keychain"
- Suggestion to use 'config set-secret' first

---

### Test 6.2: Invalid Security Level
**Objective:** Test validation of security levels

```bash
# Try invalid security level
./target/release/cryptofolio config upgrade-security test.api_key --to invalid-level 2>&1
```

**Expected Results:**
- Error message about invalid security level
- Should fail before making any changes

---

### Test 6.3: Cancelled Migration
**Objective:** Test migration cancellation

```bash
# Run migration but cancel when prompted
./target/release/cryptofolio config migrate-to-keychain
# When prompted for confirmation, type 'n'
```

**Expected Results:**
- "Cancelled. No changes made." message
- No backup created
- TOML unchanged
- Keychain unchanged

---

## Test Suite 7: Database Integrity

### Test 7.1: Verify Database State
**Objective:** Check database metadata is correct

```bash
# Check keychain_keys table
sqlite3 ~/.config/cryptofolio/database.sqlite << EOF
SELECT
  key_name,
  storage_type,
  security_level,
  datetime(created_at) as created,
  datetime(migrated_at) as migrated
FROM keychain_keys
ORDER BY key_name;
EOF
```

**Expected Results:**
- All migrated keys have `storage_type = 'keychain'`
- Security levels match what was set
- Timestamps are reasonable

---

### Test 7.2: Migration History
**Objective:** Verify migration tracking

```bash
sqlite3 ~/.config/cryptofolio/database.sqlite << EOF
SELECT
  key_name,
  datetime(migrated_at) as migration_date
FROM keychain_keys
WHERE migrated_at IS NOT NULL;
EOF
```

**Expected Results:**
- Shows when each key was migrated
- Null for keys added directly to keychain

---

## Test Suite 8: Cleanup and Reset

### Test 8.1: Remove Test Secrets
**Objective:** Clean up test data

```bash
# Remove from keychain
security delete-generic-password -s "com.cryptofolio.api-keys" -a "test.api_key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "automation.key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "production.key"
security delete-generic-password -s "com.cryptofolio.api-keys" -a "default.key"

# Clean up database
sqlite3 ~/.config/cryptofolio/database.sqlite << EOF
DELETE FROM keychain_keys WHERE key_name LIKE 'test.%' OR key_name LIKE 'automation.%';
EOF
```

---

## Summary Checklist

### Migration Workflow
- [ ] Pre-migration state verified
- [ ] Test secret added to TOML
- [ ] Migration wizard completed
- [ ] Backup created
- [ ] Secrets cleared from TOML
- [ ] Keychain status shows migrated secrets

### Touch ID Behavior
- [ ] Keychain retrieval works
- [ ] Session caching functional (no repeated prompts)
- [ ] SSH detection works (graceful fallback)

### Security Level Management
- [ ] Current levels displayed correctly
- [ ] Upgrade security successful
- [ ] Downgrade security successful (with warning)
- [ ] Invalid levels rejected

### Set Secret Operations
- [ ] Set with standard level works
- [ ] Set with touchid level works
- [ ] Set with default level works

### Backward Compatibility
- [ ] TOML storage still works for non-secrets
- [ ] Mixed storage (keychain + TOML) works
- [ ] Existing functionality unaffected

### Error Handling
- [ ] Non-existent key errors handled
- [ ] Invalid security levels rejected
- [ ] Migration cancellation works

### Database Integrity
- [ ] Database metadata correct
- [ ] Migration timestamps recorded
- [ ] No data corruption

---

## Known Limitations During Testing

⚠️ **Touch ID Native Prompts:** You won't see native macOS Touch ID dialogs due to `security-framework 2.9` API limitations. However:
- Keychain storage still works (OS-encrypted)
- Security levels are tracked
- Session caching functions
- All other features work as designed

**This is expected and documented** - full Touch ID prompts require FFI bindings (planned for v0.3.1).

---

## Test Results Template

```
Date: ___________
Tester: ___________
macOS Version: ___________
Touch ID Available: [ ] Yes [ ] No

Test Suite 1: [ ] Pass [ ] Fail [ ] Partial
Test Suite 2: [ ] Pass [ ] Fail [ ] Partial
Test Suite 3: [ ] Pass [ ] Fail [ ] Partial
Test Suite 4: [ ] Pass [ ] Fail [ ] Partial
Test Suite 5: [ ] Pass [ ] Fail [ ] Partial
Test Suite 6: [ ] Pass [ ] Fail [ ] Partial
Test Suite 7: [ ] Pass [ ] Fail [ ] Partial
Test Suite 8: [ ] Pass [ ] Fail [ ] Partial

Notes:
___________________________________________
___________________________________________
___________________________________________
```
