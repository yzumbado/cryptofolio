# Agentic Development: Building Cryptofolio with Claude Code

**How AI pair programming built a production-ready CLI application**

---

## Overview

Cryptofolio is a showcase of **agentic software development** - the practice of building software through collaboration with AI coding assistants. This document shares our development process, learnings, and metrics.

## What is Agentic Development?

Agentic development is software creation where:
- **AI acts as pair programmer** - Claude Code assists with design, implementation, testing
- **Human provides direction** - You define goals, review outputs, make decisions
- **Iterative refinement** - Continuous improvement through AI feedback
- **Quality maintained** - Tests, documentation, and best practices enforced

## Development Tools

### Claude Code
- **What:** Anthropic's official CLI for AI-assisted development
- **How:** Interactive terminal sessions with Claude
- **Why:** Direct codebase access, file editing, test execution

### Rust Ecosystem
- **Compiler:** Enforces type safety and memory safety
- **sqlx:** Compile-time SQL query validation
- **cargo test:** Comprehensive test suite

## Case Study: Multi-Currency Feature

Let's walk through how we built the multi-currency feature (v0.2) using agentic development.

### Phase 1: Problem Definition

**Human Input:**
```
"I need to track a Costa Rica on-ramp flow:
CRC (bank) → USD (bank) → USD (exchange) → USDT → BTC

Current system only supports crypto. Need fiat currency support."
```

**Claude Analysis:**
- Identified need for multi-currency system
- Proposed database-driven approach
- Suggested exchange rate tracking
- Recommended cost basis in multiple currencies

### Phase 2: Architecture Design

**AI-Generated Proposal:**

1. Database Schema:
   - `currencies` table (fiat, crypto, stablecoin)
   - `exchange_rates` table (historical tracking)
   - Extend `holdings` for multi-currency cost basis
   - Extend `transactions` for multi-currency prices

2. Core Models:
   - `Currency` struct with `AssetType` enum
   - `ExchangeRate` struct with automatic upsert
   - Helper methods for currency detection

3. CLI Commands:
   - `currency list/show/add/remove/toggle`
   - `currency set-rate/show-rate`
   - Enhanced `tx swap` with rate parameter

**Human Review:**
- ✅ Approved architecture
- ✅ Confirmed database approach
- ✅ Requested 9 pre-seeded currencies

### Phase 3: Implementation

**AI-Driven Development:**

**Step 1: Database Migration**
```rust
// Claude created Migration 002
const MIGRATION_002: &str = r#"
CREATE TABLE IF NOT EXISTS currencies (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 2,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('fiat', 'crypto', 'stablecoin')),
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    source TEXT DEFAULT 'manual',
    notes TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_currency, to_currency, timestamp)
);

-- Seed 9 currencies
INSERT INTO currencies ...
"#;
```

**Step 2: Domain Models**
```rust
// Claude created src/core/currency.rs
pub enum AssetType {
    Fiat,
    Crypto,
    Stablecoin,
}

pub struct Currency {
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub asset_type: AssetType,
    pub enabled: bool,
    // ... timestamps
}

pub struct ExchangeRate {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub timestamp: DateTime<Utc>,
    // ... metadata
}
```

**Step 3: Database Layer**
```rust
// Claude created src/db/currencies.rs
pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<Currency>>
pub async fn add_currency(pool: &SqlitePool, currency: &Currency) -> Result<()>
pub async fn add_exchange_rate(pool: &SqlitePool, rate: &ExchangeRate) -> Result<i64>
pub async fn get_latest_exchange_rate(...) -> Result<Option<ExchangeRate>>
// ... 10 more functions
```

**Step 4: CLI Commands**
```rust
// Claude created src/cli/commands/currency.rs
pub enum CurrencyCommands {
    List { enabled: bool, type_filter: Option<String> },
    Show { code: String },
    Add { code, name, symbol, decimals, type_name },
    SetRate { from, to, rate, notes },
    // ... more commands
}
```

