# 🪙 Cryptofolio Validation Guide

**Version:** v0.2 (February 2026)
**Status:** Updated with secure secret handling

Welcome! This guide will help you build, run, and validate **Cryptofolio** — a command-line tool for managing your cryptocurrency portfolio across multiple exchanges and wallets.

## 🆕 What's New in v0.2

- **🔐 Secure Secret Handling:** New `config set-secret` command prevents API keys from appearing in shell history
- **✅ File Permissions:** Automatic enforcement of secure file permissions (0600 on Unix)
- **⚠️ Security Warnings:** Comprehensive warnings about using READ-ONLY API keys
- **🤖 JSON Output:** All query commands now support `--json` flag for LLM/MCP integration and automation
- See [docs/SECURE_SECRETS.md](SECURE_SECRETS.md) for detailed security guide

---

## 📖 What is Cryptofolio?

Cryptofolio is a CLI (Command Line Interface) application that helps you:

- 📊 **Track holdings** across exchanges (Binance), hardware wallets (Ledger), and software wallets (MetaMask)
- 💰 **Monitor prices** in real-time from Binance
- 📈 **Calculate P&L** (Profit & Loss) on your investments
- 🔄 **Sync balances** automatically from connected exchanges
- 📝 **Record transactions** (buys, sells, transfers)
- 🤖 **Use natural language** to interact (e.g., "I bought 0.1 BTC today")
- 🔌 **Output JSON** for automation, scripting, and LLM/MCP integration

Think of it as a personal finance app for crypto, but running in your terminal.

---

## 🎯 What You'll Do in This Guide

By following this guide, you will:

1. **Build** the application from source code
2. **Configure** it to use Binance testnet (safe, no real money)
3. **Test** all major features work correctly
4. **Clean up** the test data when finished

**Estimated time:** 20-30 minutes

---

## 🧭 Before You Begin

### Who Is This Guide For?

- ✅ Developers validating changes before release
- ✅ Contributors testing the application locally
- ✅ Anyone curious to try Cryptofolio

### What You'll Need

| Requirement | Why | How to Check |
|-------------|-----|--------------|
| **macOS or Linux** | The app runs on Unix-based systems | You're probably on one if you cloned this repo |
| **Rust toolchain** | To compile the source code | Run `rustc --version` in terminal |
| **Git** | To clone the repository | Run `git --version` in terminal |
| **Internet connection** | To fetch prices from Binance | Try opening any website |

> 💡 **Don't have Rust installed?** Visit [rustup.rs](https://rustup.rs/) and follow the one-line installer.

---

## 🏗️ Part 1: Building the Application

### Step 1.1: Open Your Terminal

- **macOS:** Press `Cmd + Space`, type "Terminal", press Enter
- **Linux:** Press `Ctrl + Alt + T` or find Terminal in your applications

You should see a command prompt like this:
```
your-username@computer ~ %
```

### Step 1.2: Navigate to the Project

If you've cloned this repository, you need to navigate into it.

```bash
cd ~/projects/cryptofolio
```

> ⚠️ **Not sure where you cloned it?** Run `find ~ -name "cryptofolio" -type d 2>/dev/null` to search for it.

**Verify you're in the right place:**
```bash
ls -la
```

You should see files like:
```
Cargo.toml
Cargo.lock
src/
docs/
tests/
...
```

### Step 1.3: Build in Release Mode

Now, let's compile the application. Release mode creates an optimized binary.

```bash
cargo build --release
```

> ⏳ **First time building?** This will download dependencies and may take 2-5 minutes. You'll see lots of "Compiling..." messages — that's normal!

**Expected output (at the end):**
```
   Compiling cryptofolio v0.1.0
    Finished `release` profile [optimized] target(s) in X.XXs
```

### Step 1.4: Verify the Build

Let's make sure the application runs:

```bash
./target/release/cryptofolio --version
```

**Expected output:**
```
cryptofolio 0.1.0
```

🎉 **Congratulations!** You've successfully built Cryptofolio!

---

## ⚙️ Part 2: Initial Configuration

Before testing, we need to configure the application to use **testnet** (a safe sandbox with fake money).

### Step 2.1: Enable Testnet Mode

```bash
./target/release/cryptofolio config use-testnet
```

**Expected output:**
```
Testnet mode enabled.
```

### Step 2.2: Get Binance Testnet API Keys

> 🔐 **Why testnet?** Testnet uses fake money so you can safely test without risking real funds.

