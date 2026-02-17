# Cryptofolio

A powerful command-line interface for managing cryptocurrency portfolios across multiple locations - exchanges, hardware wallets, and software wallets.

---

## Problem Statement

### The Challenge

Cryptocurrency investors face a fragmented portfolio management experience:

1. **Scattered Holdings**: Assets are spread across multiple exchanges (Binance, Coinbase, Kraken), hardware wallets (Ledger, Trezor), and software wallets (MetaMask, Trust Wallet). There's no single view of total holdings.

2. **Manual Tracking**: Most investors resort to spreadsheets, which are error-prone, time-consuming to update, and don't integrate with real-time pricing.

3. **P&L Blindness**: Without proper cost basis tracking, investors don't know their actual profit/loss position until tax season arrives.

4. **Security Concerns**: Web-based portfolio trackers require API keys with withdrawal permissions, creating security risks. Many investors are uncomfortable giving third parties access to their exchange accounts.

5. **Developer Unfriendly**: Existing solutions are GUI-based apps that don't integrate into developer workflows, CI/CD pipelines, or automation scripts.

### The Solution

**Cryptofolio** is a local-first, privacy-respecting CLI tool that:

- Aggregates holdings from multiple sources into a unified view
- Tracks cost basis automatically for accurate P&L reporting
- Runs entirely on your machine - no data leaves your computer
- Provides both human-friendly output and JSON for scripting
- Integrates with Binance API for automatic syncing (read-only permissions)

---

## User Profile

### Primary Users

#### 1. The Technical Investor
- **Who**: Software developers, DevOps engineers, system administrators
- **Portfolio**: $10K - $500K across 3-5 locations
- **Pain Point**: Wants CLI tools that fit their workflow, not another web app
- **Usage**: Daily price checks, weekly portfolio reviews, automated alerts

#### 2. The Security-Conscious HODLer
- **Who**: Long-term investors prioritizing self-custody
- **Portfolio**: Majority in cold storage (Ledger/Trezor)
- **Pain Point**: Doesn't trust web apps with API keys; wants local-only solution
- **Usage**: Monthly portfolio snapshots, annual tax reporting

#### 3. The Active Trader
- **Who**: Day traders, swing traders on centralized exchanges
- **Portfolio**: Primarily on exchanges, frequent transactions
- **Pain Point**: Needs quick access to positions and P&L without leaving terminal
- **Usage**: Multiple times daily, often automated via scripts

#### 4. The DeFi Explorer
- **Who**: Users with assets across chains and protocols
- **Portfolio**: Mix of CEX, DEX, and DeFi positions
- **Pain Point**: No unified view across all holdings
- **Usage**: Weekly rebalancing, tracking across multiple wallets

### User Personas

```
┌─────────────────────────────────────────────────────────────────┐
│  ALEX - Senior Software Engineer                                │
├─────────────────────────────────────────────────────────────────┤
│  Portfolio: $85,000                                             │
│  Locations: Binance, Ledger Nano X, MetaMask                    │
│  Goals: Track P&L, automate portfolio snapshots in CI           │
│  Frustration: "I don't want another app - I live in terminal"   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  MARIA - Startup Founder                                        │
├─────────────────────────────────────────────────────────────────┤
│  Portfolio: $250,000                                            │
│  Locations: Coinbase, Trezor, company treasury wallet           │
│  Goals: Separate personal vs business holdings, tax reports     │
│  Frustration: "I need cost basis for my accountant"             │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  JAMES - Part-time Trader                                       │
├─────────────────────────────────────────────────────────────────┤
│  Portfolio: $15,000                                             │
│  Locations: Binance (testnet for practice), small Ledger stash  │
│  Goals: Learn trading without risking real money                │
│  Frustration: "Testnet tools are terrible"                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Use Cases

### UC-1: View Real-Time Portfolio

**Actor**: Any user
**Precondition**: Holdings are configured
**Flow**:
1. User runs `cryptofolio portfolio`
2. System fetches current prices from Binance
3. System calculates P&L against cost basis
4. System displays formatted portfolio with totals

**Output**:
```
PORTFOLIO OVERVIEW
======================================================================

  Total Value:     $125,847.32
  Cost Basis:      $98,500.00
  Unrealized P&L:  +$27,347.32 (+27.76%)