**Step 5: Enhanced Transaction Handling**
```rust
// Claude enhanced src/cli/commands/tx.rs
// Detect fiat-to-fiat swaps
let is_fiat_swap = from_currency.map(|c| c.is_fiat()).unwrap_or(false)
                && to_currency.map(|c| c.is_fiat()).unwrap_or(false);

if is_fiat_swap {
    // Calculate and store exchange rate
    let rate_record = ExchangeRate::new_manual(...);
    currencies::add_exchange_rate(pool, &rate_record).await?;
}
```

### Phase 4: Compilation Fixes

**AI-Assisted Debugging:**

Claude identified 24 compilation errors:
- Missing error variants (`InvalidInput`, `NotFound`, `AlreadyExists`)
- Missing fields in struct initializers
- sqlx type annotation issues (`as "created_at: String"`)
- Pattern match coverage issues

**Fix Rate:** All 24 errors resolved in single session

### Phase 5: Testing

**AI-Generated Test Suite:**

**Unit Tests (12 tests):**
```rust
#[test]
fn test_asset_type_from_str() { ... }

#[test]
fn test_currency_is_fiat() { ... }

#[test]
fn test_exchange_rate_inverse() { ... }
```

**Integration Tests (14 tests):**
```rust
#[tokio::test]
async fn test_currency_list_returns_seeded_currencies() { ... }

#[tokio::test]
async fn test_add_exchange_rate() { ... }

#[tokio::test]
async fn test_exchange_rate_upsert_on_conflict() { ... }
```

**Test Results:** ✅ 26/26 passing (100%)

### Phase 6: Documentation

**AI-Written Documentation:**

1. **README.md Updates:**
   - Multi-currency section (125 lines)
   - Costa Rica use case example
   - Currency management commands
   - Cost basis examples

2. **VALIDATION_GUIDE.md:**
   - Test V10 added (10 sub-tests)
   - Complete testing workflow
   - JSON output examples

3. **MULTI_CURRENCY_IMPLEMENTATION.md:**
   - Technical deep-dive (687 lines)
   - Database schema details
   - Implementation walkthrough
   - Testing coverage

### Results

**Development Metrics:**

| Metric | Value |
|--------|-------|
| **Total Time** | ~4 hours |
| **Files Created** | 6 new files |
| **Files Modified** | 20 existing files |
| **Lines Added** | 2,405 lines |
| **Tests Created** | 26 tests |
| **Test Pass Rate** | 100% |
| **Documentation** | 1,200+ lines |

**Quality Indicators:**

- ✅ All tests passing
- ✅ Compile-time type safety (Rust + sqlx)
- ✅ Complete documentation
- ✅ Real-world use case validated
- ✅ Zero production bugs

---

## Development Principles

### 1. AI Generates, Human Validates

**Pattern:**
- AI proposes solutions
- Human reviews and approves
- AI implements with human oversight

**Example:**
```
Human: "Add JPY currency"
AI: "I'll add JPY as a fiat currency with 0 decimals and ¥ symbol"
Human: "Approved"
AI: *Creates migration, updates seed data, writes tests*
```

### 2. Test-First Mentality

**Pattern:**
- Write tests alongside implementation
- Validate with `cargo test`
- Document test scenarios

**Result:** 110+ tests, 100% pass rate

### 3. Documentation as Code

**Pattern:**
- Generate docs during development
- Keep README concise, extract to guides
- Provide real-world examples

**Result:** 5+ documentation files, comprehensive guides

### 4. Iterative Refinement

**Pattern:**
- Quick first draft
- Review and improve
- Add edge case handling

**Example:**
```
v1: Basic currency list command
v2: Add filtering by type
v3: Add JSON output support
v4: Add enabled/disabled toggle
```

---

## Best Practices

### Working with Claude Code

**Do:**
- ✅ Provide clear context and goals
- ✅ Review all generated code
- ✅ Run tests frequently
- ✅ Ask for explanations
- ✅ Iterate on solutions

**Don't:**
- ❌ Blindly accept code without understanding
- ❌ Skip testing
- ❌ Ignore compiler warnings
- ❌ Forget to document decisions

### Code Quality

**Maintained Through:**
- Rust's type system
- sqlx compile-time checks
- Comprehensive test suite
- Human code review
- Clear architecture

