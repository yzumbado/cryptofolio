# Test 00: Installation & First Run

**Estimated Time:** 5 minutes
**Platform:** macOS, Linux, Windows
**Phase:** Core

---

## Overview

Verify binary compilation, installation, and first-run experience.

---

## Prerequisites

- ✅ Rust toolchain installed (`rustc --version`)
- ✅ Cargo installed (`cargo --version`)
- ✅ Git repository cloned
- ✅ Terminal access

---

## Test Steps

### 1. Clean Build Environment

```bash
# Remove any previous builds
cargo clean

# Verify clean state
ls target/ 2>/dev/null || echo "Clean state confirmed"
```

**Expected:**
- `target/` directory removed or doesn't exist
- Ready for fresh build

---

### 2. Compile Debug Binary

```bash
# Compile debug build
cargo build

# Check compilation time and result
echo "Exit code: $?"
```

**Expected:**
- ✅ Compilation succeeds (exit code 0)
- ✅ No errors
- ⚠️ Warnings acceptable (sqlx offline mode warnings expected)
- ✅ Build time: ~5-10 seconds (incremental)
- ✅ Binary created: `target/debug/cryptofolio`

**Common Issues:**
- `DATABASE_URL` errors: Expected (using offline mode)
- Permission errors: Check directory permissions

---

### 3. Compile Release Binary

```bash
# Compile optimized release build
cargo build --release

# Verify binary exists
ls -lh target/release/cryptofolio

# Check binary size
du -h target/release/cryptofolio
```

**Expected:**
- ✅ Compilation succeeds
- ✅ Binary exists: `target/release/cryptofolio`
- ✅ Binary size: ~5-10 MB
- ✅ Executable permissions set

---

### 4. First Run - Version Check

```bash
# Check version
./target/release/cryptofolio --version
```

**Expected Output:**
```
cryptofolio 0.2.0
```

**Validation:**
- ✅ Version displayed correctly
- ✅ No crash or error
- ✅ Clean exit (exit code 0)

---

### 5. First Run - Help Command

```bash
# Display help
./target/release/cryptofolio --help
```

**Expected Output:**
```
A CLI tool for managing crypto portfolios across exchanges and wallets

Usage: cryptofolio [OPTIONS] <COMMAND>

Commands:
  price       Get current price for one or more cryptocurrencies
  market      Get detailed market data for a cryptocurrency
  account     Manage accounts (exchanges, wallets)
  category    Manage categories for organizing accounts
  holdings    Manage holdings across accounts
  portfolio   View portfolio with P&L calculations
  tx          Manage transactions
  sync        Sync holdings from exchange APIs
  config      Manage configuration settings
  help        Print this message or the help of the given subcommand(s)
...
```

**Validation:**
- ✅ Help text displayed
- ✅ All major commands listed (price, account, portfolio, tx, config, etc.)
- ✅ Options explained
- ✅ Examples shown (if present)

---

### 6. First Run - Config Show

```bash
# Show configuration (creates config if doesn't exist)
./target/release/cryptofolio config show
```

**Expected:**
- ✅ Config file created (if first run)
- ✅ Default configuration displayed
- ✅ Config directory created
- ✅ Database initialized

**Default Config Location:**
- macOS: `~/Library/Application Support/cryptofolio/`
- Linux: `~/.config/cryptofolio/`
- Windows: `%APPDATA%\cryptofolio\`

---

### 7. Verify Installation Directories

```bash
# macOS/Linux
ls -la ~/Library/Application\ Support/cryptofolio/ 2>/dev/null || \
ls -la ~/.config/cryptofolio/ 2>/dev/null

# Check contents
find ~/Library/Application\ Support/cryptofolio/ -type f 2>/dev/null || \
find ~/.config/cryptofolio/ -type f 2>/dev/null
```

**Expected Files:**
```
config.toml           # Configuration file
database.sqlite       # SQLite database
```

**Validation:**
- ✅ Config directory created
- ✅ `config.toml` exists
- ✅ `database.sqlite` exists
- ✅ Proper permissions (600 for config.toml on Unix)

---

### 8. Binary Location & PATH

```bash
# Check if binary can be run from anywhere
which cryptofolio 2>/dev/null || echo "Not in PATH"

# Recommend installation
echo ""
echo "To install globally:"
echo "  cargo install --path ."
echo ""
echo "Or create symlink:"
echo "  sudo ln -s $(pwd)/target/release/cryptofolio /usr/local/bin/"
```

**Expected:**
- Binary works from repo directory
- Installation instructions displayed

---

### 9. Environment Detection

```bash
# Check platform detection
./target/release/cryptofolio config show | grep -i "config_dir"
```

**Expected:**
- ✅ Correct platform detected
- ✅ Proper config directory for OS
- ✅ Paths displayed correctly

---

### 10. Quick Functional Test

```bash
# Test a simple command to verify basic functionality
./target/release/cryptofolio account list

# Should show empty or existing accounts
```

**Expected:**
- ✅ Command executes
- ✅ Output formatted correctly (table or "no accounts" message)
- ✅ No crashes or errors

---

## Cleanup

None required - all steps are non-destructive.

---

## Validation Checklist

### Build
- [ ] `cargo build` succeeds
- [ ] `cargo build --release` succeeds
- [ ] No compilation errors
- [ ] Binary created successfully
- [ ] Binary has executable permissions

### First Run
- [ ] `--version` displays version
- [ ] `--help` displays help text
- [ ] `config show` runs successfully
- [ ] Config directory created
- [ ] Database initialized

### Environment
- [ ] Platform detected correctly
- [ ] Config paths appropriate for OS
- [ ] File permissions correct (Unix)

### Functionality
- [ ] Basic command works (`account list`)
- [ ] No crashes on first run
- [ ] Clean error messages (if any)

---

## Common Issues

### Issue 1: sqlx DATABASE_URL Warnings
**Symptoms:** Warnings about DATABASE_URL during compilation
**Solution:** Expected behavior - using offline mode
**Action:** Ignore warnings, verify build completes

### Issue 2: Permission Denied
**Symptoms:** Cannot create config directory
**Solution:** Check write permissions
**Action:** `chmod` or use different directory

### Issue 3: Binary Not Found After Build
**Symptoms:** `target/release/cryptofolio` doesn't exist
**Solution:** Check build output for errors
**Action:** Re-run `cargo build --release`

### Issue 4: Database Lock Error
**Symptoms:** "Database is locked" on first run
**Solution:** Concurrent access or stale lock
**Action:** Close other instances, remove `database.sqlite-journal`

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**Platform:** macOS / Linux / Windows
**Rust Version:** _______________

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

**Next Test:** [01-database-setup.md](01-database-setup.md)