----------------------------------------------------------------------
  Asset     Quantity        Price        Value              P&L
----------------------------------------------------------------------
  Ledger (Cold Storage)
  BTC      1.20000000    $97,245.00   $116,694.00  +$26,694.00 (+29.65%)
  ETH      2.50000000     $3,180.00     $7,950.00     +$450.00 (+6.00%)

  Binance (Trading)
  SOL          10.0000      $125.00     $1,250.00     +$250.00 (+25.00%)
----------------------------------------------------------------------

ASSET TOTALS
 BTC: 1.2 ($116,694)  |  ETH: 2.5 ($7,950)  |  SOL: 10 ($1,250)
```

### UC-2: Add Cold Wallet Holdings

**Actor**: Security-conscious investor
**Precondition**: Account and category exist
**Flow**:
1. User creates hardware wallet account
2. User adds wallet address for tracking
3. User manually adds holdings with cost basis
4. System stores in local SQLite database

**Commands**:
```bash
# Create the account
cryptofolio account add "Ledger Nano X" \
  --type hardware_wallet \
  --category cold-storage

# Add wallet address (for reference)
cryptofolio account address add "Ledger Nano X" bitcoin \
  "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh" \
  --label "Main BTC"

# Add holdings with cost basis
cryptofolio holdings add BTC 1.5 \
  --account "Ledger Nano X" \
  --cost 42000
```

### UC-3: Sync Exchange Holdings

**Actor**: Active trader
**Precondition**: Binance API keys configured
**Flow**:
1. User configures API credentials (read-only)
2. User runs sync command
3. System fetches balances via Binance API
4. System updates local holdings

**Commands**:
```bash
# Configure API (one-time)
cryptofolio config set binance.api_key "your_api_key"
cryptofolio config set binance.api_secret "your_api_secret"

# Create exchange account with sync enabled
cryptofolio account add "Binance Main" \
  --type exchange \
  --category trading \
  --sync

# Sync holdings
cryptofolio sync --account "Binance Main"

# Output:
# i Syncing 'Binance Main'...
# + BTC 0.05432100
# + ETH 1.25000000
# + USDT 500.00000000
# ✓ Synced 3 assets from 'Binance Main'
```

### UC-4: Record Transactions

**Actor**: Any investor tracking trades
**Flow**:
1. User records buy/sell/transfer transactions
2. System updates holdings and cost basis
3. System maintains transaction history

**Commands**:
```bash
# Record a purchase
cryptofolio tx buy BTC 0.1 \
  --account "Binance Main" \
  --price 95000 \
  --notes "DCA purchase"

# Record a sale
cryptofolio tx sell ETH 0.5 \
  --account "Binance Main" \
  --price 3200

# Transfer between accounts
cryptofolio tx transfer BTC 0.5 \
  --from "Binance Main" \
  --to "Ledger Nano X" \
  --fee 0.0001

# View transaction history
cryptofolio tx list --limit 10
```

### UC-5: Import Historical Data

**Actor**: User migrating from spreadsheet
**Flow**:
1. User exports transactions to CSV
2. User runs import command
3. System parses and validates data
4. System creates transactions and updates holdings

**Commands**:
```bash
# CSV format:
# date,type,asset,quantity,price_usd,fee,notes
# 2024-01-15,buy,BTC,0.5,45000,0.001,First purchase
# 2024-02-01,sell,ETH,1.0,3200,5.00,Taking profits

cryptofolio import transactions.csv --account "Ledger Nano X"
# ✓ Imported 47 transactions
```

### UC-6: Quick Price Check

**Actor**: Any user
**Flow**:
1. User requests current price
2. System fetches from Binance API
3. System displays formatted price

**Commands**:
```bash
# Single asset
cryptofolio price BTC
# BTC: $97,245.00

# Multiple assets
cryptofolio price BTC ETH SOL
# Symbol      Price
# ---------------------------
# BTC         $97,245.00
# ETH         $3,180.00
# SOL         $125.00

