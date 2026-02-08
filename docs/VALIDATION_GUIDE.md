# Cryptofolio Validation Guide

Manual validation protocol for verifying cryptofolio functionality after builds or changes.

**Version:** 1.0
**Last Updated:** 2024-03-15

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Build Instructions](#build-instructions)
3. [Validation Scenarios](#validation-scenarios)
   - [CLI Validation](#cli-validation)
   - [NLP Evaluation](#nlp-evaluation)
   - [Data Import Validation](#data-import-validation)
4. [Cleanup](#cleanup)
5. [Validation Checklist](#validation-checklist)

---

## Prerequisites

### Required
- Rust toolchain (rustc 1.70+)
- Internet connection (for Binance API)

### Optional (for AI features)
- **Claude API Key**: Set `ANTHROPIC_API_KEY` environment variable
- **Ollama**: Install and run locally for offline AI mode
  ```bash
  # macOS
  brew install ollama
  ollama serve &
  ollama pull llama3.2:3b
  ```

### Testnet Setup (Recommended)
1. Create a Binance Testnet account: https://testnet.binance.vision/
2. Generate API keys from the testnet
3. Configure keys (done during validation below)

---

## Build Instructions

### 1. Clone and Build (Release Mode)

```bash
# Clone repository (skip if already cloned)
git clone https://github.com/yzumbado/cryptofolio.git
cd cryptofolio

# Build in release mode
cargo build --release

# Verify build
./target/release/cryptofolio --version
```

**Expected Output:**
```
cryptofolio 0.1.0
```

### 2. Verify Help

```bash
./target/release/cryptofolio --help
```

**Expected Output:** Should display available commands including `price`, `portfolio`, `account`, `holdings`, `tx`, `sync`, `import`, `config`, `shell`.

---

## Validation Scenarios

### CLI Validation

#### V1: Configuration Setup

```bash
# Enable testnet mode
./target/release/cryptofolio config use-testnet

# Set Binance testnet credentials (use your own keys)
./target/release/cryptofolio config set binance.api_key YOUR_TESTNET_API_KEY
./target/release/cryptofolio config set binance.api_secret YOUR_TESTNET_API_SECRET

# Verify configuration
./target/release/cryptofolio config show
```

**Expected:** Config displays with `use_testnet = true` and API keys set (masked).

---

#### V2: Price Check

```bash
./target/release/cryptofolio price BTC ETH
```

**Expected Output:** Table with current prices for BTC and ETH in USD.

```
SYMBOL    PRICE
BTC       $XX,XXX.XX
ETH       $X,XXX.XX
```

---

#### V3: Account Management

```bash
# List default categories
./target/release/cryptofolio category list

# Create accounts
./target/release/cryptofolio account add "Binance Test" --type exchange --category trading
./target/release/cryptofolio account add "Ledger Nano" --type hardware_wallet --category cold-storage
./target/release/cryptofolio account add "MetaMask" --type software_wallet --category hot-wallets

# Verify accounts
./target/release/cryptofolio account list
```

**Expected:** Three accounts created and listed with correct types and categories.

---

#### V4: Holdings Management

```bash
# Add holdings to accounts
./target/release/cryptofolio holdings add BTC 0.5 --account "Ledger Nano" --cost 45000
./target/release/cryptofolio holdings add ETH 2.0 --account "MetaMask" --cost 2800
./target/release/cryptofolio holdings add BTC 0.1 --account "Binance Test" --cost 62000

# List all holdings
./target/release/cryptofolio holdings list

# List holdings for specific account
./target/release/cryptofolio holdings list --account "Ledger Nano"
```

**Expected:** Holdings displayed with quantities and cost basis.

---

#### V5: Portfolio View

```bash
# View full portfolio
./target/release/cryptofolio portfolio

# View by account
./target/release/cryptofolio portfolio --by-account

# View by category
./target/release/cryptofolio portfolio --by-category
```

**Expected:** Portfolio summary with:
- Total value in USD
- P&L (profit/loss) percentage
- Holdings breakdown

---

#### V6: Transaction Recording

```bash
# Record a buy transaction
./target/release/cryptofolio tx buy SOL 10 --account "Binance Test" --price 150

# Record a sell transaction
./target/release/cryptofolio tx sell ETH 0.5 --account "MetaMask" --price 3200

# List transactions
./target/release/cryptofolio tx list
```

**Expected:** Transactions recorded and listed with timestamps.

---

#### V7: Transfer Between Accounts

```bash
# Move holdings between accounts
./target/release/cryptofolio holdings move BTC 0.05 --from "Binance Test" --to "Ledger Nano" --yes

# Verify the transfer
./target/release/cryptofolio holdings list
```

**Expected:**
- Binance Test: 0.05 BTC (reduced from 0.1)
- Ledger Nano: 0.55 BTC (increased from 0.5)

---

### NLP Evaluation

Start the interactive shell for natural language testing:

```bash
./target/release/cryptofolio shell
```

#### N1: Price Queries

| Input | Expected Intent | Expected Action |
|-------|-----------------|-----------------|
| `What's the price of Bitcoin?` | `price.check` | Displays BTC price |
| `How much is ETH right now?` | `price.check` | Displays ETH price |
| `btc price` | `price.check` | Displays BTC price |

---

#### N2: Portfolio Queries

| Input | Expected Intent | Expected Action |
|-------|-----------------|-----------------|
| `Show my portfolio` | `portfolio.view` | Displays portfolio |
| `How am I doing?` | `portfolio.view` | Displays portfolio |
| `What do I have?` | `holdings.list` | Lists holdings |

---

#### N3: Transaction Recording (Multi-turn)

**Test Conversation:**
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

**Expected:** Multi-turn conversation collecting missing info, then confirmation prompt.

---

#### N4: Sync Command

| Input | Expected Intent | Expected Action |
|-------|-----------------|-----------------|
| `Sync my exchanges` | `sync` | Triggers sync |
| `Refresh everything` | `sync` | Triggers sync |

---

#### N5: Out of Scope

| Input | Expected Response |
|-------|-------------------|
| `What's the weather?` | "I can only help with cryptocurrency portfolio management" |
| `Tell me a joke` | Out of scope message |

Exit shell:
```
you> exit
```

---

### Data Import Validation

#### I1: Import Sample Transactions

```bash
# Import the sample CSV
./target/release/cryptofolio import tests/fixtures/sample_transactions.csv --account "Ledger Nano"

# Verify transactions imported
./target/release/cryptofolio tx list --limit 20

# Verify holdings updated
./target/release/cryptofolio holdings list --account "Ledger Nano"
```

**Expected:**
- 8 transactions imported
- Holdings reflect the net effect (buys - sells)

---

## Cleanup

### Option A: Delete Database (Full Reset)

```bash
# Find and remove the database
rm -f ~/.config/cryptofolio/database.sqlite

# Verify cleanup
./target/release/cryptofolio holdings list
```

**Expected:** Empty holdings list (fresh database created).

---

### Option B: Remove Test Data (Keep Config)

```bash
# Remove specific accounts (will cascade delete holdings)
./target/release/cryptofolio account remove "Binance Test" --yes
./target/release/cryptofolio account remove "Ledger Nano" --yes
./target/release/cryptofolio account remove "MetaMask" --yes

# Verify cleanup
./target/release/cryptofolio account list
./target/release/cryptofolio holdings list
```

**Expected:** No accounts or holdings remaining.

---

### Option C: Reset to Defaults (Full Cleanup)

```bash
# Remove entire config directory
rm -rf ~/.config/cryptofolio/

# Verify (will create fresh config and database)
./target/release/cryptofolio config show
```

**Expected:** Default configuration displayed, empty database.

---

## Validation Checklist

Use this checklist to track validation progress:

| # | Category | Test | Status |
|---|----------|------|--------|
| V1 | CLI | Configuration setup | ☐ |
| V2 | CLI | Price check | ☐ |
| V3 | CLI | Account management | ☐ |
| V4 | CLI | Holdings management | ☐ |
| V5 | CLI | Portfolio view | ☐ |
| V6 | CLI | Transaction recording | ☐ |
| V7 | CLI | Transfer between accounts | ☐ |
| N1 | NLP | Price queries | ☐ |
| N2 | NLP | Portfolio queries | ☐ |
| N3 | NLP | Transaction recording (multi-turn) | ☐ |
| N4 | NLP | Sync command | ☐ |
| N5 | NLP | Out of scope handling | ☐ |
| I1 | Import | CSV import | ☐ |
| -- | Cleanup | Database/config cleanup | ☐ |

---

## Troubleshooting

### Build Errors

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Database Locked

```bash
# Check for running processes
lsof ~/.config/cryptofolio/database.sqlite

# Force remove if needed
rm -f ~/.config/cryptofolio/database.sqlite
```

### API Errors

- Verify testnet mode is enabled: `cryptofolio config show`
- Check API keys are correct
- Ensure internet connectivity
- Binance testnet may have rate limits

### AI Not Working

- Check if `ANTHROPIC_API_KEY` is set: `echo $ANTHROPIC_API_KEY`
- For offline mode, verify Ollama is running: `ollama list`
- Check AI mode in config: should show "Hybrid" in shell welcome

---

## Notes

- All tests use **testnet** mode to avoid real transactions
- NLP tests require either Claude API key or Ollama running
- The sample CSV in `tests/fixtures/` can be modified for additional test cases
- Run cleanup after validation to reset for next test cycle

---

*Document Version: 1.0*
