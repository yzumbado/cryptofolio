# Proposed Validation Structure for Cryptofolio v0.3.0

## 📋 Overview

Restructured validation testing into **15 focused test files** organized by functional area, replacing the single monolithic testing guide.

---

## 🗂️ Proposed File Structure

```
cryptofolio/
├── validation/
│   ├── README.md                        ✅ Created (master index)
│   │
│   ├── 00-installation.md               🆕 Installation & First Run
│   ├── 01-database-setup.md             🆕 Database & Migrations
│   │
│   ├── 10-config-basics.md              🆕 Basic Configuration
│   ├── 11-config-keychain.md            🆕 Keychain Integration
│   │
│   ├── 20-accounts.md                   🆕 Account Management
│   ├── 21-holdings.md                   🆕 Holdings Management
│   ├── 22-transactions.md               🆕 Transaction Recording
│   │
│   ├── 30-binance-sync.md               🆕 Binance Integration
│   ├── 31-market-data.md                🆕 Market Data & Prices
│   │
│   ├── 40-export-formats.md             🆕 Export (CSV/JSON/SQL)
│   ├── 41-portfolio-views.md            🆕 Portfolio Display
│   │
│   ├── 50-tax-lots.md                   🆕 Tax Lots (Foundation)
│   ├── 51-realized-pnl.md               🆕 Realized P&L (Foundation)
│   │
│   ├── 60-keychain-migration.md         🆕 Keychain Migration
│   ├── 61-keychain-operations.md        🆕 Keychain Operations
│   ├── 62-keychain-security.md          🆕 Security Levels
│   │
│   ├── 70-error-handling.md             🆕 Error Handling
│   ├── 71-edge-cases.md                 🆕 Edge Cases
│   │
│   ├── 80-backward-compat.md            🆕 Backward Compatibility
│   │
│   ├── templates/
│   │   └── test-template.md             🆕 Template for new tests
│   │
│   ├── results/                         📁 Test execution results
│   │   └── .gitkeep
│   │
│   ├── logs/                            📁 Detailed logs
│   │   └── .gitkeep
│   │
│   └── scripts/
│       ├── run-all-tests.sh             🆕 Run all tests
│       ├── run-test.sh                  🆕 Run single test
│       └── generate-report.sh           🆕 Generate summary report
│
└── docs/
    └── testing/                         📁 Additional test documentation
        ├── PHASE1_TESTS.md              🔄 Existing Phase 1 tests
        ├── PHASE2_TESTS.md              🔄 Existing Phase 2 tests
        └── TROUBLESHOOTING.md           🆕 Common issues & solutions
```

---

## 📚 Detailed File Descriptions

### 🔧 Installation & Setup (00-01)

#### **00-installation.md**
**Purpose:** Verify installation and first-run experience
**Contents:**
- Binary compilation (`cargo build --release`)
- Binary location verification
- First run initialization
- Help command validation
- Version check
- Environment detection (macOS/Linux/etc.)

**Estimated Time:** 5 minutes
**Prerequisites:** Rust toolchain installed

---

#### **01-database-setup.md**
**Purpose:** Verify database initialization and migrations
**Contents:**
- Database file creation
- Migration execution (1, 2, 3, 5)
- Schema verification (12 tables)
- Index verification
- Foreign key constraints
- Default data seeding
- Database location check

**Estimated Time:** 5 minutes
**Prerequisites:** Binary compiled

---

### ⚙️ Configuration Management (10-11)

#### **10-config-basics.md**
**Purpose:** Test basic configuration operations
**Contents:**
- `config show` - Display current config
- `config set` - Set non-secret values
- TOML file creation
- File permissions (0600 on Unix)
- Config directory creation
- Multiple config values
- Invalid key handling
- Boolean/number validation

**Estimated Time:** 5 minutes
**Tests:** 8-10 test cases

---

#### **11-config-keychain.md**
**Purpose:** Test keychain integration basics
**Contents:**
- Keychain availability detection
- Platform detection (macOS vs others)
- `keychain-status` command (empty state)
- `set-secret` with keychain
- Secret retrieval from keychain
- Session caching behavior
- SSH session detection
- Fallback to TOML (non-macOS)

**Estimated Time:** 10 minutes
**Tests:** 10-12 test cases
**Platform:** macOS for full testing

---

### 📊 Portfolio Management (20-22)