# Detailed market data
cryptofolio market BTC --24h
# BTC / USDT
#
#   Price: $97,245.00
#
# 24h Statistics
#
#   Change: +$2,145.00 (+2.25%)
#   High: $98,500.00
#   Low: $94,800.00
#   Volume: 12,543.57 BTC
#   Quote Volume: $1,215,535,260.00
```

---

## Real-Life Usage Examples

### Morning Routine Check

```bash
# Quick portfolio check before market opens
$ cryptofolio portfolio

PORTFOLIO OVERVIEW
======================================================================

  Total Value:     $52,847.32
  Cost Basis:      $48,500.00
  Unrealized P&L:  +$4,347.32 (+8.96%)

# Check if any significant price moves overnight
$ cryptofolio market BTC --24h | grep Change
  Change: +$1,245.00 (+1.30%)
```

### Weekly DCA Script

```bash
#!/bin/bash
# weekly-dca.sh - Run every Sunday

# Record this week's DCA purchase
cryptofolio tx buy BTC 0.01 \
  --account "Binance Main" \
  --price $(cryptofolio price BTC --json | jq -r '.price') \
  --notes "Weekly DCA $(date +%Y-%m-%d)"

# Sync to get updated balance
cryptofolio sync --account "Binance Main"

# Log portfolio value
echo "$(date): $(cryptofolio portfolio --json | jq -r '.total_value_usd')" >> ~/portfolio-log.txt
```

### Moving to Cold Storage

```bash
# After accumulating on exchange, move to cold storage

# Check current Binance balance
$ cryptofolio holdings list --account "Binance Main"
# Asset     Quantity        Cost Basis    Account
# BTC       0.25000000      $92,000       Binance Main

# Transfer to Ledger (record the transaction)
$ cryptofolio tx transfer BTC 0.24 \
    --from "Binance Main" \
    --to "Ledger Nano X" \
    --fee 0.0001 \
    --notes "Monthly cold storage transfer"

# ✓ Recorded transfer: 0.24 BTC from 'Binance Main' to 'Ledger Nano X'

# Verify holdings updated
$ cryptofolio holdings list
# Asset     Quantity        Cost Basis    Account
# BTC       0.00990000      $92,000       Binance Main
# BTC       0.24000000      $92,000       Ledger Nano X
```

### Tax Season Preparation

```bash
# Export all transactions for the year
$ cryptofolio tx export 2024-transactions.csv --from 2024-01-01 --to 2024-12-31

# Export specific account for the year
$ cryptofolio tx export binance-2024.csv --account "Binance Main" --from 2024-01-01 --to 2024-12-31

# Export specific asset trades
$ cryptofolio tx export btc-trades-2024.csv --asset BTC --from 2024-01-01 --to 2024-12-31

# View realized P&L (future feature)
$ cryptofolio pnl --realized --year 2024
# Realized P&L for 2024:
#   Total Proceeds:  $15,420.00
#   Cost Basis:      $12,800.00
#   Realized Gain:   $2,620.00
```

### CI/CD Integration (Monitoring)

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

## Technical Design

### Architecture Overview

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
│  │  price   │ │ account  │ │ holdings │ │portfolio │ │   sync   │  │
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
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │  Portfolio   │  │   Category   │  │       P&L Calculator     │  │
│  │  - Aggregate │  │  - Trading   │  │  - Unrealized            │  │
│  │  - By Account│  │  - Cold      │  │  - Realized (FIFO)       │  │
│  │  - By Category│ │  - Hot       │  │  - Per Asset/Account     │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
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
│  └────────────┘  │  │  └────────────┘  │  │  └────────────────┘  │
│                  │  │                  │  │                      │
│  ~/.config/      │  │  HTTPS + HMAC    │  │  ~/.config/          │
│  cryptofolio/    │  │  SHA256 Auth     │  │  cryptofolio/        │
│  database.sqlite │  │                  │  │  config.toml         │
└──────────────────┘  └──────────────────┘  └──────────────────────┘
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    PORTFOLIO VIEW FLOW                          │
└─────────────────────────────────────────────────────────────────┘

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

### Database Schema

```sql
┌─────────────────────────────────────────────────────────────────┐
│                      DATABASE SCHEMA                            │
└─────────────────────────────────────────────────────────────────┘

