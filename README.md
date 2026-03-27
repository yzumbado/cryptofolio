# Cryptofolio

> AI-Powered CLI for Multi-Currency Crypto Portfolio Management

[![Version](https://img.shields.io/badge/version-0.5.0-blue.svg)](CHANGELOG.md)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Developed with Claude Code](https://img.shields.io/badge/developed%20with-Claude%20Code-blueviolet.svg)](https://claude.ai/claude-code)

**Your crypto portfolio, in your terminal, under your control.**

Track cryptocurrency and fiat holdings across exchanges, wallets, and bank accounts with AI-powered natural language interface and comprehensive multi-currency support.

[Quick Start](#quick-start) • [Features](#features) • [Installation](#installation) • [Agentic Development](#agentic-development) • [Documentation](#documentation)

---

## Why Cryptofolio?

✅ **Ethereum Wallet Sync** - Automatic balance & ERC-20 token detection (NEW in v0.5.0)
✅ **Bitcoin Blockchain Sync** - HD wallet & testnet support (v0.5.0)
✅ **Binance History Sync** - Full trade/deposit/withdrawal/fiat history import
✅ **Keychain Security (macOS)** - OS-encrypted storage with Touch ID support
✅ **Multi-Currency Support** - Track CRC, USD, EUR alongside BTC, ETH, USDT
✅ **Local-First & Private** - All data stays on your machine
✅ **AI-Powered** - Natural language commands with Claude/Ollama integration
✅ **Agentic Development** - Built using AI pair programming with Claude Code
✅ **Read-Only Exchange Access** - Secure API integration (Binance)
✅ **Developer-Friendly** - JSON output, scriptable, CI/CD ready

---

## Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
  - [Multi-Currency Support](#-multi-currency-support)
  - [AI-Powered Interface](#-ai-powered-interface)
  - [Security First](#-security-first)
  - [Developer-Friendly](#-developer-friendly)
- [Agentic Development](#agentic-development)
- [Installation](#installation)
- [Usage](#usage)
  - [Basic Commands](#basic-commands)
  - [Real-World Examples](#real-world-examples)
- [Documentation](#documentation)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Quick Start

### Prerequisites
- macOS, Linux, or Windows
- Rust 1.70+ (for building from source)

### Installation

**From Source:**
```bash
git clone https://github.com/yourusername/cryptofolio.git
cd cryptofolio
cargo build --release
sudo cp target/release/cryptofolio /usr/local/bin/
```

**Verify:**
```bash
cryptofolio --version
# cryptofolio 0.5.0
```

### First Steps

**1. Check Bitcoin price:**
```bash
cryptofolio price BTC
# BTC: $70,253.98
```

**2. Create a wallet account:**
```bash
cryptofolio account add "My Ledger" --type hardware_wallet --category cold-storage
```

**3. Add holdings:**
```bash
cryptofolio holdings add BTC 0.5 --account "My Ledger" --cost 45000
```

**4. View portfolio:**
```bash
cryptofolio portfolio
```

**5. Try natural language (AI mode):**
```bash
cryptofolio shell
you> Show me my portfolio
you> I bought 0.1 BTC today at $95,000
```

**Next Steps:**
- [Connect Binance](#binance-integration) for auto-sync
- [Multi-currency setup](#-multi-currency-support) for fiat tracking
- [Security best practices](#-security-first)

---

## Features

### 💱 Multi-Currency Support

Track both fiat and crypto with automatic exchange rate management.

**Supported:**
- **Fiat:** USD, CRC (Costa Rican Colón), EUR, and custom additions
- **Crypto:** BTC, ETH, BNB, SOL
- **Stablecoins:** USDT, USDC

**Example: Costa Rica On-Ramp Flow**
```bash
# Convert CRC → USD → USDT → BTC with full cost basis tracking
cryptofolio account add "Banco Nacional" --type bank
cryptofolio holdings add CRC 100000 --account "Banco Nacional"

# Bank conversion: CRC to USD at rate 550
cryptofolio tx swap CRC 100000 USD 181.82 --rate 550 --account "Banco Nacional"
# ✓ Exchange rate automatically stored

# Transfer to on-ramp
cryptofolio account add "Lulubit" --type exchange --category on-ramp
cryptofolio tx transfer USD 181.82 --from "Banco Nacional" --to "Lulubit"

# Buy USDT
cryptofolio tx swap USD 181.82 USDT 176 --account "Lulubit"

# Transfer to exchange
cryptofolio tx transfer USDT 176 --from "Lulubit" --to "Binance" --fee 0.1

# Finally, buy BTC
cryptofolio tx swap USDT 175.9 BTC 0.0025 --account "Binance"

# View complete journey with cost basis preserved
cryptofolio portfolio
```

**Currency Management:**
```bash
cryptofolio currency list                    # All currencies
cryptofolio currency add JPY "Japanese Yen" "¥" --type fiat
cryptofolio currency set-rate CRC USD 550    # Manual rate entry
cryptofolio currency show-rate CRC USD --history  # Rate history
```

[See full multi-currency guide →](docs/MULTI_CURRENCY_IMPLEMENTATION.md)

### ₿ Bitcoin Wallet Tracking

Track Bitcoin wallets with automatic blockchain synchronization (mainnet & testnet).

**Add Bitcoin Wallets:**
```bash
# Mainnet wallet
cryptofolio wallet add "My BTC" --blockchain bitcoin \
  --address bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh

# Testnet wallet (automatic detection)
cryptofolio wallet add "Test Wallet" --blockchain bitcoin \
  --address tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx

# HD wallet with xpub
cryptofolio wallet add "Hardware Wallet" --blockchain bitcoin \
  --xpub xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKr...
```

**Sync Blockchain Data:**
```bash
# Sync single wallet
cryptofolio wallet sync "My BTC"
# ✓ Synced BITCOIN balance: 0.05420000
# Transactions: 12
# Total received: 0.15000000 BTC

# Sync all wallets
cryptofolio wallet sync --all

# Import transaction history
cryptofolio wallet sync "My BTC" --import-history
# ✓ Imported 12 transactions
```

**Supported Address Formats:**
- **Legacy (P2PKH):** 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa
- **P2SH:** 3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy
- **Bech32 (SegWit):** bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh
- **Testnet:** tb1q..., m..., n..., 2...
- **Extended Public Keys:** xpub, ypub, zpub, tpub, upub, vpub

**Features:**
- ✅ Automatic mainnet/testnet detection
- ✅ Blockstream API integration (no local node required)
- ✅ Balance and transaction history sync
- ✅ Decimal precision (no floating-point errors)
- ✅ Cross-chain validation (prevents ETH address on BTC wallet)

[See blockchain sync guide →](BLOCKCHAIN_SYNC.md) • [Testnet setup →](TESTNET_SETUP_GUIDE.md) • [Testnet support →](TESTNET_SUPPORT.md)

### Ξ Ethereum Wallet Tracking

Track Ethereum wallets with automatic ERC-20 token detection (mainnet & Sepolia testnet).

**Add Ethereum Wallets:**
```bash
# Mainnet wallet
cryptofolio wallet add "My ETH" --blockchain ethereum \
  --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0

# Sepolia testnet wallet
cryptofolio wallet add "Test Wallet" --blockchain ethereum \
  --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0 \
  --network testnet
```

**Sync Blockchain Data:**
```bash
# Sync single wallet
cryptofolio wallet sync "My ETH"
# ✓ Synced ETH balance: 2.5420
# ✓ Found 3 tokens
#   USDT: 1000.00
#   USDC: 500.00
#   LINK: 25.50

# Import transaction history
cryptofolio wallet sync "My ETH" --import-history
# ✓ Imported 42 transactions
```

**ERC-20 Token Support:**
- ✅ Automatic token detection from transaction history
- ✅ Multi-decimal precision (6 for USDT/USDC, 18 for most tokens)
- ✅ Token aggregation (calculates balance from all transfers)
- ✅ Common tokens: USDT, USDC, DAI, LINK, UNI, AAVE, etc.

**Features:**
- ✅ Automatic mainnet/testnet detection
- ✅ Etherscan API integration (no local node required)
- ✅ Balance, token, and transaction history sync
- ✅ Gas tracking for cost basis calculations
- ✅ Checksum address validation

[Testnet setup →](TESTNET_SETUP_GUIDE_ETHEREUM.md) • [Validation checklist →](VALIDATION_CHECKLIST_ETHEREUM.md)

### 🤖 AI-Powered Interface

Natural language commands powered by Claude or local Ollama.

```bash
cryptofolio shell

  🪙 Cryptofolio v0.5.0
  AI-Powered Portfolio Assistant

  💰 Portfolio: $61,442.89 (+109.57%)
  🧪 Testnet  •  🦙 AI Ready (Ollama)

you> What's the price of Bitcoin?
you> Show my portfolio
you> I bought 0.1 BTC today at $95,000
you> How much ETH do I have?
```

**AI Providers:**
- **Claude** (cloud) - Advanced reasoning for complex queries
- **Ollama** (local) - Privacy-first, runs on your machine
- **Hybrid** - Automatically chooses best provider
- **Pattern-based** - Fallback regex matching (no AI needed)

**Check AI status:**
```bash
cryptofolio status

  🤖 AI Providers
  ─────────────────────────────────────
  ☁️ Claude       Offline (API key not configured)
  🦙 Ollama       Connected (llama3.2:3b)

  ⚡ AI Mode      Hybrid (Local + Cloud)
  🎯 Active       Ollama only (llama3.2:3b)
```

### 🔒 Security First

**macOS Keychain Storage (NEW!)** - OS-encrypted secret storage
**Read-only API access** - Never grant withdrawal permissions
**Local-first** - All data stays on your machine
**Secure secrets** - API keys never in shell history or plaintext
**File permissions** - Automatic 0600 on config files

```bash
# Secure API key storage with macOS Keychain (v0.3+)
cryptofolio config set-secret binance.api_secret
Enter secret (hidden): ********
✓ Stored in macOS Keychain (Touch ID Protected)

# Multiple input methods
echo "secret" | cryptofolio config set-secret binance.api_secret  # Stdin
cryptofolio config set-secret binance.api_secret --from-env BINANCE_SECRET  # Env

# Security levels (macOS only)
cryptofolio config set-secret api.key --security-level standard       # Mac unlock
cryptofolio config set-secret api.key --security-level touchid        # Touch ID (recommended)
cryptofolio config set-secret api.key --security-level touchid-only   # Biometric only

# Migrate existing secrets from TOML to Keychain
cryptofolio config migrate-to-keychain

# Check keychain status
cryptofolio config keychain-status
```

**Keychain Security Features (macOS):**
- ✅ **OS-Encrypted Storage** - Protected by macOS security
- ✅ **Touch ID Support** - Biometric authentication for secrets
- ✅ **No Plaintext** - Secrets never stored in TOML files
- ✅ **Backup Protected** - Keychain items excluded from backups
- ✅ **Auto-Migration** - Easy upgrade from v0.2.0

**Binance API Key Setup:**

When creating your Binance API key:
1. Go to Binance → API Management → Create API
2. **Enable ONLY:** ✅ Enable Reading
3. **DISABLE (CRITICAL):** ❌ Enable Spot & Margin Trading, ❌ Enable Withdrawals

**Why READ-ONLY?** Even with keychain encryption, always use read-only API keys:
- **READ-ONLY keys:** Attacker can only view portfolio → No financial loss ✅
- **WRITE permissions:** Attacker can steal funds → Total loss ❌

[Security best practices →](SECURITY.md)

### 📊 Developer-Friendly

**JSON output** - All commands support `--json`
**Scriptable** - Integrate with jq, Python, CI/CD
**MCP compatible** - Build Model Context Protocol tools

```bash
# Extract portfolio value
cryptofolio portfolio --json | jq -r '.total_value_usd'

# Alert on threshold
TOTAL=$(cryptofolio portfolio --json | jq -r '.total_value_usd')
if (( $(echo "$TOTAL < 50000" | bc -l) )); then
  notify-send "Portfolio Alert" "Total value below $50k!"
fi

# Daily snapshots
echo "$(date): $(cryptofolio portfolio --json)" >> ~/portfolio-history.jsonl
```

**JSON output available for:**
- `portfolio --json` - Portfolio overview
- `price BTC ETH --json` - Price data
- `market BTCUSDT --json` - Market data
- `holdings list --json` - Holdings
- `account list --json` - Accounts
- `tx list --json` - Transactions
- `currency list --json` - Currencies
- `config show --json` - Configuration

### 💰 Profit & Loss Tracking

Automatic P&L calculation with FIFO/LIFO tax lot matching.

**Real-time P&L on every trade:**
```bash
# Buy transactions create tax lots automatically
$ cryptofolio tx buy BTC 1.0 --account Binance --price 40000
✓ Recorded buy: 1.0000 BTC @ $40,000.00 in 'Binance'

$ cryptofolio tx buy BTC 1.0 --account Binance --price 50000
✓ Recorded buy: 1.0000 BTC @ $50,000.00 in 'Binance'

# Sell transactions match tax lots and show realized P&L
$ cryptofolio tx sell BTC 1.5 --account Binance --price 60000
✓ Recorded sell: 1.5000 BTC @ $60,000.00 from 'Binance' (Realized P&L: +$25,000.00)
```

**P&L Commands:**
```bash
# Overall summary
$ cryptofolio pnl summary
=== P&L Summary ===
Realized P&L:   +$25,000.00
Unrealized P&L: +$8,174.00
─────────────────────────────
Net P&L:        +$33,174.00

# Detailed realized gains/losses
$ cryptofolio pnl realized
Date          Asset     Quantity      Cost Basis    Proceeds      Gain/Loss
---------------------------------------------------------------------------
2026-03-02    BTC       1.0000        $40,000.00    $60,000.00    +$20,000.00
2026-03-02    BTC       0.5000        $25,000.00    $30,000.00    +$5,000.00

# Unrealized P&L on current holdings
$ cryptofolio pnl unrealized

# Per-asset breakdown
$ cryptofolio pnl by-asset BTC

# Replay historical transactions
$ cryptofolio pnl backfill
```

**Features:**
- ✅ FIFO (First In, First Out) matching
- ✅ LIFO (Last In, First Out) matching
- ✅ Automatic tax lot tracking
- ✅ Realized gain/loss calculation
- ✅ Unrealized P&L monitoring
- ✅ Holding period tracking (for tax reporting)
- ✅ Per-asset and per-account breakdowns

### 🔄 Binance History Sync (NEW in v0.4.0)

Automatically import your complete Binance transaction history with a single command.

```bash
# First: store your API keys securely
cryptofolio config set-secret binance.api_key
cryptofolio config set-secret binance.api_secret

# Dry-run: see what would be imported
cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT --dry-run

# Full import
cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT --full-history

# Incremental (only fetches new transactions since last sync)
cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT
```

**What's imported:**
- ✅ **Spot trades** → Buy / Sell transactions with automatic P&L
- ✅ **Crypto deposits** → Transfer In (updates holdings)
- ✅ **Crypto withdrawals** → Transfer Out (reduces holdings)
- ✅ **Fiat on-ramp orders** → Buy transactions (credit card, bank)
- ✅ **Internal transfers** → Spot ↔ Earn wallet moves

**Key features:**
- Duplicate-safe: re-running never creates duplicate transactions
- Incremental: watermarks remember last sync, only fetches what's new
- `--full-history` re-imports everything from the beginning
- `--dry-run` previews without writing anything

### ✨ Additional Features

- ✅ **Binance Integration** - Balance sync + full history import (trades, deposits, withdrawals)
- ✅ **Transaction History** - CSV import/export with filtering
- ✅ **Cost Basis Tracking** - Accurate P&L calculations with tax lot matching
- ✅ **Testnet Support** - Practice without real funds
- ✅ **Customizable Formatting** - Decimal precision, thousands separators
- ✅ **Interactive Shell** - Tab completion and command history
- ✅ **Dry-Run Mode** - Preview changes without committing

---

## Agentic Development

### Built with Claude Code

Cryptofolio is a **showcase of agentic software development** - built using AI pair programming with Claude Code (Anthropic's official CLI).

**Development Approach:**
- 🤖 **AI-Driven Implementation** - Features designed and coded with Claude's assistance
- 🧪 **Test-First Development** - 110+ tests written alongside implementation
- 📚 **Auto-Documentation** - Comprehensive docs generated during development
- 🔄 **Iterative Refinement** - Continuous improvement through AI feedback

### Case Study: Multi-Currency Feature

**Phase 1: Design (AI-Assisted)**
```
Human: "I need to track CRC → USD → USDT → BTC conversions"
Claude: "Let me design a multi-currency architecture..."
```

Claude proposed:
- Database-driven currency model (extensible without code changes)
- Exchange rate table with automatic upsert logic
- Multi-currency cost basis fields
- Automatic rate storage for fiat swaps

**Phase 2: Implementation (AI Pair Programming)**
```
Human: "Implement the database schema"
Claude: *Creates migration with currencies & exchange_rates tables*
Claude: *Adds 9 pre-seeded currencies (USD, CRC, EUR, BTC, ETH, BNB, SOL, USDT, USDC)*
Claude: *Updates holdings/transactions for multi-currency support*
Claude: *Implements 14 database functions for currency management*
```

**Phase 3: Testing (AI-Generated)**
```
Human: "Add comprehensive tests"
Claude: *Creates 12 unit tests for currency models*
Claude: *Creates 14 integration tests for database layer*
Claude: *Tests Costa Rica on-ramp flow end-to-end*
```

**Phase 4: Documentation (AI-Written)**
```
Human: "Document this for users"
Claude: *Updates README with multi-currency section (125 lines)*
Claude: *Adds 10 test scenarios to VALIDATION_GUIDE.md*
Claude: *Creates MULTI_CURRENCY_IMPLEMENTATION.md (687 lines)*
```

**Result:** Complete feature in ~4 hours:
- ✅ 2,405 lines of code added
- ✅ 26 tests (all passing)
- ✅ 1,200+ lines of documentation
- ✅ Real-world use case validated
- ✅ Zero production bugs

### AI Development Metrics

| Metric | Value |
|--------|-------|
| **Total Tests** | 341 (203 unit + 138 integration) |
| **Test Pass Rate** | 100% |
| **Development Time** | ~4 hours per feature (vs 18-26 hours manual) |
| **Time Savings** | ~80% |
| **Code Quality** | Rust compile-time guarantees + sqlx type safety |
| **Documentation** | README + 5 guides + inline docs |

### Learn More

- [Full development process walkthrough](docs/AGENTIC_DEVELOPMENT.md)
- [Architecture deep-dive](docs/ARCHITECTURE.md)
- [Contributing with AI assistance](CONTRIBUTING.md)

**Want to build with AI?** Check out our guide on [AI pair programming for Cryptofolio contributions](CONTRIBUTING.md).

---

## Installation

### From Source (Recommended)

**Requirements:**
- Rust 1.70 or later ([install](https://rustup.rs/))
- Git

**Steps:**
```bash
git clone https://github.com/yourusername/cryptofolio.git
cd cryptofolio
cargo build --release
sudo cp target/release/cryptofolio /usr/local/bin/
```

**Verify:**
```bash
cryptofolio --version
# cryptofolio 0.3.1
```

### Platform-Specific Notes

**macOS:**
```bash
# Using Homebrew (coming soon)
brew install cryptofolio
```

**Linux:**
```bash
# Debian/Ubuntu - install dependencies first
sudo apt install build-essential pkg-config libssl-dev

# Then build from source
```

**Windows:**
```powershell
# Install Rust from https://rustup.rs/
# Then build from source using PowerShell
```

### Troubleshooting

**"cargo: command not found"**
- Install Rust: https://rustup.rs/

**SQLite errors:**
- Install SQLite development libraries:
  - Ubuntu/Debian: `sudo apt install libsqlite3-dev`
  - macOS: `brew install sqlite`

---

## Usage

### Basic Commands

**Portfolio Management:**
```bash
cryptofolio portfolio                    # View portfolio
cryptofolio holdings list                # List all holdings
cryptofolio holdings add BTC 0.5 --account "My Ledger" --cost 45000
cryptofolio holdings move BTC 0.1 --from "Binance" --to "Ledger"
```

**Price Checking:**
```bash
cryptofolio price BTC ETH               # Current prices
cryptofolio price NIGHT                 # Binance Alpha tokens
cryptofolio market BTC --24h            # 24h market data
```

**Transactions:**
```bash
cryptofolio tx buy BTC 0.1 --account Binance --price 95000
cryptofolio tx sell ETH 0.5 --account Binance --price 3200
cryptofolio tx transfer BTC 0.24 --from "Binance" --to "Ledger" --fee 0.0001
cryptofolio tx swap USD 100 USDT 97 --account Lulubit  # Multi-currency
cryptofolio tx list --limit 20
cryptofolio tx export 2024.csv --from 2024-01-01 --to 2024-12-31
```

**Currency Management:**
```bash
cryptofolio currency list                           # All currencies
cryptofolio currency show CRC                       # Currency details
cryptofolio currency add JPY "Japanese Yen" "¥" --type fiat --decimals 0
cryptofolio currency set-rate CRC USD 550 --notes "Bank rate"
cryptofolio currency show-rate CRC USD --history    # Rate history
cryptofolio currency toggle CRC --disable           # Disable without deleting
```

**Accounts:**
```bash
cryptofolio account add "Ledger" --type hardware_wallet --category cold-storage
cryptofolio account add "Binance" --type exchange --category trading --sync
cryptofolio account list
cryptofolio account show Binance
cryptofolio sync --account "Binance"   # Sync balances from API
```

**Binance History Sync (v0.4.0):**
```bash
# Import all transaction history
cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT

# Dry-run preview
cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run

# Full re-import from scratch
cryptofolio sync-history --account Binance --symbols BTCUSDT --full-history

# Import from a specific date
cryptofolio sync-history --account Binance --symbols BTCUSDT --from 2024-01-01

# Skip specific record types
cryptofolio sync-history --account Binance --symbols BTCUSDT --no-fiat --no-transfers
```

**Configuration:**
```bash
cryptofolio config show
cryptofolio config set-secret binance.api_secret  # Secure input
cryptofolio config set display.decimals 6
cryptofolio config use-testnet
```

**Global Flags:**
- `--json` - Output in JSON format
- `--quiet` - Suppress non-essential output
- `--testnet` - Use Binance testnet
- `--yes` - Skip confirmation prompts
- `--dry-run` - Preview changes without committing

### Real-World Examples

**Morning Portfolio Check:**
```bash
cryptofolio portfolio
cryptofolio market BTC --24h | grep Change
```

**Weekly DCA Script:**
```bash
#!/bin/bash
# weekly-dca.sh - Run every Sunday

cryptofolio tx buy BTC 0.01 \
  --account "Binance" \
  --price $(cryptofolio price BTC --json | jq -r '.price') \
  --notes "Weekly DCA $(date +%Y-%m-%d)"

cryptofolio sync --account "Binance"
echo "$(date): $(cryptofolio portfolio --json | jq -r '.total_value_usd')" >> ~/portfolio-log.txt
```

**Costa Rica On-Ramp Flow:**
```bash
# 1. Bank account with CRC
cryptofolio account add "Banco Nacional" --type bank
cryptofolio holdings add CRC 100000 --account "Banco Nacional"

# 2. Convert CRC → USD at bank
cryptofolio tx swap CRC 100000 USD 181.82 --rate 550 --account "Banco Nacional"

# 3. Transfer to on-ramp
cryptofolio account add "Lulubit" --type exchange --category on-ramp
cryptofolio tx transfer USD 181.82 --from "Banco Nacional" --to "Lulubit"

# 4. Buy USDT
cryptofolio tx swap USD 181.82 USDT 176 --account "Lulubit"

# 5. Transfer to exchange
cryptofolio tx transfer USDT 176 --from "Lulubit" --to "Binance" --fee 0.1

# 6. Buy BTC
cryptofolio tx swap USDT 175.9 BTC 0.0025 --account "Binance"

# View complete cost basis chain
cryptofolio portfolio
```

**P&L Tracking Workflow:**
```bash
# Track Bitcoin trades with automatic P&L
# Tax lot 1: Buy low
cryptofolio tx buy BTC 1.0 --account "Binance" --price 40000
# ✓ Tax lot created @ $40k

# Tax lot 2: Buy higher
cryptofolio tx buy BTC 1.0 --account "Binance" --price 50000
# ✓ Tax lot created @ $50k

# Sell with FIFO matching (first lot matched first)
cryptofolio tx sell BTC 1.5 --account "Binance" --price 60000
# ✓ Realized P&L: +$25,000.00
#   - 1.0 BTC from lot 1: ($60k - $40k) × 1.0 = $20k
#   - 0.5 BTC from lot 2: ($60k - $50k) × 0.5 = $5k

# Check P&L summary
cryptofolio pnl summary
# Realized P&L:   +$25,000.00
# Unrealized P&L: +$8,174.00  (0.5 BTC remaining @ $50k cost basis)
# Net P&L:        +$33,174.00

# View detailed realized gains for tax reporting
cryptofolio pnl realized
# Shows: disposal date, quantity, cost basis, proceeds, gain/loss, holding period
```

**Tax Season Export:**
```bash
# Export all 2024 transactions
cryptofolio tx export 2024-transactions.csv --from 2024-01-01 --to 2024-12-31

# Export specific account
cryptofolio tx export binance-2024.csv --account "Binance" --from 2024-01-01 --to 2024-12-31

# Export specific asset
cryptofolio tx export btc-trades.csv --asset BTC --from 2024-01-01 --to 2024-12-31
```

**CI/CD Integration:**
```yaml
# .github/workflows/portfolio-monitor.yml
name: Daily Portfolio Snapshot

on:
  schedule:
    - cron: '0 8 * * *'  # Every day at 8 AM

jobs:
  snapshot:
    runs-on: ubuntu-latest
    steps:
      - name: Install cryptofolio
        run: cargo install cryptofolio

      - name: Take snapshot
        run: |
          cryptofolio portfolio --json > snapshot.json

      - name: Check for large changes
        run: |
          CHANGE=$(jq '.unrealized_pnl_percent' snapshot.json)
          if (( $(echo "$CHANGE < -10" | bc -l) )); then
            echo "::warning::Portfolio down more than 10%!"
          fi
```

---

## Documentation

### User Guides
- [Multi-Currency Guide](docs/MULTI_CURRENCY_IMPLEMENTATION.md) - Fiat, crypto, stablecoins
- [Security Best Practices](SECURITY.md) - API keys, file permissions
- [Contributing Guide](CONTRIBUTING.md) - AI-assisted development workflow

### Technical Documentation
- [Architecture Overview](docs/ARCHITECTURE.md) - System design, database schema
- [Agentic Development Process](docs/AGENTIC_DEVELOPMENT.md) - How we built this with AI
- [Validation Guide](docs/VALIDATION_GUIDE.md) - Testing scenarios

### Development
- [Stability Plan](STABILITY_PLAN.md) - **Short-term development guide (NEW!)**
- [Stability Checklist](STABILITY_CHECKLIST.md) - Quick reference for daily work
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)
- [Roadmap](docs/ROADMAP.md)

**Note:** All documentation was written with AI assistance using Claude Code.

---

## Roadmap

### v0.2.0 (✅ Released - February 2026)
- ✅ Multi-currency support (fiat, crypto, stablecoins)
- ✅ Exchange rate management with historical tracking
- ✅ Bank account type
- ✅ Secure secret handling (stdin, file, env, interactive)
- ✅ JSON output for all query commands
- ✅ CSV transaction export with filtering
- ✅ Customizable number formatting

[View v0.2.0 release notes →](CHANGELOG.md#020---2026-02-19)

### v0.3.1 (✅ Released - March 2026)
- ✅ macOS Keychain integration (OS-encrypted storage)
- ✅ Touch ID security levels (Standard, Protected, Only)
- ✅ P&L Engine foundation (tax lots, realized gains)
- ✅ FIFO/LIFO cost basis matching
- ✅ Comprehensive test suite (259 tests, 100% pass rate)
- ✅ Quality improvements (95-100% critical code coverage)

[View v0.3.1 release notes →](CHANGELOG.md#031---2026-03-01)

### v0.4.0 (✅ Released - March 2026) - Binance Deep Integration
- ✅ Complete trade history import (`sync-history` command)
- ✅ Deposit / withdrawal / fiat order / internal transfer sync
- ✅ Incremental sync with watermarks (only fetches new records)
- ✅ Duplicate detection (safe to re-run at any time)
- ✅ Dry-run mode (preview without writing)
- ✅ 341 total tests (44 new integration tests), 100% passing

[View v0.4.0 release notes →](RELEASE_NOTES_v0.4.0.md)

### v0.5.0 (Planned) - Visual Data Exploration (Experimental)
- [ ] Local Node.js dashboard (no external dependencies)
- [ ] Rich data visualization (charts, graphs, trends)
- [ ] Interactive portfolio explorer
- [ ] Time-series analysis
- [ ] Portfolio composition breakdown
- [ ] Historical performance tracking

### Long-Term Vision
- Multi-chain DeFi integration
- Advanced analytics and insights
- Community-built dashboard plugins
- Real-time portfolio monitoring

**Want to influence the roadmap?** See [ROADMAP.md](docs/ROADMAP.md) for detailed plans.

---

## Contributing

We welcome contributions! Cryptofolio is built using **agentic development** with Claude Code, making it easy for anyone to contribute - even if you're not a Rust expert.

### How to Contribute

**Traditional Development:**
```bash
# Fork and clone
git clone https://github.com/yourusername/cryptofolio.git
cd cryptofolio

# Create a branch
git checkout -b feature/my-feature

# Make changes, add tests
cargo test

# Submit PR
```

**AI-Assisted Development (Recommended):**

We encourage using Claude Code for contributions:

```bash
# Start AI pair programming session
claude

you> "I want to add support for JPY currency"
Claude> "Let me help you implement that..."
```

**Why AI-Assisted?**
- 🚀 **Faster development** - Claude writes boilerplate
- 🧪 **Better tests** - AI generates comprehensive test suites
- 📚 **Auto-documentation** - Docs written as you code
- 🎯 **Higher quality** - Rust's type system + AI verification

### Contribution Ideas

**Good First Issues:**
- Add new currency support
- Improve error messages
- Add examples to documentation
- Write integration tests

**Intermediate:**
- Add new exchange integration
- Implement new transaction types
- Enhance AI natural language processing

**Advanced:**
- Build local dashboard
- Implement tax calculation algorithms
- Add DeFi protocol integration

[Full contributing guide →](CONTRIBUTING.md)

### Code of Conduct

We follow the [Contributor Covenant](CODE_OF_CONDUCT.md). Be respectful and inclusive.

---

## Binance Integration

### Setup (Secure Method - v0.2+)

**1. Create Read-Only API Key:**
1. Go to Binance → API Management → Create API
2. **Enable ONLY:** ✅ Enable Reading
3. **DISABLE:** ❌ Trading, ❌ Withdrawals, ❌ Transfers

> ⚠️ **For `sync-history`**, your API key also needs **"Enable Spot & Margin Trading" permission disabled** but must have access to trade history and deposit/withdrawal endpoints. Read-only is sufficient — Binance grants trade history access on all read-only keys.

**2. Configure Cryptofolio (Securely):**
```bash
# Use set-secret for hidden input
cryptofolio config set-secret binance.api_key
Enter secret (hidden): ********

cryptofolio config set-secret binance.api_secret
Enter secret (hidden): ********
```

**3. Create Synced Account:**
```bash
cryptofolio account add "Binance" --type exchange --category trading --sync
```

**4. Sync Current Balances:**
```bash
cryptofolio sync --account "Binance"
# ✓ Synced 3 assets from 'Binance'
```

**5. Import Full Transaction History (NEW in v0.4.0):**
```bash
# Dry-run first
cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT,BNBUSDT \
  --full-history \
  --dry-run

# Real import
cryptofolio sync-history --account Binance \
  --symbols BTCUSDT,ETHUSDT,BNBUSDT \
  --full-history

# Incremental (daily use)
cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT
```

**Testnet Mode:**
```bash
cryptofolio config use-testnet
# Practice with fake funds on Binance testnet
```

---

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) 🦀
- Developed using [Claude Code](https://claude.ai/claude-code) - Anthropic's official CLI
- Binance API integration
- Inspired by the need for privacy-first portfolio management

## Support

- 📖 [Documentation](docs/)
- 🐛 [Issues](https://github.com/yourusername/cryptofolio/issues)
- 🔐 [Security Policy](SECURITY.md)

## License

MIT License - see [LICENSE](LICENSE) for details.

Copyright © 2026 Cryptofolio Contributors

---

**If you find Cryptofolio useful, give us a star on GitHub!**

**Interested in agentic development?** Check out our [development story](docs/AGENTIC_DEVELOPMENT.md) to learn how we built this with AI.

```
   ___                  _         __       _ _
  / __\ __ _   _ _ __ | |_ ___  / _| ___ | (_) ___
 / / | '__| | | | '_ \| __/ _ \| |_ / _ \| | |/ _ \
/ /__| |  | |_| | |_) | || (_) |  _| (_) | | | (_) |
\____/_|   \__, | .__/ \__\___/|_|  \___/|_|_|\___/
           |___/|_|
```