#### **20-accounts.md**
**Purpose:** Test account CRUD operations
**Contents:**
- `account add` - Create accounts
- `account list` - List all accounts
- `account show` - Display details
- `account update` - Modify account
- `account delete` - Remove account
- Category assignment
- Account types (exchange, wallet, bank)
- Sync flag management
- JSON output

**Estimated Time:** 10 minutes
**Tests:** 12-15 test cases

---

#### **21-holdings.md**
**Purpose:** Test holdings management
**Contents:**
- `holdings add` - Manual entry
- `holdings list` - Display holdings
- `holdings update` - Modify quantities
- `holdings delete` - Remove holdings
- Cost basis tracking
- Multi-currency support
- Account filtering
- Asset filtering
- Zero balance handling

**Estimated Time:** 10 minutes
**Tests:** 10-12 test cases

---

#### **22-transactions.md**
**Purpose:** Test transaction recording
**Contents:**
- `tx buy` - Record purchases
- `tx sell` - Record sales
- `tx swap` - Record exchanges
- `tx transfer` - Record transfers
- `tx list` - Display history
- `tx show` - Transaction details
- Fee recording
- Price tracking
- Date/time handling
- External ID tracking

**Estimated Time:** 15 minutes
**Tests:** 15-20 test cases

---

### 🔄 Exchange Integration (30-31)

#### **30-binance-sync.md**
**Purpose:** Test Binance API integration
**Contents:**
- API key configuration
- Authentication testing
- `sync --account Binance`
- Holdings sync
- Balance verification
- Testnet vs mainnet
- Rate limiting
- Error handling (invalid keys)
- Sync history

**Estimated Time:** 10 minutes
**Tests:** 8-10 test cases
**Prerequisites:** Binance API keys (testnet recommended)

---

#### **31-market-data.md**
**Purpose:** Test market data fetching
**Contents:**
- `price BTC` - Single price
- `price BTC ETH SOL` - Multiple prices
- `market BTC` - Market data
- 24-hour statistics
- Currency support (USD, CRC, EUR)
- Exchange rate handling
- Offline behavior
- JSON output

**Estimated Time:** 5 minutes
**Tests:** 6-8 test cases
**Prerequisites:** Network connection

---

### 📈 Reporting & Export (40-41)

#### **40-export-formats.md** (Phase 1)
**Purpose:** Test export functionality
**Contents:**
- `tx export file.csv` - CSV export
- `tx export file.json --format json` - JSON export
- `tx export file.sql --format sql` - SQL export
- Account filtering (`--account`)
- Asset filtering (`--asset`)
- Date range filtering (`--from`, `--to`)
- Limit parameter
- Large dataset handling (1000+ transactions)
- SQL import verification

**Estimated Time:** 10 minutes
**Tests:** 12-15 test cases

---

#### **41-portfolio-views.md**
**Purpose:** Test portfolio display
**Contents:**
- `portfolio` - Overall summary
- `portfolio --by-category` - Category breakdown
- `portfolio --by-asset` - Asset breakdown
- `portfolio --account` - Single account
- P&L calculations (unrealized)
- Multi-currency display
- JSON output
- Sorting options
- Zero balance filtering

**Estimated Time:** 10 minutes
**Tests:** 8-10 test cases

---

### 💰 Tax & P&L Foundation (50-51)

#### **50-tax-lots.md** (Phase 1 Foundation)
**Purpose:** Verify tax lot database infrastructure
**Contents:**
- Database schema verification
- `tax_lots` table exists
- Columns and types correct
- Indexes present
- Foreign keys working
- Check constraints (cost_basis_method)
- Repository layer (basic CRUD)
- **Note:** No calculation logic yet (Phase 3)

**Estimated Time:** 5 minutes
**Tests:** 6-8 test cases
**Scope:** Database/repository only

---

#### **51-realized-pnl.md** (Phase 1 Foundation)
**Purpose:** Verify realized P&L database infrastructure
**Contents:**
- Database schema verification
- `realized_pnl` table exists
- Columns and types correct
- Indexes present
- Foreign keys to tax_lots
- Repository layer (basic CRUD)
- **Note:** No calculation logic yet (Phase 3)

**Estimated Time:** 5 minutes
**Tests:** 6-8 test cases
**Scope:** Database/repository only

---

### 🔐 Security & Keychain (60-62)