categories                    accounts
┌────────────────┐           ┌─────────────────────────┐
│ id (PK)        │◀──────────│ category_id (FK)        │
│ name           │           │ id (PK)                 │
│ sort_order     │           │ name                    │
│ created_at     │           │ account_type            │
└────────────────┘           │ config (JSON)           │
                             │ sync_enabled            │
                             │ created_at              │
                             └───────────┬─────────────┘
                                         │
              ┌──────────────────────────┼──────────────────────────┐
              │                          │                          │
              ▼                          ▼                          ▼
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│  wallet_addresses   │    │      holdings       │    │    transactions     │
├─────────────────────┤    ├─────────────────────┤    ├─────────────────────┤
│ id (PK)             │    │ id (PK)             │    │ id (PK)             │
│ account_id (FK)     │    │ account_id (FK)     │    │ tx_type             │
│ blockchain          │    │ asset               │    │ from_account_id(FK) │
│ address             │    │ quantity            │    │ from_asset          │
│ label               │    │ avg_cost_basis      │    │ from_quantity       │
│ created_at          │    │ updated_at          │    │ to_account_id (FK)  │
└─────────────────────┘    └─────────────────────┘    │ to_asset            │
                                                      │ to_quantity         │
                                                      │ price_usd           │
                                                      │ fee / fee_asset     │
                                                      │ timestamp           │
                                                      └─────────────────────┘
```

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Language | Rust | Performance, safety, single binary |
| CLI Framework | clap v4 | Derive macros, excellent UX |
| Async Runtime | Tokio | Industry standard |
| HTTP Client | reqwest | Mature, TLS support |
| Database | SQLite + sqlx | Embedded, type-safe |
| Decimals | rust_decimal | Financial precision |
| Config | TOML + dirs | Human-readable, XDG paths |

### Key Design Decisions

1. **Local-First**: All data stored locally in SQLite. No cloud sync, no account required.

2. **Read-Only Exchange Access**: Binance integration uses only read endpoints. No trading via API (yet).

3. **Cost Basis Tracking**: Average cost method by default. FIFO/LIFO planned for tax optimization.

4. **Double-Entry Transactions**: Transfers have source and destination, enabling accurate tracking.

5. **Category System**: Flexible grouping (Trading, Cold Storage, DeFi) for organization.

---

## Security Considerations

### Threat Model

| Threat | Mitigation |
|--------|------------|
| API key theft | Keys stored in local config, not transmitted except to Binance |
| Shell history exposure | Planned: Read secrets from stdin, not arguments |
| Database tampering | SQLite file permissions (0600), planned encryption |
| Man-in-the-middle | HTTPS only, certificate validation |
| Malicious price data | Single trusted source (Binance), sanity checks |

### API Key Security

```
CURRENT (v0.1):
┌─────────────────────────────────────────────────────────────────┐
│  ⚠️  API keys stored in plaintext in config.toml                │
│  ⚠️  'config set' command visible in shell history              │
└─────────────────────────────────────────────────────────────────┘

PLANNED (v0.2):
┌─────────────────────────────────────────────────────────────────┐
│  ✓ Read secrets from stdin: echo $KEY | cryptofolio config...   │
│  ✓ Support keyring/keychain integration                        │
│  ✓ Warn if secrets passed as arguments                         │
└─────────────────────────────────────────────────────────────────┘
```

### Binance API Permissions

**Required Permissions** (minimum):
- ✅ Enable Reading (view balances, trades)

**NOT Required**:
- ❌ Enable Spot & Margin Trading
- ❌ Enable Withdrawals
- ❌ Enable Internal Transfer

**Recommendation**: Create a dedicated API key with read-only permissions.

### Data Privacy

- **No telemetry**: Zero usage data collected
- **No network calls** except to Binance API when explicitly requested
- **Local storage**: All data in `~/.config/cryptofolio/`
- **No cloud backup**: User responsible for backing up database

---

## 📢 What's New in v0.2

### 🔐 Secure Secret Handling (February 2026)

**Critical Security Update:** API keys and secrets are no longer exposed in shell history!

#### New `config set-secret` Command

```bash
# Interactive mode with hidden input
cryptofolio config set-secret binance.api_secret
Enter secret (hidden): ********

