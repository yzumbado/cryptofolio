# Test 10: Configuration Basics

**Estimated Time:** 10 minutes
**Platform:** macOS, Linux, Windows
**Phase:** Core

---

## Overview

Verify basic configuration management commands (show, set, get) and TOML file operations.

---

## Prerequisites

- ✅ Binary compiled (Test 00 passed)
- ✅ Database initialized (Test 01 passed)
- ✅ Config directory created
- ✅ config.toml exists

---

## Test Steps

### 1. Show Current Configuration

```bash
./target/release/cryptofolio config show
```

**Expected Output:**
```
Configuration File: /Users/yzumbado/Library/Application Support/cryptofolio/config.toml

[general]
default_currency = "USD"
base_currency = "USD"

[binance]
api_key = "..." (if set)

[display]
decimals = 8
date_format = "%Y-%m-%d %H:%M:%S"
```

**Validation:**
- ✅ Config file path displayed
- ✅ All sections shown
- ✅ Values formatted correctly
- ✅ Secrets masked (if any)
- ✅ No errors

---

### 2. Get Specific Configuration Value

```bash
# Get default currency
./target/release/cryptofolio config get general.default_currency

# Get decimal places
./target/release/cryptofolio config get display.decimals

# Get non-existent key
./target/release/cryptofolio config get nonexistent.key
```

**Expected Results:**

**Existing key:**
```
USD
```

**Non-existent key:**
```
[ERROR] Key 'nonexistent.key' not found in configuration
```

**Validation:**
- ✅ Existing keys return values
- ✅ Non-existent keys show clear error
- ✅ No crash on invalid keys
- ✅ Output is clean (value only)

---

### 3. Set Configuration Value

```bash
# Set default currency
./target/release/cryptofolio config set general.default_currency EUR

# Verify change
./target/release/cryptofolio config get general.default_currency

# Set custom value
./target/release/cryptofolio config set display.decimals 6

# Verify
./target/release/cryptofolio config show
```

**Expected Results:**
- ✅ Set command succeeds
- ✅ Value persists in config.toml
- ✅ Get returns new value
- ✅ Show displays updated value

**Verification in TOML:**
```bash
cat ~/Library/Application\ Support/cryptofolio/config.toml | grep default_currency
# Should show: default_currency = "EUR"
```

---

### 4. Set Nested Configuration

```bash
# Create new section with value
./target/release/cryptofolio config set custom.test_value "123"

# Set another value in same section
./target/release/cryptofolio config set custom.another_value "abc"

# Verify both
./target/release/cryptofolio config show
```

**Expected in config.toml:**
```toml
[custom]
test_value = "123"
another_value = "abc"
```

**Validation:**
- ✅ New sections created automatically
- ✅ Multiple values in same section work
- ✅ TOML structure valid

---

### 5. Set Number and Boolean Values

```bash
# Set integer
./target/release/cryptofolio config set test.number 42

# Set boolean
./target/release/cryptofolio config set test.flag true

# Set decimal
./target/release/cryptofolio config set test.decimal 3.14159

# Verify types preserved
cat ~/Library/Application\ Support/cryptofolio/config.toml
```

**Expected TOML:**
```toml
[test]
number = 42
flag = true
decimal = 3.14159
```

**Validation:**
- ✅ Numbers stored without quotes
- ✅ Booleans as true/false (not strings)
- ✅ Decimals preserve precision
- ✅ Type inference working

---

### 6. Handle Special Characters

```bash
# Set value with spaces
./target/release/cryptofolio config set test.with_spaces "Hello World"

# Set value with quotes
./target/release/cryptofolio config set test.with_quotes "He said \"Hello\""

# Set value with special chars
./target/release/cryptofolio config set test.special "user@example.com"

# Verify
./target/release/cryptofolio config get test.with_spaces
./target/release/cryptofolio config get test.special
```

**Expected Results:**
- ✅ Spaces preserved: `Hello World`
- ✅ Quotes escaped properly
- ✅ Special chars (@ . -) handled
- ✅ No TOML parsing errors

---

### 7. Update Existing Value

```bash
# Get initial value
./target/release/cryptofolio config get general.default_currency

# Update to new value
./target/release/cryptofolio config set general.default_currency GBP

# Verify update
./target/release/cryptofolio config get general.default_currency
# Should show: GBP

# Check config file directly
grep default_currency ~/Library/Application\ Support/cryptofolio/config.toml
# Should show only one entry (old value replaced)
```

**Validation:**
- ✅ Old value replaced (not duplicated)
- ✅ New value persists
- ✅ No duplicate keys in TOML

---

### 8. Configuration File Permissions