#### **60-keychain-migration.md** (Phase 2)
**Purpose:** Test TOML → Keychain migration workflow
**Contents:**
- Pre-migration state check
- Add test secrets to TOML
- Run migration wizard
- Security level selection
- Confirmation prompts
- Backup creation (config.toml.backup)
- Secrets migrated to keychain
- Secrets cleared from TOML
- Database metadata updated
- Post-migration verification
- Rollback testing (cancel)

**Estimated Time:** 15 minutes
**Tests:** 10-12 test cases
**Platform:** macOS required

---

#### **61-keychain-operations.md** (Phase 2)
**Purpose:** Test keychain daily operations
**Contents:**
- `set-secret` with security levels
- Secret retrieval (automatic)
- `keychain-status` display
- JSON output (`--json`)
- Multiple secrets management
- Secret deletion
- macOS Keychain verification (`security` command)
- Session caching (15-min timeout)
- SSH session behavior
- Environment variable option (`--from-env`)
- File option (`--secret-file`)

**Estimated Time:** 10 minutes
**Tests:** 12-15 test cases

---

#### **62-keychain-security.md** (Phase 2)
**Purpose:** Test security level management
**Contents:**
- `upgrade-security` command
- `downgrade-security` command
- Standard → Touch ID Protected
- Touch ID Protected → Touch ID Only
- Touch ID Only → Standard (with warning)
- Invalid level rejection
- Non-existent key handling
- Database updates
- Security level verification
- Confirmation prompts
- Touch ID availability detection

**Estimated Time:** 10 minutes
**Tests:** 10-12 test cases
**Platform:** macOS required

---

### 🧪 Error Handling & Edge Cases (70-71)

#### **70-error-handling.md**
**Purpose:** Test error messages and recovery
**Contents:**
- Invalid commands
- Missing required parameters
- Non-existent accounts
- Non-existent assets
- Invalid amounts (negative, zero, non-numeric)
- Invalid dates
- Insufficient balance
- Database connection errors
- File permission errors
- Network errors (API calls)
- Helpful error messages
- Exit codes (0=success, 1=error, 2=usage)

**Estimated Time:** 15 minutes
**Tests:** 20-25 test cases

---

#### **71-edge-cases.md**
**Purpose:** Test boundary conditions
**Contents:**
- Very large quantities (1000000000 BTC)
- Very small quantities (0.00000001 BTC)
- Special characters in names
- Unicode support (₿, €, ₡)
- Long account names (>100 chars)
- Empty strings
- Null/None handling
- Date boundaries (year 2000, 2100)
- Large datasets (10000+ transactions)
- Concurrent operations
- Database locks

**Estimated Time:** 20 minutes
**Tests:** 15-20 test cases

---

### 🔙 Backward Compatibility (80)

#### **80-backward-compat.md**
**Purpose:** Ensure v0.2.0 features still work
**Contents:**
- Load v0.2.0 database
- Migrate v0.2.0 config
- All v0.2.0 commands functional
- Data integrity after migration
- No breaking changes
- API compatibility
- Configuration compatibility
- Export format compatibility
- Existing data readable
- Performance comparison

**Estimated Time:** 15 minutes
**Tests:** 10-12 test cases
**Prerequisites:** v0.2.0 sample data

---

## 🎯 Pre-Commit Validation Workflow

### Phase 2 Critical Path (60 minutes)

**Must complete before committing Phase 2:**

```bash
# 1. Installation & Setup (10 min)
✅ 00-installation.md
✅ 01-database-setup.md

# 2. Config Basics (15 min)
✅ 10-config-basics.md
✅ 11-config-keychain.md

# 3. Phase 2 Core (25 min)
✅ 60-keychain-migration.md
✅ 61-keychain-operations.md
✅ 62-keychain-security.md

# 4. Compatibility (10 min)
✅ 80-backward-compat.md
```

### Extended Validation (2 hours)

**Recommended before release:**

```bash
# All tests in order
00 → 01 → 10 → 11 → 20 → 21 → 22 → 30 → 31 →
40 → 41 → 50 → 51 → 60 → 61 → 62 → 70 → 71 → 80
```

---

## 📊 Test Coverage Matrix

| Feature Area | Test Files | Estimated Time | Phase |
|--------------|-----------|----------------|-------|
| Installation | 00-01 | 10 min | Core |
| Configuration | 10-11 | 15 min | Phase 2 |
| Portfolio | 20-22 | 35 min | v0.2 |
| Exchange | 30-31 | 15 min | v0.2 |
| Export | 40-41 | 20 min | Phase 1 |
| Tax Foundation | 50-51 | 10 min | Phase 1 |
| Keychain | 60-62 | 35 min | Phase 2 |
| Error/Edge | 70-71 | 35 min | Quality |
| Compatibility | 80 | 15 min | Quality |
| **TOTAL** | **19 files** | **~3 hours** | - |