# Automation modes
echo "secret" | cryptofolio config set-secret binance.api_secret  # Stdin
cryptofolio config set-secret binance.api_secret --secret-file ~/.secrets/key  # File
cryptofolio config set-secret binance.api_secret --from-env MY_SECRET  # Env var
```

#### Security Improvements

✅ **Shell history protection** - Secrets never appear in bash/zsh history
✅ **Process list protection** - Secrets not visible in `ps` output
✅ **File permissions** - Auto-enforced 0600 on Unix/macOS/Linux
✅ **User education** - Comprehensive warnings about READ-ONLY API keys
✅ **Multiple input methods** - Interactive, stdin, file, environment variable

**IMPORTANT:** This version emphasizes using **READ-ONLY** API keys only. Encrypted keychain storage coming in v0.3!

See [docs/SECURE_SECRETS.md](docs/SECURE_SECRETS.md) for detailed security guide.

### 🤖 JSON Output for All Commands (February 2026)

**Machine-Readable Output:** All query commands now support `--json` flag for LLM/MCP integration and automation!

#### Commands with JSON Support

All data-retrieval commands now output structured JSON when using the `--json` flag:

```bash
# Portfolio (existing)
cryptofolio portfolio --json

# Price data (existing)
cryptofolio price BTC ETH --json

# Market data (existing)
cryptofolio market BTCUSDT --json

# Holdings (NEW in v0.2)
cryptofolio holdings list --json
cryptofolio holdings list --account Binance --json

# Accounts (NEW in v0.2)
cryptofolio account list --json
cryptofolio account show Binance --json

# Transactions (NEW in v0.2)
cryptofolio tx list --json
cryptofolio tx list --account Binance --limit 50 --json

# Configuration (NEW in v0.2)
cryptofolio config show --json
```

#### Use Cases

**1. LLM/AI Integration** - Claude, ChatGPT, or custom agents can now parse portfolio data:
```bash
# Ask Claude about your portfolio
cryptofolio portfolio --json | claude-cli "Analyze my portfolio and suggest rebalancing"
```

**2. MCP Server Integration** - Build Model Context Protocol tools:
```javascript
// MCP tool definition
{
  "name": "get_crypto_portfolio",
  "description": "Get current cryptocurrency portfolio",
  "inputSchema": { "type": "object", "properties": {} },
  "handler": () => execSync("cryptofolio portfolio --json").toString()
}
```

**3. Automation & Scripting** - Process data with jq, Python, Node.js:
```bash
# Extract total value for monitoring
cryptofolio portfolio --json | jq -r '.total_value_usd'

# Alert if portfolio drops below threshold
TOTAL=$(cryptofolio portfolio --json --quiet | jq -r '.total_value_usd' | tr -d '$' | tr -d ',')
if (( $(echo "$TOTAL < 50000" | bc -l) )); then
  notify-send "Portfolio Alert" "Total value below $50k!"
fi

# Log daily snapshots
echo "$(date): $(cryptofolio portfolio --json)" >> ~/portfolio-history.jsonl
```

**4. Dashboard Integration** - Feed data to web dashboards or monitoring tools:
```python
import subprocess
import json

# Get portfolio data
result = subprocess.run(
    ["cryptofolio", "portfolio", "--json"],
    capture_output=True,
    text=True
)
portfolio = json.loads(result.stdout)

