# Test 01: Database Setup & Migrations

**Estimated Time:** 5 minutes
**Platform:** macOS, Linux, Windows
**Phase:** Core

---

## Overview

Verify database initialization, migration system, and schema integrity.

---

## Prerequisites

- ✅ Binary compiled (Test 00 passed)
- ✅ First run completed
- ✅ Config directory created
- ✅ Database file exists

---

## Test Steps

### 1. Verify Database Location

```bash
# macOS
ls -lh ~/Library/Application\ Support/cryptofolio/database.sqlite

# Linux
ls -lh ~/.config/cryptofolio/database.sqlite

# Check file size (should be >100 KB with migrations)
du -h ~/Library/Application\ Support/cryptofolio/database.sqlite
```

**Expected Results:**
- ✅ Database file exists
- ✅ File size: ~150-200 KB (with schema)
- ✅ Readable by user
- ✅ File permissions: 644 or 600

---

### 2. Check Database Version

```bash
# Connect to database
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.headers on
.mode column
SELECT * FROM _migrations ORDER BY version;
EOF
```

**Expected Output:**
```
version
----------
1
2
3
5
```

**Validation:**
- ✅ Migration table exists
- ✅ Migrations 1, 2, 3, 5 applied
- ✅ Migration 4 skipped (P&L - Phase 3)
- ✅ No duplicate versions

---

### 3. Verify Core Tables Schema

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.schema accounts
.schema categories
.schema holdings
.schema transactions
EOF
```

**Expected Tables:**
- ✅ `accounts` - Exchange/wallet accounts
- ✅ `categories` - Account categorization
- ✅ `holdings` - Asset holdings
- ✅ `transactions` - Transaction history

**Key Columns to Verify:**

**accounts:**
- id (TEXT PRIMARY KEY)
- name (TEXT NOT NULL)
- account_type (TEXT)
- category_id (TEXT REFERENCES categories)
- sync_enabled (BOOLEAN)

**transactions:**
- id (INTEGER PRIMARY KEY)
- account_id (TEXT REFERENCES accounts)
- tx_type (TEXT CHECK - buy/sell/swap/transfer)
- asset (TEXT)
- quantity (TEXT)
- price (TEXT)
- tx_date (DATETIME)

---

### 4. Verify Keychain Table (Phase 2)

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.schema keychain_keys
EOF
```

**Expected Schema:**
```sql
CREATE TABLE keychain_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_name TEXT NOT NULL UNIQUE,
    storage_type TEXT NOT NULL CHECK(storage_type IN ('keychain', 'toml', 'env')),
    security_level TEXT CHECK(security_level IN ('standard', 'touchid', 'touchid-only')),
    last_accessed DATETIME,
    migrated_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**Validation:**
- ✅ Table exists (MIGRATION_005 applied)
- ✅ CHECK constraints on storage_type
- ✅ CHECK constraints on security_level
- ✅ UNIQUE constraint on key_name
- ✅ Index on key_name exists

---

### 5. Check Indexes

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
.indexes
EOF
```

**Expected Indexes:**
- ✅ `idx_accounts_category` (on accounts.category_id)
- ✅ `idx_holdings_account` (on holdings.account_id)
- ✅ `idx_holdings_asset` (on holdings.asset)
- ✅ `idx_transactions_account` (on transactions.account_id)
- ✅ `idx_transactions_date` (on transactions.tx_date)
- ✅ `idx_keychain_keys_name` (on keychain_keys.key_name)

**Validation:**
- ✅ All expected indexes present
- ✅ No duplicate indexes
- ✅ Performance-critical columns indexed

---