---

## 🔄 Migration from Current Documentation

### Current Files → New Structure

```
PHASE2_TESTING_GUIDE.md → Split into:
  ├── 00-installation.md (new content)
  ├── 01-database-setup.md (new content)
  ├── 11-config-keychain.md (from Test Suite 1-2)
  ├── 60-keychain-migration.md (from Test Suite 1)
  ├── 61-keychain-operations.md (from Test Suite 3-4)
  ├── 62-keychain-security.md (from Test Suite 3)
  └── 70-error-handling.md (from Test Suite 6)

PHASE2_TEST_RESULTS.md → Keep as reference:
  └── validation/results/20260221-phase2-results.md

Existing v0.2 features → New tests:
  ├── 20-accounts.md (new)
  ├── 21-holdings.md (new)
  ├── 22-transactions.md (new)
  └── 30-binance-sync.md (new)
```

---

## 🚀 Next Steps

### 1. Create Test Files (Priority Order)

**High Priority (Pre-Commit):**
1. ✅ validation/README.md (Done)
2. 🆕 00-installation.md
3. 🆕 01-database-setup.md
4. 🆕 10-config-basics.md
5. 🆕 11-config-keychain.md
6. 🆕 60-keychain-migration.md
7. 🆕 61-keychain-operations.md
8. 🆕 62-keychain-security.md
9. 🆕 80-backward-compat.md

**Medium Priority (Pre-Release):**
10. 🆕 40-export-formats.md
11. 🆕 70-error-handling.md
12. 🆕 20-accounts.md
13. 🆕 21-holdings.md
14. 🆕 22-transactions.md

**Low Priority (Extended Testing):**
15. 🆕 30-binance-sync.md
16. 🆕 31-market-data.md
17. 🆕 41-portfolio-views.md
18. 🆕 50-tax-lots.md
19. 🆕 51-realized-pnl.md
20. 🆕 71-edge-cases.md

### 2. Create Support Files

- 🆕 templates/test-template.md
- 🆕 scripts/run-all-tests.sh
- 🆕 scripts/run-test.sh
- 🆕 scripts/generate-report.sh
- 🆕 TROUBLESHOOTING.md

### 3. Execute Validation

```bash
# Run critical path tests
./scripts/run-test.sh 00-installation
./scripts/run-test.sh 01-database-setup
./scripts/run-test.sh 10-config-basics
./scripts/run-test.sh 11-config-keychain
./scripts/run-test.sh 60-keychain-migration
./scripts/run-test.sh 61-keychain-operations
./scripts/run-test.sh 62-keychain-security
./scripts/run-test.sh 80-backward-compat

# Generate report
./scripts/generate-report.sh
```

---

## ✅ Benefits of New Structure

**1. Focused Testing**
- Each file covers one functional area
- Easier to locate specific tests
- Faster execution (run only what's needed)

**2. Better Organization**
- Logical grouping by feature
- Clear naming convention
- Easy to navigate

**3. Parallel Testing**
- Multiple testers can work simultaneously
- Different team members can own different areas
- Independent test execution

**4. Maintainability**
- Update one area without affecting others
- Add new tests easily
- Remove deprecated tests cleanly

**5. Progress Tracking**
- Clear checklist for pre-commit validation
- Track completion per functional area
- Generate focused reports

**6. Onboarding**
- New contributors can start with one area
- Clear test structure to follow
- Template for adding new tests

---

## 📅 Recommended Timeline

### Day 1: Create Test Files (4 hours)
- Create all 20 test files
- Populate with detailed test cases
- Create templates and scripts

### Day 2: Execute Critical Path (2 hours)
- Run tests 00, 01, 10, 11
- Run tests 60, 61, 62
- Run test 80
- Document results

### Day 3: Extended Validation (4 hours)
- Run remaining tests
- Document issues found
- Fix critical bugs
- Re-test fixes

### Day 4: Final Review (2 hours)
- Review all test results
- Update documentation
- Commit code with test results
- Tag release

**Total: ~12 hours** (spread over 4 days)

---

**Recommendation:** Start with creating the 8 critical path test files first, then execute validation before committing Phase 2 code.

Would you like me to create the test files in priority order?