# Send to monitoring service
send_to_grafana(portfolio["total_value_usd"])
```

#### JSON Output Format

All JSON outputs follow consistent patterns:

- **Numbers as strings** - Preserves precision for financial data
- **ISO 8601 timestamps** - Standard date/time format
- **Null-safe fields** - Optional fields use `null` instead of omission
- **Pretty-printed** - Human-readable formatting by default

**Example Portfolio JSON:**
```json
{
  "total_value_usd": "61442.89",
  "total_cost_basis": "29317.39",
  "unrealized_pnl": "32125.50",
  "unrealized_pnl_percent": "109.57",
  "entries": [
    {
      "account_name": "Binance",
      "holdings": [
        {
          "asset": "BTC",
          "quantity": "0.09121000",
          "price_usd": "70253.98",
          "value_usd": "6407.86",
          "cost_basis": null,
          "pnl": null
        }
      ]
    }
  ]
}
```

See examples in [docs/VALIDATION_GUIDE.md](docs/VALIDATION_GUIDE.md) for more JSON usage patterns.

---

## Release Announcement

# 🚀 Cryptofolio v0.1.0 - Initial Release

**Your crypto portfolio, in your terminal, under your control.**

We're excited to announce the first release of Cryptofolio, a command-line tool for managing cryptocurrency portfolios across exchanges and wallets.

## Why Cryptofolio?

If you're a developer or power user who:
- Lives in the terminal
- Holds crypto across multiple locations
- Cares about privacy and self-custody
- Wants accurate P&L tracking
- Needs scriptable portfolio access

...then Cryptofolio is for you.

## ✨ Key Features

### 📊 Unified Portfolio View
See all your holdings in one place with real-time P&L calculations:

```
$ cryptofolio portfolio

PORTFOLIO OVERVIEW
======================================================================
  Total Value:     $61,442.89
  Cost Basis:      $29,317.39
  Unrealized P&L:  +$32,125.50 (+109.57%)

----------------------------------------------------------------------
  Asset         Quantity         Price         Value              P&L
----------------------------------------------------------------------
  Binance
  BTC         0.09121000     $70,253.98      $6,407.86                -
  ETH             2.4594      $2,088.30      $5,136.11                -
  NIGHT         26016.95         $0.05      $1,396.97                -
  Cold Storage
  BTC         0.09112651     $70,253.98      $6,402.00                -
----------------------------------------------------------------------
```

### 💰 Multi-Location Support
Track assets across:
- **Exchanges**: Binance (with auto-sync)
- **Hardware Wallets**: Ledger, Trezor
- **Software Wallets**: MetaMask, Trust Wallet

### 📈 Real-Time Market Data + Binance Alpha
Fetch prices from both Binance Spot and **Binance Alpha** markets:
```
$ cryptofolio price BTC ETH NIGHT
Symbol      Price
---------------------------
BTC         $70,253.98
ETH         $2,088.30
NIGHT       $0.05          # ← From Binance Alpha API
```

### 📝 Transaction Tracking
Record buys, sells, transfers, and swaps with automatic cost basis updates:
```
$ cryptofolio tx buy BTC 0.1 --account Binance --price 95000
✓ Recorded buy: 0.1 BTC @ $95,000.00 in 'Binance'
```

### 🔒 Privacy First
- All data stored locally (`~/.config/cryptofolio/`)
- No cloud accounts, no telemetry
- Read-only exchange API access

### 🧪 Testnet Support
Practice with Binance testnet before using real funds:
```
$ cryptofolio config use-testnet
✓ Testnet mode enabled
```

## 📦 Installation

### From Source (Rust required)
```bash
git clone https://github.com/yourusername/cryptofolio.git
cd cryptofolio
cargo build --release
cp target/release/cryptofolio /usr/local/bin/
```

### Verify Installation
```bash
cryptofolio --version
# cryptofolio 0.1.0
```

## 🚀 Quick Start

### 1. Check a Price
```bash
cryptofolio price BTC
```

### 2. Create an Account
```bash
cryptofolio account add "My Ledger" --type hardware_wallet --category cold-storage
```

### 3. Add Holdings
```bash
cryptofolio holdings add BTC 0.5 --account "My Ledger" --cost 45000
```

### 4. View Portfolio
```bash
cryptofolio portfolio
```

### 5. (Optional) Connect Binance

**⚠️ SECURITY: Use the new `config set-secret` command to avoid exposing secrets in shell history!**

```bash
# Set API credentials SECURELY (v0.2+)
cryptofolio config set-secret binance.api_key
# Enter key (hidden): ********

cryptofolio config set-secret binance.api_secret
# Enter secret (hidden): ********

# Create synced account
cryptofolio account add "Binance" --type exchange --category trading --sync