### 6. Verify Foreign Key Constraints

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
PRAGMA foreign_keys = ON;
PRAGMA foreign_key_check;
EOF
```

**Expected:**
- ✅ No output (all foreign keys valid)
- ✅ Foreign keys enabled

**If errors appear:**
```
table       rowid       parent      fkid
----------  ----------  ----------  ----------
```
Should be empty!

---

### 7. Test Database Integrity

```bash
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
PRAGMA integrity_check;
EOF
```

**Expected Output:**
```
ok
```

**Validation:**
- ✅ No corruption detected
- ✅ All pages valid
- ✅ B-tree structures intact

---

### 8. Query Sample Data

```bash
# Check if any data exists
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
SELECT COUNT(*) as account_count FROM accounts;
SELECT COUNT(*) as category_count FROM categories;
SELECT COUNT(*) as holding_count FROM holdings;
SELECT COUNT(*) as transaction_count FROM transactions;
SELECT COUNT(*) as keychain_count FROM keychain_keys;
EOF
```

**Expected:**
- Counts may be 0 (fresh install) or >0 (existing data)
- No errors during query execution
- All tables queryable

---

### 9. Test Write Operations

```bash
# Test category insert
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
INSERT INTO categories (id, name) VALUES ('test-cat', 'Test Category');
SELECT * FROM categories WHERE id = 'test-cat';
DELETE FROM categories WHERE id = 'test-cat';
EOF
```

**Expected:**
- ✅ INSERT succeeds
- ✅ SELECT returns row
- ✅ DELETE succeeds
- ✅ No constraints violated

---

### 10. Migration Idempotency Test

```bash
# Re-run migrations (should be no-op due to IF NOT EXISTS)
cd /Users/yzumbado/projects/cryptofolio

# Check current state
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite \
  "SELECT COUNT(*) FROM _migrations"

# Restart app (triggers migration check)
./target/release/cryptofolio config show > /dev/null

# Check migrations unchanged
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite \
  "SELECT COUNT(*) FROM _migrations"
```

**Expected:**
- ✅ Migration count unchanged
- ✅ No duplicate migrations applied
- ✅ No errors during startup
- ✅ Database schema unchanged

---

## Cleanup

```bash
# Remove test data (if any was created)
sqlite3 ~/Library/Application\ Support/cryptofolio/database.sqlite << EOF
DELETE FROM categories WHERE id LIKE 'test-%';
DELETE FROM accounts WHERE id LIKE 'test-%';
EOF
```

---

## Validation Checklist

### Database File
- [ ] Database file exists at correct location
- [ ] File size appropriate (~150-200 KB)
- [ ] File permissions correct (600 or 644)
- [ ] File readable by application

### Migration System
- [ ] `_migrations` table exists
- [ ] Migrations 1, 2, 3, 5 applied
- [ ] No duplicate migrations
- [ ] Idempotent (re-run safe)

### Schema Integrity
- [ ] All core tables exist (accounts, categories, holdings, transactions)
- [ ] keychain_keys table exists (Phase 2)
- [ ] All expected columns present
- [ ] CHECK constraints working
- [ ] UNIQUE constraints working
- [ ] NOT NULL constraints working

### Indexes
- [ ] All expected indexes present
- [ ] No duplicate indexes
- [ ] Index names follow convention

### Foreign Keys
- [ ] Foreign key constraints defined
- [ ] Foreign key check passes
- [ ] Referential integrity maintained

### Database Health
- [ ] PRAGMA integrity_check passes
- [ ] No corruption detected
- [ ] All tables queryable
- [ ] Write operations work
- [ ] Read operations work

---

## Common Issues

### Issue 1: Database Locked
**Symptoms:** "database is locked" error
**Solution:** Close other connections, remove `-journal` file
**Action:**
```bash
# Find processes using database
lsof ~/Library/Application\ Support/cryptofolio/database.sqlite

# Kill if safe, or wait for completion
```

### Issue 2: Missing Migrations
**Symptoms:** Table not found errors
**Solution:** Check migration log, manually apply if needed
**Action:**
```bash
# Check which migrations applied
sqlite3 database.sqlite "SELECT * FROM _migrations"

# Manually apply missing migration (if safe)
```

### Issue 3: Foreign Key Violations
**Symptoms:** FOREIGN KEY constraint failed
**Solution:** Check orphaned records
**Action:**
```bash
PRAGMA foreign_key_check;
# Delete orphaned records or fix references
```

### Issue 4: Corrupted Database
**Symptoms:** "database disk image is malformed"
**Solution:** Restore from backup or export/reimport
**Action:**
```bash
# Export data
sqlite3 database.sqlite .dump > backup.sql

# Recreate database
rm database.sqlite
cryptofolio config show  # Recreates

# Reimport (if needed)
sqlite3 database.sqlite < backup.sql
```

---

## Test Result

**Status:** [ ] PASS [ ] FAIL [ ] PARTIAL

**Tested by:** _______________
**Date:** _______________
**Database Version:** _______________ (from _migrations table)
**Tables Found:** _______________

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

**Next Test:** [10-config-basics.md](10-config-basics.md)
