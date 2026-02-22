# Cryptofolio v0.3.0 - Validation Test Suite

## Overview

This directory contains structured validation tests organized by functional area. Each test file is designed to be executed independently for thorough manual validation before code commits.

---

## Test Suite Structure

### 📦 Installation & Setup
- **00-installation.md** - Binary installation, first run, environment setup
- **01-database-setup.md** - Database initialization, migrations, schema verification

### ⚙️ Configuration Management
- **10-config-basics.md** - Basic config commands, show/set, TOML operations
- **11-config-keychain.md** - Keychain integration (Phase 2), migration, security levels

### 📊 Portfolio Management
- **20-accounts.md** - Account CRUD operations, categories
- **21-holdings.md** - Holdings management, manual entry, updates
- **22-transactions.md** - Transaction recording, buy/sell/swap/transfer

### 🔄 Exchange Integration
- **30-binance-sync.md** - Binance API sync, authentication, data import
- **31-market-data.md** - Price fetching, market data, currency support

### 📈 Reporting & Export (Phase 1)
- **40-export-formats.md** - CSV/JSON/SQL export functionality
- **41-portfolio-views.md** - Portfolio display, P&L views, summaries

### 💰 Tax & P&L (Phase 1 Foundation)
- **50-tax-lots.md** - Tax lot database verification (foundation only)
- **51-realized-pnl.md** - Realized P&L database verification (foundation only)

### 🔐 Security & Keychain (Phase 2)
- **60-keychain-migration.md** - TOML → Keychain migration workflow
- **61-keychain-operations.md** - Set secrets, retrieve, security levels
- **62-keychain-security.md** - Security level management, upgrade/downgrade

### 🧪 Error Handling & Edge Cases
- **70-error-handling.md** - Error messages, validation, recovery
- **71-edge-cases.md** - Boundary conditions, special characters, large datasets

### 🔙 Backward Compatibility
- **80-backward-compat.md** - v0.2.0 feature compatibility, data migration

---

## Test Execution Order

### Quick Validation (30 minutes)
Essential tests for basic functionality:
```
1. 00-installation.md          (5 min)
2. 01-database-setup.md         (5 min)
3. 10-config-basics.md          (5 min)
4. 11-config-keychain.md        (10 min)
5. 40-export-formats.md         (5 min)
```

### Standard Validation (90 minutes)
Comprehensive validation for release:
```
All tests in order (00-80)
```

### Phase 2 Focus (45 minutes)
Keychain-specific validation:
```
1. 00-installation.md           (5 min)
2. 01-database-setup.md         (5 min)
3. 10-config-basics.md          (5 min)
4. 11-config-keychain.md        (10 min)
5. 60-keychain-migration.md     (10 min)
6. 61-keychain-operations.md    (5 min)
7. 62-keychain-security.md      (5 min)
```

---

## Test Result Tracking

Each test file includes:
- ✅ **Prerequisites** - What must be ready before testing
- 📋 **Test Steps** - Detailed step-by-step instructions
- ✔️ **Expected Results** - What should happen
- 🐛 **Common Issues** - Known problems and solutions
- ✅ **Checklist** - Quick pass/fail tracking

### Test Status Legend
- ✅ **PASS** - Test completed successfully
- ❌ **FAIL** - Test failed, issue found
- ⚠️ **PARTIAL** - Test passed with known limitations
- ⏭️ **SKIP** - Test not applicable or skipped
- 🔄 **RETEST** - Test needs to be run again

---

## Pre-Commit Validation Checklist

Before committing Phase 2 code, complete:

### Critical Path (Must Pass)
- [ ] 00-installation.md
- [ ] 01-database-setup.md
- [ ] 10-config-basics.md
- [ ] 11-config-keychain.md
- [ ] 60-keychain-migration.md
- [ ] 61-keychain-operations.md
- [ ] 62-keychain-security.md
- [ ] 80-backward-compat.md

### Recommended (Should Pass)
- [ ] 20-accounts.md
- [ ] 21-holdings.md
- [ ] 22-transactions.md
- [ ] 40-export-formats.md
- [ ] 70-error-handling.md

### Optional (Nice to Have)
- [ ] 30-binance-sync.md (requires API keys)
- [ ] 31-market-data.md (requires network)
- [ ] 71-edge-cases.md (extended testing)

---

## Environment Setup

### Requirements
- macOS (for keychain testing)
- Rust 1.70+ (for compilation)
- SQLite 3.x (database)
- Optional: Binance API keys (for sync testing)

### Test Database
Each test uses: `~/.config/cryptofolio/database.sqlite` (or Library/Application Support)

**⚠️ Warning:** Some tests modify the database. Back up important data first!

### Backup Before Testing
```bash
# Backup config
cp -r ~/.config/cryptofolio ~/.config/cryptofolio.backup

# Or on macOS
cp -r ~/Library/Application\ Support/cryptofolio ~/Library/Application\ Support/cryptofolio.backup
```

---

## Test Artifacts

### Generated Files
- `validation/results/` - Test execution results
- `validation/logs/` - Detailed logs
- `validation/screenshots/` - UI screenshots (if applicable)

### Result Template
Each test creates a result file:
```
validation/results/YYYYMMDD-HHMMSS-{test-name}.md
```

Example:
```
validation/results/20260221-153000-config-keychain.md
```

---

## Contributing Test Cases

### Adding New Tests
1. Choose appropriate functional area (00-80)
2. Use template: `validation/templates/test-template.md`
3. Follow naming convention: `{number}-{area}.md`
4. Update this README with new test

### Test Template Structure
```markdown
# Test: {Name}

## Overview
Brief description

## Prerequisites
- Requirement 1
- Requirement 2

## Test Steps
1. Step 1
2. Step 2

## Expected Results
- Result 1
- Result 2

## Cleanup
Steps to reset

## Checklist
- [ ] Test passed
- [ ] Issues documented
```

---

## Quick Reference

### Run All Tests
```bash
cd validation
./run-all-tests.sh
```

### Run Single Test
```bash
cd validation
./run-test.sh 11-config-keychain
```

### View Results
```bash
cd validation/results
ls -lt | head -10
```

---

## Support

### Issues During Testing
1. Check `validation/TROUBLESHOOTING.md`
2. Review test's "Common Issues" section
3. Check Phase 2 documentation
4. File issue with test result attached

### Test Feedback
Improvements welcome! Update tests as you find better validation approaches.

---

**Last Updated:** February 21, 2026
**Test Suite Version:** v0.3.0
**Status:** Phase 2 Complete, Ready for Validation