# Sync holdings
cryptofolio sync
```

**IMPORTANT:** Only use **READ-ONLY** API keys! See [Security Best Practices](#-security-best-practices) below.

---

## 🔒 Security Best Practices

### API Key Security (v0.2+)

Cryptofolio v0.2 introduces **secure secret handling** to protect your API keys.

#### ✅ Setting API Keys Securely

**Use `config set-secret` instead of `config set`:**

```bash
# ✅ SECURE (v0.2+) - Hidden input, no shell history
cryptofolio config set-secret binance.api_secret
Enter secret (hidden): ********

# ❌ INSECURE (old method) - Visible in shell history!
cryptofolio config set binance.api_secret "YOUR_SECRET"  # DON'T DO THIS!
```

**Multiple input methods for different scenarios:**

```bash
# Interactive (recommended for first-time setup)
cryptofolio config set-secret binance.api_secret

# From stdin (for scripts/automation)
echo "secret" | cryptofolio config set-secret binance.api_secret

# From file (for deployment)
cryptofolio config set-secret binance.api_secret --secret-file ~/.secrets/key

# From environment variable (for containers)
cryptofolio config set-secret binance.api_secret --from-env BINANCE_SECRET
```

#### 🔐 Binance API Key Setup

**When creating your Binance API key:**

1. Go to Binance → API Management → Create API
2. **Enable ONLY:**
   - ✅ Enable Reading
3. **DISABLE (CRITICAL):**
   - ❌ Enable Spot & Margin Trading
   - ❌ Enable Withdrawals
   - ❌ Enable Internal Transfer
   - ❌ Enable Futures

**Why READ-ONLY?**

Cryptofolio v0.2 stores API keys in **plaintext** in `~/.config/cryptofolio/config.toml` (file permissions: `0600`).

If your computer is compromised:
- **READ-ONLY keys:** Attacker can only view portfolio → No financial loss ✅
- **WRITE permissions:** Attacker can steal funds → Total loss ❌

**Encrypted keychain storage is coming in v0.3!**

#### 📁 File Permissions

Cryptofolio automatically sets secure permissions on Unix/macOS/Linux:

```bash
# Config file is automatically set to 0600 (owner read/write only)
$ ls -la ~/.config/cryptofolio/config.toml
-rw-------  1 user  group  512 Feb 16 10:30 config.toml
```

On Windows, ensure only your user account has read access.

#### 📚 More Information

See [docs/SECURE_SECRETS.md](docs/SECURE_SECRETS.md) for:
- Detailed security guide
- Integration with password managers
- Troubleshooting
- Best practices checklist

---

## 🤖 Interactive Shell & AI Features

### Interactive Shell Mode
Start an interactive session with tab completion and command history:
```bash
$ cryptofolio shell

  🪙 Cryptofolio v0.1.0
  AI-Powered Portfolio Assistant

  💰 Portfolio: $61,442.89 (+109.57%)
  🧪 Testnet  •  🦙 AI Ready (Ollama)

  Type 'help' for commands, or describe what you want to do.
  Use 'status' for full system diagnostics.
  Press Ctrl+C to cancel, 'exit' to quit.

you>
```

### Natural Language Commands
In shell mode, you can use natural language:
```
you> What's the price of Bitcoin?
you> Show my portfolio
you> I bought 0.1 BTC today at $95,000
you> How much ETH do I have?
```

### AI Providers
Cryptofolio supports multiple AI backends:

| Provider | Mode | Setup |
|----------|------|-------|
| **Claude** (Cloud) | Online | Set `ANTHROPIC_API_KEY` environment variable |
| **Ollama** (Local) | Offline | Run Ollama locally with `llama3.2:3b` model |
| **Hybrid** | Auto | Uses Ollama for simple tasks, Claude for complex |
| **Pattern-based** | Fallback | Works without any AI - uses regex matching |

### System Status
Check your configuration and AI provider status:
```bash
$ cryptofolio status

  📊 System Status
  ─────────────────────────────────────
  📁 Config       ~/.config/cryptofolio/config.toml
  🗄️ Database     ~/.config/cryptofolio/database.sqlite
  🧪 Mode         Testnet (safe)

  🤖 AI Providers
  ─────────────────────────────────────
  ☁️ Claude       Offline (API key not configured)
  🦙 Ollama       Connected (llama3.2:3b)

  ⚡ AI Mode      Hybrid (Local + Cloud)
  🎯 Active       Ollama only (llama3.2:3b)