### Communication with AI

**Effective Prompts:**
- "Add support for CoinGecko import with error handling and tests"
- "Refactor this function to reduce complexity while maintaining behavior"
- "Write integration tests for the Costa Rica on-ramp flow"

**Less Effective:**
- "Make it better"
- "Fix the bug"
- "Add features"

---

## Challenges & Solutions

### Challenge 1: Type System Complexity

**Problem:** Rust's strict type system can be intimidating

**Solution:** Claude explains type errors and suggests fixes
- Lifetime annotations
- Trait bounds
- Option/Result handling

### Challenge 2: sqlx Compile-Time Checks

**Problem:** Queries must be validated against real database

**Solution:** AI creates test database and runs migrations
- Sets up DATABASE_URL
- Generates migration files
- Validates query syntax

### Challenge 3: Comprehensive Testing

**Problem:** Writing tests is time-consuming

**Solution:** AI generates test suites automatically
- Unit tests for models
- Integration tests for database
- End-to-end workflow tests

---

## Metrics: AI vs Manual Development

### Time Savings

| Task | Manual Est. | AI-Assisted | Savings |
|------|-------------|-------------|---------|
| Database schema design | 2-3 hours | 30 min | 75% |
| Model implementation | 3-4 hours | 45 min | 80% |
| Database layer | 4-6 hours | 1 hour | 80% |
| CLI commands | 3-4 hours | 1 hour | 70% |
| Test suite | 4-6 hours | 45 min | 85% |
| Documentation | 2-3 hours | 30 min | 80% |
| **Total** | **18-26 hours** | **~4 hours** | **~80%** |

### Quality Comparison

| Metric | Manual Dev | AI-Assisted |
|--------|------------|-------------|
| Test Coverage | Variable | Comprehensive |
| Documentation | Often delayed | Generated alongside |
| Code Consistency | Varies | Enforced |
| Best Practices | Depends on dev | Automatically applied |
| Bug Rate | Typical | Lower (type safety + tests) |

---

## Learnings

### What Works Well

1. **Architecture Design** - AI excels at proposing solutions
2. **Boilerplate Code** - Repetitive code generated quickly
3. **Test Generation** - Comprehensive test suites
4. **Documentation** - Clear, well-structured docs
5. **Error Fixing** - Fast iteration on compiler errors

### What Needs Human Input

1. **Product Decisions** - What features to build
2. **UX Choices** - Command structure, output format
3. **Security Review** - Sensitive operations validation
4. **Performance Optimization** - Profiling and tuning
5. **Release Planning** - Version strategy, roadmap

---

## Future of Agentic Development

### Trends

- More developers using AI pair programming
- Better integration with development tools
- Improved code generation quality
- Faster iteration cycles

### At Cryptofolio

We'll continue building with AI assistance:
- v0.3: CoinGecko integration with Claude
- v0.4: Dashboard development with AI
- Ongoing: Bug fixes and feature additions

---

## Try Agentic Development Yourself

### Getting Started

1. **Install Claude Code** (or similar AI coding tool)
2. **Start Small** - Add a simple feature
3. **Review Everything** - Understand generated code
4. **Iterate** - Improve based on feedback
5. **Build Confidence** - Tackle larger features

### Example First Project

```bash
# Start Claude Code session
claude

# First task
you> "Add support for EUR currency to Cryptofolio"

# Follow AI guidance
# Review generated code
# Run tests
# Commit changes
```

---

## Conclusion

Agentic development with Claude Code enabled us to build Cryptofolio's multi-currency feature in ~4 hours instead of days, with:

- ✅ Production-ready code
- ✅ Comprehensive tests (100% passing)
- ✅ Complete documentation
- ✅ Type-safe implementation

**The key:** AI accelerates development, humans ensure quality.

---

**Questions?** Open an issue or discussion on GitHub.

**Want to contribute?** Check out [CONTRIBUTING.md](../CONTRIBUTING.md) for AI-assisted development workflow.

---

**Last Updated:** February 19, 2026
**Built with:** Claude Sonnet 4.5