```bash
# Check file permissions (Unix/macOS only)
ls -l ~/Library/Application\ Support/cryptofolio/config.toml

# Expected: -rw------- or -rw-r--r--
# Ideal: 600 (owner read/write only)
```

**Validation:**
- ✅ File owned by current user
- ✅ Not world-writable (no 'w' in last group)
- ✅ Readable by application
- ⚠️ Ideally 600 for security (especially with secrets)

---

### 9. Invalid Key Handling

```bash
# Try to set invalid key format
./target/release/cryptofolio config set invalid-key value 2>&1

# Try to set empty key
./target/release/cryptofolio config set "" value 2>&1

# Try to get from invalid section
./target/release/cryptofolio config get ...invalid 2>&1
```

**Expected Results:**
- ✅ Clear error messages
- ✅ No crash
- ✅ No config corruption
- ✅ Helpful suggestions

**Example Error:**
```
[ERROR] Invalid key format. Use format: section.key
```

---

### 10. TOML Syntax Validation

```bash
# Manually add invalid TOML to test parser
# DO NOT DO THIS - just verify error handling exists

# Instead, verify valid TOML
cat ~/Library/Application\ Support/cryptofolio/config.toml

# Should parse without errors
./target/release/cryptofolio config show
```

**Validation:**
- ✅ Valid TOML syntax throughout
- ✅ All sections properly formed
- ✅ All strings properly quoted
- ✅ All arrays/tables valid
- ✅ No trailing commas
- ✅ No syntax errors

---

## Cleanup

```bash
# Remove test values
./target/release/cryptofolio config set general.default_currency USD
./target/release/cryptofolio config set display.decimals 8

# You may need to manually remove custom sections from TOML:
# Edit: ~/Library/Application Support/cryptofolio/config.toml
# Remove [custom], [test] sections if present
```

Or use a text editor:
```bash
# macOS
open -a TextEdit ~/Library/Application\ Support/cryptofolio/config.toml

# Linux
nano ~/.config/cryptofolio/config.toml
```

---

## Validation Checklist

### Config Show Command
- [ ] Displays config file path
- [ ] Shows all configuration sections
- [ ] Values formatted correctly
- [ ] Secrets masked/hidden
- [ ] No errors on valid config

### Config Get Command
- [ ] Returns value for existing keys
- [ ] Clear error for non-existent keys
- [ ] No crash on invalid keys
- [ ] Output is clean (value only, no extra text)

### Config Set Command
- [ ] Sets new values successfully
- [ ] Updates existing values (no duplication)
- [ ] Creates new sections automatically
- [ ] Persists changes to TOML file
- [ ] Handles strings, numbers, booleans
- [ ] Handles special characters

### File Operations
- [ ] TOML file created if missing
- [ ] TOML syntax always valid
- [ ] File permissions appropriate (600 or 644)
- [ ] No corruption after operations
- [ ] Changes persist across restarts

### Error Handling
- [ ] Invalid keys rejected
- [ ] Empty keys rejected
- [ ] Helpful error messages
- [ ] No crashes on invalid input
- [ ] Config file not corrupted on error

---

## Common Issues

### Issue 1: Permission Denied
**Symptoms:** Cannot write to config.toml
**Solution:** Check file permissions
**Action:**
```bash
chmod 600 ~/Library/Application\ Support/cryptofolio/config.toml
```

### Issue 2: TOML Parse Error
**Symptoms:** "Failed to parse config" error
**Solution:** Check for syntax errors in TOML
**Action:**
```bash
# Backup current config
cp config.toml config.toml.backup

# Validate TOML (if toml tool installed)
# Or manually inspect for syntax errors
```

### Issue 3: Value Not Persisting
**Symptoms:** Set command succeeds but value doesn't save
**Solution:** Check disk space, file locks
**Action:**
```bash
df -h  # Check disk space
lsof config.toml  # Check for locks
```

### Issue 4: Unexpected Value Type
**Symptoms:** Number stored as string
**Solution:** Ensure no quotes around numbers
**Action:**
```bash
# Instead of:
config set test.number "42"  # Wrong - quotes make it a string

# Use:
config set test.number 42    # Correct - stored as integer
```

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**Config File Location:** _______________

**Tests Passed:**
- [ ] Show command
- [ ] Get command (existing key)
- [ ] Get command (non-existent key)
- [ ] Set command (new value)
- [ ] Set command (update value)
- [ ] Nested configuration
- [ ] Type handling (string, number, boolean)
- [ ] Special characters
- [ ] File permissions
- [ ] Error handling

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

**Next Test:** [11-config-keychain.md](11-config-keychain.md)