```

## 📋 Available Commands

| Command | Description |
|---------|-------------|
| `price` | Get current cryptocurrency prices |
| `market` | Get detailed market data with 24h stats |
| `account` | Manage accounts (add, remove, list) |
| `category` | Manage categories |
| `holdings` | Manage holdings (add, remove, move) |
| `portfolio` | View portfolio with P&L |
| `tx` | Record transactions (buy, sell, transfer, swap) |
| `sync` | Sync holdings from exchange |
| `import` | Import transactions from CSV |
| `config` | Manage configuration (includes `set-secret` for secure API keys) |
| `shell` | Start interactive shell with AI-powered natural language |
| `status` | Show system diagnostics and AI provider status |

### Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Output in JSON format for scripting |
| `--quiet` | Suppress non-essential output |
| `--testnet` | Use Binance testnet |
| `--yes` | Skip confirmation prompts |
| `--dry-run` | Preview changes without committing (tx commands) |

## 🔧 CLI Best Practices

Cryptofolio follows the [CLI Guidelines](https://clig.dev) for a great command-line experience:

### Output Modes
```bash
# JSON output for scripting/automation
cryptofolio portfolio --json
cryptofolio price BTC ETH --json

# Quiet mode - suppress non-essential output
cryptofolio sync --quiet

# Combine for CI/CD pipelines
cryptofolio portfolio --json --quiet | jq '.total_value_usd'
```

### Safe Destructive Operations
```bash
# Confirmation prompts protect against accidents
$ cryptofolio account remove "My Wallet"
⚠ This will delete account 'My Wallet' and all associated holdings. Continue? [y/N]

# Skip prompts with --yes flag
cryptofolio account remove "My Wallet" --yes
```

### Dry-Run Mode
```bash
# Preview transaction changes without committing
$ cryptofolio tx buy BTC 0.5 --account Binance --price 95000 --dry-run
[DRY RUN] Would record: buy 0.5 BTC @ $95,000.00 in 'Binance'
```

### Progress Indicators
Long-running operations show progress:
```bash
$ cryptofolio sync
Syncing account 'Binance'... ⠋
✓ Synced 5 assets from 'Binance'

$ cryptofolio import data.csv --account "My Wallet"
Importing transactions... ████████████████████ 100% (34/34)
✓ Imported 34 transactions, 2 errors
```

### Environment Variables
Configure via environment for CI/CD:
```bash
export CRYPTOFOLIO_TESTNET=true
export CRYPTOFOLIO_JSON=true
cryptofolio portfolio  # Uses testnet, outputs JSON
```

## 🗺️ Roadmap

### v0.2 (Current - February 2026)
- [x] **Secure secret input** (stdin, file, env, interactive)
- [x] **File permissions enforcement** (auto 0600 on Unix)
- [x] **Security warnings** for READ-ONLY API keys
- [x] **JSON output for all query commands** (portfolio, price, market, holdings, account, tx, config)
- [x] **Transaction history export (CSV)** with filtering and date ranges
- [x] **Help text improvements** with comprehensive examples and workflows
- [ ] Customizable number formatting

### v0.3 (Next)
- [ ] **Encrypted keychain storage** (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- [ ] Multiple exchange support (Coinbase, Kraken)
- [ ] Realized P&L calculations
- [ ] Tax report export
- [ ] `--quiet` flag for all commands
- [ ] Progress indicators for long operations

### v0.4
- [ ] Interactive TUI dashboard
- [ ] Price alerts with notifications
- [ ] DCA automation
- [ ] "Did you mean?" suggestions for errors

## 🤝 Contributing

Contributions welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**Built with 🦀 Rust and ❤️ for the crypto community.**

```
   ___                  _         __       _ _
  / __\ __ _   _ _ __ | |_ ___  / _| ___ | (_) ___
 / / | '__| | | | '_ \| __/ _ \| |_ / _ \| | |/ _ \
/ /__| |  | |_| | |_) | || (_) |  _| (_) | | | (_) |
\____/_|   \__, | .__/ \__\___/|_|  \___/|_|_|\___/
           |___/|_|
```