1. Go to [testnet.binance.vision](https://testnet.binance.vision/)
2. Log in with GitHub
3. Click "Generate HMAC_SHA256 Key"
4. Copy both the **API Key** and **Secret Key**

### Step 2.3: Configure API Keys Securely

**⚠️ IMPORTANT:** Use the new `config set-secret` command (v0.2+) to avoid exposing secrets in shell history!

**Interactive method (recommended):**
```bash
# Set API key (you'll be prompted to enter it hidden)
./target/release/cryptofolio config set-secret binance.api_key
Enter secret (hidden): [paste your API key]

# Set API secret
./target/release/cryptofolio config set-secret binance.api_secret
Enter secret (hidden): [paste your API secret]
```

**Alternative: From stdin (for automation):**
```bash
echo "YOUR_API_KEY" | ./target/release/cryptofolio config set-secret binance.api_key
echo "YOUR_SECRET_KEY" | ./target/release/cryptofolio config set-secret binance.api_secret
```

> 🔐 **Why use `set-secret`?** It prevents your API keys from appearing in shell history, which is a security risk. The old `config set` command will warn you if you try to use it for secrets.

### Step 2.4: Verify Configuration

```bash
./target/release/cryptofolio config show
```

**Expected output:**
```
[general]
use_testnet = true
currency = "USD"

[binance]
api_key = "***" (set)
api_secret = "***" (set)

[display]
color = true
decimals = 8
```

✅ **Configuration complete!**

---

## 🧪 Part 3: CLI Validation

Now let's test each feature of the application.

> 📝 **Tip:** Each test section shows the command to run and what you should expect to see.

---

### Test V0: System Status Check 🔍

**What we're testing:** Can we see the system configuration and AI provider status?

```bash
./target/release/cryptofolio status
```

**Expected output:**
```
  📊 System Status
  ─────────────────────────────────────
  📁 Config       ~/.config/cryptofolio/config.toml
  🗄️ Database     ~/.config/cryptofolio/database.sqlite
  🧪 Mode         Testnet (safe)

  🤖 AI Providers
  ─────────────────────────────────────
  ☁️ Claude       Offline (API key not configured)
  🦙 Ollama       Connected (llama3.2:3b)  OR  Offline (Not running)

  ⚡ AI Mode      Hybrid (Local + Cloud)
  🎯 Active       Ollama only (llama3.2:3b)  OR  Pattern-based (no LLM available)
```

> 💡 The AI provider status will vary depending on your setup. If Ollama isn't running, you'll see "Pattern-based" as the active mode - this is normal and means natural language will use regex pattern matching.

---

### Test V1: Check Cryptocurrency Prices 💵

**What we're testing:** Can the app fetch live prices from Binance?

```bash
./target/release/cryptofolio price BTC ETH SOL
```

**Expected output:**
```
SYMBOL    PRICE
BTC       $XX,XXX.XX
ETH       $X,XXX.XX
SOL       $XXX.XX
```

> 💡 Prices will vary — you should see current market prices.

---

### Test V2: Create Accounts 📁

**What we're testing:** Can we create different types of accounts?

```bash
# Create an exchange account
./target/release/cryptofolio account add "Binance Test" --type exchange --category trading

# Create a hardware wallet
./target/release/cryptofolio account add "Ledger Nano" --type hardware-wallet --category cold-storage

# Create a software wallet
./target/release/cryptofolio account add "MetaMask" --type software-wallet --category hot-wallets
```

**Verify all accounts were created:**
```bash
./target/release/cryptofolio account list
```

**Expected output:**
```
ACCOUNT        TYPE              CATEGORY       SYNC
Binance Test   exchange          trading        No
Ledger Nano    hardware-wallet   cold-storage   No
MetaMask       software-wallet   hot-wallets    No
```

---

### Test V3: Add Holdings 📦

**What we're testing:** Can we add cryptocurrency holdings to accounts?

```bash
# Add 0.5 BTC to Ledger (bought at $45,000)
./target/release/cryptofolio holdings add BTC 0.5 --account "Ledger Nano" --cost 45000

# Add 2.0 ETH to MetaMask (bought at $2,800)
./target/release/cryptofolio holdings add ETH 2.0 --account "MetaMask" --cost 2800

# Add 0.1 BTC to Binance (bought at $62,000)
./target/release/cryptofolio holdings add BTC 0.1 --account "Binance Test" --cost 62000

# Add 10 SOL to Binance (bought at $150)
./target/release/cryptofolio holdings add SOL 10 --account "Binance Test" --cost 150
```

**Verify holdings:**
```bash
./target/release/cryptofolio holdings list
```

**Expected output:** A table showing all holdings with quantities and cost basis.

---

### Test V4: View Portfolio 📊

**What we're testing:** Can we see our total portfolio value and P&L?

```bash
# Full portfolio view
./target/release/cryptofolio portfolio
```

**Expected output:**
```
PORTFOLIO SUMMARY
─────────────────────────────────────
Total Value:     $XX,XXX.XX
Cost Basis:      $XX,XXX.XX
Unrealized P&L:  +$X,XXX.XX (+XX.XX%)

HOLDINGS
...
```

**Try grouping by account:**
```bash
./target/release/cryptofolio portfolio --by-account
```

**Try grouping by category:**
```bash
./target/release/cryptofolio portfolio --by-category
```

---

### Test V5: Record Transactions 📝

**What we're testing:** Can we record buy/sell transactions?

```bash
# Record a buy transaction
./target/release/cryptofolio tx buy ADA 100 --account "Binance Test" --price 0.45

# Record a sell transaction
./target/release/cryptofolio tx sell ETH 0.5 --account "MetaMask" --price 3200

# View transaction history
./target/release/cryptofolio tx list
```

**Expected output:** List of transactions with timestamps, types, and amounts.

---

### Test V6: Transfer Between Accounts 🔄

**What we're testing:** Can we move holdings between accounts?

```bash
# Check current BTC holdings
./target/release/cryptofolio holdings list

# Move 0.05 BTC from Binance to Ledger
./target/release/cryptofolio holdings move BTC 0.05 --from "Binance Test" --to "Ledger Nano" --yes

# Verify the transfer
./target/release/cryptofolio holdings list
```

**Expected result:**
- Binance Test: BTC reduced from 0.1 to 0.05
- Ledger Nano: BTC increased from 0.5 to 0.55

---

### Test V7: Import Transactions from CSV 📥

**What we're testing:** Can we import transactions from a CSV file?

The repository includes a sample CSV file at `tests/fixtures/sample_transactions.csv`.

```bash
# View the sample CSV (optional)
cat tests/fixtures/sample_transactions.csv

# Import transactions
./target/release/cryptofolio import tests/fixtures/sample_transactions.csv --account "Ledger Nano"

# Verify import
./target/release/cryptofolio tx list --limit 20
```

**Expected result:** 8 transactions imported from the CSV file.

---

### Test V8: JSON Output for Automation 🤖

**What we're testing:** Can all commands output machine-readable JSON for LLM/MCP integration and scripting?

**NEW in v0.2:** All query commands now support the `--json` flag for automation and AI integration.

#### V8.1: Portfolio JSON Output

```bash
# Get portfolio in JSON format
./target/release/cryptofolio portfolio --json
```

**Expected output:** Pretty-printed JSON with structure:
```json
{
  "total_value_usd": "112552.27",
  "total_cost_basis": "84445.00",
  "unrealized_pnl": "28107.27",
  "unrealized_pnl_percent": "33.28",
  "entries": [
    {
      "account_name": "Binance Test",
      "category_name": "Trading",
      "holdings": [
        {
          "asset": "BTC",
          "quantity": "0.05",
          "current_price": "68969.61",
          "current_value": "3448.48",
          "cost_basis": "62000",
          "unrealized_pnl": "348.48",
          "unrealized_pnl_percent": "11.24"
        }
      ]
    }
  ]
}
```

#### V8.2: Holdings JSON Output

```bash
# List all holdings in JSON
./target/release/cryptofolio holdings list --json

# Filter by account
./target/release/cryptofolio holdings list --account "Binance Test" --json
```

**Expected output:** Array of holdings:
```json
[
  {
    "asset": "BTC",
    "quantity": "0.05",
    "cost_basis": "62000",
    "account": "Binance Test",
    "account_id": "dc8c029a-680e-444c-96b8-a03a56110721"
  }
]
```

#### V8.3: Account JSON Output

```bash
# List all accounts in JSON
./target/release/cryptofolio account list --json

# Show specific account details
./target/release/cryptofolio account show "Binance Test" --json
```

**Expected output for list:**
```json
[
  {
    "name": "Binance Test",
    "account_type": "Exchange",
    "category": "Trading",
    "sync_enabled": false,
    "is_testnet": false
  }
]
```

**Expected output for show:**
```json
{
  "name": "Binance Test",
  "account_type": "Exchange",
  "category": "Trading",
  "is_testnet": false,
  "sync_enabled": false,
  "created_at": "2026-02-17T02:58:12.785695+00:00",
  "addresses": []
}
```

#### V8.4: Transaction JSON Output

```bash
# List transactions in JSON
./target/release/cryptofolio tx list --limit 5 --json
```

**Expected output:** Array of transactions:
```json
[
  {
    "id": 63,
    "timestamp": "2026-02-08T17:05:26.123148+00:00",
    "tx_type": "Buy",
    "from_account_id": null,
    "to_account_id": "dc8c029a-680e-444c-96b8-a03a56110721",
    "from_asset": null,
    "from_quantity": null,
    "to_asset": "ADA",
    "to_quantity": "100",
    "price_usd": "0.45",
    "fee": null,
    "fee_asset": null,
    "notes": null
  }
]
```

#### V8.5: Price JSON Output

```bash
# Get prices in JSON format
./target/release/cryptofolio price BTC ETH --json
```

**Expected output:**
```json
[
  {
    "symbol": "BTCUSDT",
    "price": "68969.61000000"
  },
  {
    "symbol": "ETHUSDT",
    "price": "2489.15000000"
  }
]
```

#### V8.6: Configuration JSON Output

```bash
# View config in JSON
./target/release/cryptofolio config show --json
```

**Expected output:**
```json
{
  "general": {
    "default_account": null,
    "use_testnet": true,
    "currency": "USD"
  },
  "binance": {
    "api_key_configured": true,
    "api_secret_configured": true
  },
  "display": {
    "color": true,
    "decimals": 8
  },
  "paths": {
    "config_dir": "/Users/you/.config/cryptofolio",
    "database": "/Users/you/.config/cryptofolio/database.sqlite"
  }
}
```

#### V8.7: Scripting with jq

Test JSON output with `jq` (install with `brew install jq` on macOS):

```bash
# Extract total portfolio value
./target/release/cryptofolio portfolio --json | jq -r '.total_value_usd'

# Get only BTC holdings
./target/release/cryptofolio holdings list --json | jq '.[] | select(.asset == "BTC")'

# Count transactions
./target/release/cryptofolio tx list --json | jq 'length'

# Alert if portfolio below threshold
TOTAL=$(./target/release/cryptofolio portfolio --json --quiet | jq -r '.total_value_usd' | tr -d '$' | tr -d ',')
if (( $(echo "$TOTAL < 100000" | bc -l) )); then
  echo "Portfolio below $100k: $$TOTAL"
fi
```

**Expected results:**
- All JSON outputs should be valid (parse without errors)
- Numbers returned as strings to preserve precision
- Timestamps in ISO 8601 format
- Empty results return `[]` for arrays

#### V8.8: LLM Integration Test (Optional)

If you have `jq` and want to test LLM-style queries:

```bash
# Prepare portfolio summary for AI
./target/release/cryptofolio portfolio --json | jq '{
  total_value: .total_value_usd,
  profit_loss: .unrealized_pnl,
  profit_percent: .unrealized_pnl_percent,
  num_holdings: (.entries | map(.holdings | length) | add)
}'
```

**Use case:** This JSON can be fed to Claude, ChatGPT, or MCP tools for portfolio analysis.

---

## 🤖 Part 4: NLP (Natural Language) Evaluation

Cryptofolio can understand natural language! Let's test the AI features.

### Step 4.1: Start Interactive Shell

```bash
./target/release/cryptofolio shell
```

**Expected welcome screen:**
```
  Cryptofolio v0.1.0
  AI-Powered Portfolio Assistant

  Portfolio: $XX,XXX.XX (+XX.XX%)
  Mode: Testnet
  AI: Hybrid

  Type 'help' for commands, or describe what you want to do.
  Press Ctrl+C to cancel, 'exit' to quit.

you>
```

### Step 4.2: Test Natural Language Commands

Try typing these phrases and observe the responses:

| You Type | Expected Behavior |
|----------|-------------------|
| `What's the price of Bitcoin?` | Shows BTC price |
| `How much is ETH?` | Shows ETH price |
| `Show my portfolio` | Displays portfolio summary |
| `What do I have?` | Lists your holdings |
| `sync` | Syncs with Binance |

### Step 4.3: Test Multi-Turn Conversation

Try this conversation (you can cancel at the confirmation step):

```
you> I bought some bitcoin today

  How much BTC did you buy?
  > 0.1

  What price per unit?
  > 95000

  Which account?
  > Binance Test

  Transaction: BUY
  Asset: BTC
  Quantity: 0.1
  Price: $95000.00
  Account: Binance Test
  Total: $9500.00

  Confirm? [Y/n] n
```

> 💡 Type `n` to cancel — we're just testing the conversation flow.

### Step 4.4: Test Out-of-Scope Handling

```
you> What's the weather like?
```

**Expected:** A polite message saying it can only help with crypto portfolio management.

### Step 4.5: Exit the Shell

```
you> exit
```

---

## 🧹 Part 5: Cleanup

After testing, you should clean up the test data.

### Option A: Full Reset (Recommended)

This removes all data and configuration:

```bash
rm -rf ~/.config/cryptofolio/
```

**Verify cleanup:**
```bash
./target/release/cryptofolio config show
```

You should see default configuration (you'll need to reconfigure for next test).

---

### Option B: Keep Configuration, Reset Data

This keeps your API keys but removes accounts and holdings:

```bash
rm -f ~/.config/cryptofolio/database.sqlite
```

---

### Option C: Remove Only Test Accounts

If you want to keep other data:

```bash
./target/release/cryptofolio account remove "Binance Test" --yes
./target/release/cryptofolio account remove "Ledger Nano" --yes
./target/release/cryptofolio account remove "MetaMask" --yes
```

---

## ✅ Validation Checklist

Use this checklist to track your progress:

| # | Category | Test | Status |
|---|----------|------|:------:|
| — | **Build** | | |
| 1.3 | Build | `cargo build --release` completes | ☐ |
| 1.4 | Build | `--version` shows version | ☐ |
| — | **Config** | | |
| 2.1 | Config | Testnet mode enabled | ☐ |
| 2.3 | Config | API keys configured | ☐ |
| — | **CLI** | | |
| V0 | CLI | System status displays | ☐ |
| V1 | CLI | Price check works | ☐ |
| V2 | CLI | Accounts created | ☐ |
| V3 | CLI | Holdings added | ☐ |
| V4 | CLI | Portfolio displays | ☐ |
| V5 | CLI | Transactions recorded | ☐ |
| V6 | CLI | Transfer works | ☐ |
| V7 | CLI | CSV import works | ☐ |
| — | **NLP** | | |
| 4.2 | NLP | Natural language queries | ☐ |
| 4.3 | NLP | Multi-turn conversation | ☐ |
| 4.4 | NLP | Out-of-scope handling | ☐ |
| — | **Cleanup** | | |
| 5 | Cleanup | Test data removed | ☐ |

---

## 🔧 Troubleshooting

### ❌ Build Fails

**Try cleaning and rebuilding:**
```bash
cargo clean
cargo build --release
```

**Still failing?** Check you have the latest Rust:
```bash
rustup update
```

---

### ❌ "Command not found"

Make sure you're running the binary with the correct path:
```bash
./target/release/cryptofolio --version
```

Not:
```bash
cryptofolio --version  # ❌ Won't work unless installed globally
```

---

### ❌ API Errors / Price Check Fails

1. **Check testnet mode is enabled:**
   ```bash
   ./target/release/cryptofolio config show
   ```
   Look for `use_testnet = true`

2. **Verify API keys are set:**
   Look for `api_key = "***" (set)`

3. **Check internet connection:**
   ```bash
   curl -s https://api.binance.com/api/v3/ping
   ```
   Should return `{}`

---

### ❌ AI/NLP Not Working

**For online mode (Claude):**
```bash
# Check if API key is set
echo $ANTHROPIC_API_KEY
```

**For offline mode (Ollama):**
```bash
# Check if Ollama is running
ollama list
```

If neither is configured, NLP features will fall back to rule-based parsing (less accurate but functional).

---

### ❌ "Database Locked" Error

Another process might be using the database:
```bash
# Find the process
lsof ~/.config/cryptofolio/database.sqlite

# Force remove if needed
rm -f ~/.config/cryptofolio/database.sqlite
```

---

## 📚 Quick Reference

| Action | Command |
|--------|---------|
| Check price | `./target/release/cryptofolio price BTC` |
| List accounts | `./target/release/cryptofolio account list` |
| List holdings | `./target/release/cryptofolio holdings list` |
| View portfolio | `./target/release/cryptofolio portfolio` |
| Start shell | `./target/release/cryptofolio shell` |
| Full help | `./target/release/cryptofolio --help` |

---

## 🤝 Need Help?

- 📖 Check the [main documentation](../README.md)
- 🐛 Report issues on [GitHub](https://github.com/yzumbado/cryptofolio/issues)
- 💬 See [CONVERSATIONAL_CLI.md](./CONVERSATIONAL_CLI.md) for AI feature details

---

*Document Version: 1.2 — Last Updated: 2026-02-16 (v0.2 - Secure Secret Handling)*
