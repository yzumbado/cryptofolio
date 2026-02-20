# Cryptofolio Architecture

**Version:** 0.2.0
**Last Updated:** February 2026

This document describes the technical architecture of Cryptofolio, a local-first cryptocurrency portfolio manager built with Rust.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [System Layers](#system-layers)
- [Data Flow](#data-flow)
- [Database Schema](#database-schema)
- [Technology Stack](#technology-stack)
- [Design Decisions](#design-decisions)
- [Module Structure](#module-structure)

---

## Architecture Overview

Cryptofolio follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         USER INTERFACE                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    CLI (clap)                                │   │
│  │  cryptofolio <command> [subcommand] [args] [flags]          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      COMMAND HANDLERS                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │  price   │ │ account  │ │ holdings │ │portfolio │ │ currency │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        CORE DOMAIN                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │   Account    │  │   Holding    │  │      Transaction         │  │
│  │  - Exchange  │  │  - Asset     │  │  - Buy/Sell/Transfer     │  │
│  │  - Hardware  │  │  - Quantity  │  │  - Cost Basis Tracking   │  │
│  │  - Software  │  │  - CostBasis │  │  - Double-Entry Style    │  │
│  │  - Bank      │  │              │  │  - Multi-Currency        │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │  Portfolio   │  │   Category   │  │       P&L Calculator     │  │
│  │  - Aggregate │  │  - Trading   │  │  - Unrealized            │  │
│  │  - By Account│  │  - Cold      │  │  - Realized (planned)    │  │
│  │  - By Category│ │  - Banking   │  │  - Per Asset/Account     │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Currency (v0.2)                           │  │
│  │  - Multi-currency support (fiat, crypto, stablecoins)       │  │
│  │  - Exchange rate tracking                                   │  │
│  │  - Cost basis in multiple currencies                        │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              ▼                  ▼                  ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────┐
│   PERSISTENCE    │  │    EXCHANGE      │  │    CONFIGURATION     │
│  ┌────────────┐  │  │  ┌────────────┐  │  │  ┌────────────────┐  │
│  │   SQLite   │  │  │  │  Binance   │  │  │  │  config.toml   │  │
│  │  - accounts│  │  │  │  - Prices  │  │  │  │  - API keys    │  │
│  │  - holdings│  │  │  │  - Balances│  │  │  │  - Preferences │  │
│  │  - txns    │  │  │  │  - Trades  │  │  │  │  - Defaults    │  │
│  │  - currency│  │  │  └────────────┘  │  │  └────────────────┘  │
│  └────────────┘  │  │                  │  │                      │
│  ~/.config/      │  │  HTTPS + HMAC    │  │  ~/.config/          │
│  cryptofolio/    │  │  SHA256 Auth     │  │  cryptofolio/        │
│  database.sqlite │  │                  │  │  config.toml         │
└──────────────────┘  └──────────────────┘  └──────────────────────┘
```

---

## System Layers

### 1. User Interface Layer

**Responsibility:** Parse user input, display output

**Technologies:**
- `clap v4` - CLI argument parsing with derive macros
- `colored` - Terminal color output
- `indicatif` - Progress bars and spinners

**Key Files:**
- `src/cli/mod.rs` - CLI structure and command definitions
- `src/cli/output.rs` - Output formatting utilities

### 2. Command Handler Layer

**Responsibility:** Orchestrate business logic for each command

**Commands:**
- `price` - Fetch cryptocurrency prices
- `market` - Get detailed market data
- `account` - Manage accounts (CRUD operations)
- `category` - Manage categories
- `currency` - Manage currencies and exchange rates
- `holdings` - Manage holdings (add, remove, move)
- `portfolio` - View aggregated portfolio
- `tx` - Record transactions
- `sync` - Sync from exchange APIs
- `import` - Import from CSV
- `config` - Manage configuration
- `shell` - Interactive AI-powered shell
- `status` - System diagnostics

**Key Files:**
- `src/cli/commands/*.rs` - Individual command handlers

### 3. Core Domain Layer

**Responsibility:** Business logic and domain models

**Models:**
- `Account` - Exchange, wallet, or bank account
- `Holding` - Asset position with cost basis
- `Transaction` - Buy, sell, transfer, or swap
- `Currency` - Fiat, crypto, or stablecoin definition
- `ExchangeRate` - Historical exchange rate data
- `Category` - Account grouping
- `Portfolio` - Aggregated view of holdings

**Key Files:**
- `src/core/account.rs`
- `src/core/holdings.rs`
- `src/core/transaction.rs`
- `src/core/currency.rs` (v0.2)

### 4. Persistence Layer

**Responsibility:** Data storage and retrieval

**Database:** SQLite (embedded)

**Operations:**
- Migrations - Schema versioning
- CRUD - Create, Read, Update, Delete
- Queries - Complex data retrieval

**Key Files:**
- `src/db/migrations.rs` - Schema migrations
- `src/db/accounts.rs` - Account repository
- `src/db/holdings.rs` - Holdings repository
- `src/db/transactions.rs` - Transaction repository
- `src/db/currencies.rs` - Currency repository (v0.2)

### 5. Integration Layer

**Responsibility:** External API communication

**Exchanges:**
- Binance (Spot + Alpha)
- CoinGecko (planned v0.3)
- CoinMarketCap (planned v0.3)

**Key Files:**
- `src/exchange/binance.rs`

---

## Data Flow

### Portfolio View Flow

```
User: cryptofolio portfolio
         │
         ▼
    ┌─────────┐
    │  CLI    │ Parse args, validate
    └────┬────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│  SQLite DB  │ Load accounts, holdings
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│ Binance API │ Fetch current prices
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐
    │ P&L Calc│ Calculate unrealized P&L
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │ Output  │ Format table, colors
    └────┬────┘
         │
         ▼
      stdout
```

### Transaction Recording Flow

```
User: cryptofolio tx buy BTC 0.1 --account Binance --price 95000
         │
         ▼
    ┌─────────┐
    │  CLI    │ Parse transaction details
    └────┬────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│  Validate   │ Check account exists, amounts valid
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│  SQLite DB  │ Insert transaction
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│  SQLite DB  │ Update holdings (add quantity, update cost basis)
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐
    │ Output  │ Confirm transaction recorded
    └────┬────┘
         │
         ▼
      stdout
```

### Multi-Currency Swap Flow (v0.2)

```
User: cryptofolio tx swap CRC 100000 USD 181.82 --rate 550
         │
         ▼
    ┌─────────┐
    │  CLI    │ Parse swap details
    └────┬────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│ Currency DB │ Detect fiat-to-fiat swap
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│ Exchange DB │ Store exchange rate (550 CRC/USD)
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│Transaction  │ Record swap transaction
    └────┬────┘     └─────────────┘
         │
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Handler │────▶│  Holdings   │ Update holdings (remove CRC, add USD)
    └────┬────┘     └─────────────┘
         │
         ▼
      stdout (confirmation + rate stored notification)
```

---

## Database Schema

### Tables

#### categories
```sql
CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### accounts
```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category_id TEXT NOT NULL,
    account_type TEXT NOT NULL,
    config TEXT NOT NULL DEFAULT '{}',
    sync_enabled BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (category_id) REFERENCES categories(id)
);
```

#### holdings
```sql
CREATE TABLE holdings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    quantity TEXT NOT NULL,
    avg_cost_basis TEXT,
    cost_basis_currency TEXT,        -- v0.2
    avg_cost_basis_base TEXT,        -- v0.2
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, asset)
);
```

#### transactions
```sql
CREATE TABLE transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_type TEXT NOT NULL,
    from_account_id TEXT,
    from_asset TEXT,
    from_quantity TEXT,
    to_account_id TEXT,
    to_asset TEXT,
    to_quantity TEXT,
    price_usd TEXT,
    price_currency TEXT,             -- v0.2
    price_amount TEXT,               -- v0.2
    exchange_rate TEXT,              -- v0.2
    exchange_rate_pair TEXT,         -- v0.2
    fee TEXT,
    fee_asset TEXT,
    external_id TEXT,
    notes TEXT,
    timestamp DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### currencies (v0.2)
```sql
CREATE TABLE currencies (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 2,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('fiat', 'crypto', 'stablecoin')),
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### exchange_rates (v0.2)
```sql
CREATE TABLE exchange_rates (
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
```

### Relationships

```
categories (1) ──< (N) accounts
accounts (1) ──< (N) holdings
accounts (1) ──< (N) wallet_addresses
accounts (1) ──< (N) transactions (from/to)
```

---

## Technology Stack

| Layer | Technology | Version | Rationale |
|-------|------------|---------|-----------|
| **Language** | Rust | 1.70+ | Performance, memory safety, type safety, single binary |
| **CLI Framework** | clap | 4.x | Derive macros, subcommands, excellent UX |
| **Async Runtime** | Tokio | 1.x | Industry standard, mature ecosystem |
| **HTTP Client** | reqwest | 0.11.x | TLS support, async, widely used |
| **Database** | SQLite | 3.x | Embedded, zero-config, ACID compliant |
| **SQL Toolkit** | sqlx | 0.7.x | Compile-time checked queries, async |
| **Decimals** | rust_decimal | 1.x | Financial precision, no floating point errors |
| **JSON** | serde_json | 1.x | Serialization for JSON output |
| **Config** | TOML | - | Human-readable configuration |
| **Paths** | dirs | 5.x | XDG Base Directory specification |
| **Colors** | colored | 2.x | Terminal output styling |
| **Progress** | indicatif | 0.17.x | Progress bars and spinners |
| **Datetime** | chrono | 0.4.x | Timezone-aware timestamps |

---

## Design Decisions

### 1. Local-First Architecture

**Decision:** All data stored locally in SQLite

**Rationale:**
- Privacy - No data sent to cloud
- Speed - No network latency
- Reliability - Works offline
- Control - User owns their data

**Trade-offs:**
- No automatic backup
- No cross-device sync
- User responsible for data safety

### 2. Read-Only Exchange Access

**Decision:** Only read permissions for exchange APIs

**Rationale:**
- Security - Cannot withdraw or trade if compromised
- Trust - Users comfortable with read-only access
- Scope - Portfolio tracking, not trading

**Trade-offs:**
- Cannot execute trades via CLI
- Cannot automated portfolio rebalancing

### 3. SQLite with sqlx

**Decision:** Use sqlx for type-safe SQL queries

**Rationale:**
- Compile-time query validation
- No ORM overhead
- Direct SQL control
- Prevents SQL injection

**Trade-offs:**
- Requires DATABASE_URL at compile time
- More verbose than ORM

### 4. Average Cost Basis (Current)

**Decision:** Default to average cost method for holdings

**Rationale:**
- Simplest to implement
- Most common for crypto
- No need to track lot IDs

**Future:** FIFO/LIFO for tax optimization (v0.3)

### 5. Double-Entry Transactions

**Decision:** Transfers have both source and destination

**Rationale:**
- Accurate tracking across accounts
- Audit trail
- Balance verification

### 6. Multi-Currency Support (v0.2)

**Decision:** Database-driven currency model

**Rationale:**
- Extensible - Add currencies without code changes
- Flexible - Support any fiat or crypto
- Scalable - Historical exchange rates

---

## Module Structure

```
src/
├── cli/                    # Command-line interface
│   ├── commands/           # Command handlers
│   │   ├── account.rs     # Account management
│   │   ├── currency.rs    # Currency management (v0.2)
│   │   ├── holdings.rs    # Holdings management
│   │   ├── import.rs      # CSV import
│   │   ├── mod.rs
│   │   ├── portfolio.rs   # Portfolio view
│   │   ├── sync.rs        # Exchange sync
│   │   └── tx.rs          # Transaction recording
│   ├── mod.rs             # CLI entry point
│   └── output.rs          # Output formatting
│
├── core/                  # Domain models
│   ├── account.rs         # Account model
│   ├── category.rs        # Category model
│   ├── currency.rs        # Currency & ExchangeRate (v0.2)
│   ├── holdings.rs        # Holding model
│   ├── mod.rs
│   └── transaction.rs     # Transaction model
│
├── db/                    # Database layer
│   ├── accounts.rs        # Account repository
│   ├── categories.rs      # Category repository
│   ├── currencies.rs      # Currency repository (v0.2)
│   ├── holdings.rs        # Holdings repository
│   ├── migrations.rs      # Schema migrations
│   ├── mod.rs
│   └── transactions.rs    # Transaction repository
│
├── exchange/              # Exchange integrations
│   ├── binance.rs         # Binance API client
│   └── mod.rs
│
├── config.rs              # Configuration management
├── error.rs               # Error types
├── lib.rs                 # Library entry point
└── main.rs                # Application entry point
```

---

## Security Architecture

### Data Storage

- **Config:** `~/.config/cryptofolio/config.toml` (0600 permissions)
- **Database:** `~/.config/cryptofolio/database.sqlite` (0600 permissions)
- **API Keys:** Stored in config (plaintext in v0.2, encrypted in v0.3)

### Network Communication

- **HTTPS Only:** All API calls use TLS
- **HMAC SHA256:** Binance API authentication
- **No Telemetry:** Zero data sent to Anthropic or third parties

### Input Validation

- **SQL Injection:** Prevented by sqlx parameterized queries
- **Command Injection:** No shell execution of user input
- **Path Traversal:** Restricted to XDG directories

---

## Performance Considerations

### Database Optimization

- **Indexes:** Primary keys, unique constraints
- **Query Planning:** Efficient JOIN operations
- **Connection Pooling:** sqlx connection pool

### API Rate Limiting

- **Binance:** Respects API rate limits
- **Caching:** Price data cached for requests within same second

### Memory Usage

- **Streaming:** Large CSV imports streamed, not loaded entirely
- **Lazy Loading:** Holdings loaded on-demand
- **Decimal Precision:** No floating point, uses rust_decimal

---

## Future Architecture (v0.3+)

### Planned Changes

1. **macOS Keychain Integration** - Encrypted credential storage
2. **CoinGecko/CoinMarketCap** - Additional price sources
3. **CSV Reports** - Customizable report templates
4. **Realized P&L** - FIFO/LIFO tax lot tracking

### Experimental (v0.4)

- **Local Node.js Dashboard** - Visual data exploration
- **Chart.js/D3.js** - Interactive charts
- **Read-only SQLite Access** - Dashboard queries database directly

---

**Last Updated:** February 19, 2026
**Version:** 0.2.0
