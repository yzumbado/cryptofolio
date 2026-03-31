# BDD Implementation Plan for v0.5.0

**Approach:** Behavior-Driven Development using cucumber-rs
**Goal:** Define wallet integration behavior through executable specifications

---

## 🎯 Why BDD for Wallet Integration?

### Benefits
1. **Clear Requirements** - Feature files document exact behavior
2. **Stakeholder Validation** - You can review and approve scenarios
3. **Living Documentation** - Tests describe system behavior
4. **Outside-In Development** - Start with user perspective
5. **Regression Safety** - Scenarios become automated tests

### BDD Cycle
```
1. Define Feature (Gherkin)
   ↓
2. Write Failing Test
   ↓
3. Implement Code
   ↓
4. Test Passes (Green)
   ↓
5. Refactor
   ↓
6. Repeat
```

---

## 🏗️ Project Structure

```
tests/
  features/                    # Feature files (Gherkin)
    wallet_management.feature
    bitcoin_sync.feature
    ethereum_sync.feature
    erc20_tokens.feature
    portfolio_audit.feature
    node_management.feature
    multi_chain.feature

  step_definitions/            # Step implementations
    wallet_steps.rs
    bitcoin_steps.rs
    ethereum_steps.rs
    audit_steps.rs
    node_steps.rs
    common_steps.rs

  support/                     # Test helpers
    world.rs                   # Shared state
    fixtures.rs                # Test data
    blockchain_mock.rs         # Mock blockchain responses

Cargo.toml                     # Add cucumber dependency
```

---

## 📦 Setup: Add cucumber-rs

### Cargo.toml
```toml
[dev-dependencies]
cucumber = "0.20"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

[[test]]
name = "bdd"
harness = false  # Don't use default test harness
```

### tests/bdd.rs (Test Runner)
```rust
use cucumber::{given, when, then, World};

// Import step definitions
mod step_definitions;
mod support;

use support::world::CryptofolioWorld;

#[tokio::main]
async fn main() {
    CryptofolioWorld::cucumber()
        .run("tests/features")
        .await;
}
```

---

## 🌍 World Setup (Shared State)

### tests/support/world.rs
```rust
use cucumber::{World, WorldInit};
use sqlx::SqlitePool;
use std::collections::HashMap;

#[derive(Debug, WorldInit)]
pub struct CryptofolioWorld {
    pub pool: Option<SqlitePool>,
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub wallets: HashMap<String, WalletData>,
    pub node_configs: HashMap<String, NodeConfig>,
    pub last_audit_result: Option<AuditResult>,
}

#[derive(Debug, Clone)]
pub struct WalletData {
    pub blockchain: String,
    pub address: String,
    pub balance: Option<String>,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub blockchain: String,
    pub node_type: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct AuditResult {
    pub total_value: String,
    pub exchange_percentage: f64,
    pub wallet_percentage: f64,
    pub discrepancies: Vec<String>,
}

impl Default for CryptofolioWorld {
    fn default() -> Self {
        Self {
            pool: None,
            output: None,
            exit_code: None,
            wallets: HashMap::new(),
            node_configs: HashMap::new(),
            last_audit_result: None,
        }
    }
}

impl CryptofolioWorld {
    pub async fn setup_test_db(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        self.pool = Some(pool);
        Ok(())
    }

    pub fn get_pool(&self) -> &SqlitePool {
        self.pool.as_ref().expect("Database not initialized")
    }
}
```

---

## 📝 Feature Files (Gherkin)

### tests/features/wallet_management.feature
```gherkin
Feature: Wallet Management
  As a cryptocurrency holder
  I want to track my wallets across different blockchains
  So that I can see all my holdings in one place

  Background:
    Given a fresh cryptofolio installation
    And the database is initialized

  Scenario: Add a Bitcoin wallet with single address
    When I run "cryptofolio wallet add 'My Ledger' --blockchain bitcoin --address bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    Then the command should succeed
    And I should see "✓ Added Bitcoin wallet 'My Ledger'"
    And the wallet "My Ledger" should exist in the database
    And the wallet should have blockchain "bitcoin"

  Scenario: Add a Bitcoin wallet with xpub (HD wallet)
    Given I have a valid Bitcoin xpub "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvphXNM4Xo"
    When I run "cryptofolio wallet add 'Sparrow' --blockchain bitcoin --xpub zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvphXNM4Xo"
    Then the command should succeed
    And the wallet "Sparrow" should support HD derivation
    And I should be able to derive addresses from the xpub

  Scenario: Add an Ethereum wallet
    When I run "cryptofolio wallet add 'MetaMask' --blockchain ethereum --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"
    Then the command should succeed
    And the wallet "MetaMask" should exist with blockchain "ethereum"

  Scenario: List all wallets
    Given I have added wallet "Ledger" for blockchain "bitcoin"
    And I have added wallet "MetaMask" for blockchain "ethereum"
    When I run "cryptofolio wallet list"
    Then I should see a table containing:
      | Name     | Blockchain | Address     | Last Synced |
      | Ledger   | Bitcoin    | bc1q...     | Never       |
      | MetaMask | Ethereum   | 0x742d...   | Never       |

  Scenario: Prevent duplicate wallet addresses
    Given I have added wallet "Ledger" with address "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    When I try to add another wallet with the same address
    Then the command should fail
    And I should see "Error: Address already tracked"
```

### tests/features/bitcoin_sync.feature
```gherkin
Feature: Bitcoin Wallet Synchronization
  As a Bitcoin holder
  I want to sync my wallet from the blockchain
  So that my balance and transaction history are up-to-date

  Background:
    Given a fresh cryptofolio installation
    And I have added a Bitcoin wallet "My Wallet" with address "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"

  Scenario: Sync Bitcoin wallet from public API
    Given the Bitcoin blockchain has the following data for my address:
      | Balance | Transactions |
      | 0.5 BTC | 23           |
    And I am using blockchain.info as the provider
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then the command should succeed
    And I should see "✓ Synced BTC balance: 0.5000"
    And I should see "✓ Imported 23 transactions"
    And the wallet balance should be "0.5" BTC
    And there should be 23 transactions in the database

  Scenario: Sync Bitcoin wallet from local node
    Given I have a Bitcoin Core node running at "localhost:8332"
    And the node is fully synced
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then it should use the local node
    And the sync should be faster than using public API
    And I should see "✓ Synced from local node"

  Scenario: Sync HD wallet (xpub) discovers multiple addresses
    Given I have added wallet "Sparrow" with xpub
    And the xpub has funds in 5 derived addresses
    When I run "cryptofolio wallet sync 'Sparrow' --discover"
    Then it should derive addresses up to the gap limit
    And it should find balances in 5 addresses
    And the total balance should be the sum of all addresses

  Scenario: Incremental sync only fetches new transactions
    Given I have synced "My Wallet" previously
    And the last sync was at block height 835000
    And there are 3 new transactions since block 835000
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then it should only fetch transactions after block 835000
    And it should import 3 new transactions
    And the previous 23 transactions should remain unchanged

  Scenario: Fallback to public API when local node is unavailable
    Given I have configured a local Bitcoin node
    But the node is not running
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then I should see a warning "⚠️ Local node unavailable, using blockchain.info"
    And the sync should succeed using the fallback provider
```

### tests/features/ethereum_sync.feature
```gherkin
Feature: Ethereum Wallet Synchronization
  As an Ethereum holder
  I want to sync my wallet including ERC-20 tokens
  So that I can track my complete Ethereum portfolio

  Background:
    Given a fresh cryptofolio installation
    And I have added an Ethereum wallet "MetaMask" with address "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"

  Scenario: Sync Ethereum wallet gets ETH balance
    Given the Ethereum blockchain shows my address has 2.5 ETH
    When I run "cryptofolio wallet sync 'MetaMask'"
    Then the command should succeed
    And I should see "✓ ETH: 2.5000"
    And the wallet balance should be "2.5" ETH

  Scenario: Automatically detect ERC-20 tokens
    Given my Ethereum address holds the following tokens:
      | Token | Balance    |
      | USDT  | 5,000.00   |
      | rETH  | 15.234     |
      | RPL   | 50.00      |
    When I run "cryptofolio wallet sync 'MetaMask' --detect-tokens"
    Then I should see "✓ USDT: 5,000.00 (ERC-20)"
    And I should see "✓ rETH: 15.234 (ERC-20)"
    And I should see "✓ RPL: 50.00 (ERC-20)"
    And all three tokens should be in the holdings table

  Scenario: Track specific ERC-20 tokens
    Given I want to track USDT (0xdac17f958d2ee523a2206206994597c13d831ec7)
    When I run "cryptofolio wallet add-token 'MetaMask' --token USDT --contract 0xdac17f958d2ee523a2206206994597c13d831ec7"
    And I run "cryptofolio wallet sync 'MetaMask'"
    Then the USDT balance should be retrieved
    And other tokens should not be detected

  Scenario: Import transaction history including gas fees
    Given my address has 10 transactions
    And each transaction has a gas fee
    When I run "cryptofolio wallet sync 'MetaMask' --history"
    Then it should import all 10 transactions
    And each transaction should have a gas fee recorded
    And the fee asset should be "ETH"
```

### tests/features/portfolio_audit.feature
```gherkin
Feature: Portfolio Audit
  As a cryptocurrency investor
  I want to audit my complete portfolio
  So that I can verify all my funds are accounted for

  Background:
    Given a fresh cryptofolio installation
    And I have the following exchange holdings:
      | Exchange | Asset | Amount    |
      | Binance  | BTC   | 0.1904    |
      | Binance  | ETH   | 1.8322    |
      | Binance  | USDT  | 36,383.00 |
    And I have the following wallet holdings:
      | Wallet   | Blockchain | Asset | Amount    |
      | Ledger   | Bitcoin    | BTC   | 0.5000    |
      | MetaMask | Ethereum   | ETH   | 2.5000    |
      | MetaMask | Ethereum   | USDT  | 5,000.00  |
      | MetaMask | Ethereum   | rETH  | 15.234    |

  Scenario: Run basic portfolio audit
    When I run "cryptofolio audit"
    Then I should see an audit summary
    And the summary should show:
      | Metric              | Value   |
      | Total BTC           | 0.6904  |
      | Total ETH           | 4.3322  |
      | Total USDT          | 41,383  |
      | Coverage            | 100%    |

  Scenario: Detailed audit shows account breakdown
    When I run "cryptofolio audit --detailed"
    Then I should see a detailed breakdown:
      """
      Asset    Exchange    Wallets         Total      Status
      ──────────────────────────────────────────────────────
      BTC      0.1904      0.5000          0.6904     ✓ OK
               └─Binance   └─Ledger

      ETH      1.8322      2.5000          4.3322     ✓ OK
               └─Binance   └─MetaMask

      rETH     0.0000      15.2340         15.2340    ⚠️ Only in wallets
                           └─MetaMask (ERC-20)
      """

  Scenario: Audit highlights discrepancies
    Given I have 0.5 BTC in my Ledger wallet
    But the transaction history only shows 0.3 BTC received
    When I run "cryptofolio audit --detailed"
    Then I should see a warning for BTC:
      """
      ⚠️ Discrepancy detected:
         Wallet balance: 0.5 BTC
         Transaction history: 0.3 BTC
         Unaccounted: 0.2 BTC
      """

  Scenario: Audit calculates portfolio distribution
    When I run "cryptofolio audit"
    Then I should see distribution percentages:
      """
      💰 Total Portfolio Value: $152,483 USD
         Exchange:  $48,234 (31.6%)
         Wallets:   $104,249 (68.4%)
      """

  Scenario: Audit provides recommendations
    Given 90% of my portfolio is on exchanges
    When I run "cryptofolio audit"
    Then I should see a recommendation:
      """
      ⚠️ Recommendations:
      • Consider moving more assets to cold storage (currently 10%)
      • Your exchange holdings exceed recommended threshold (90%)
      """
```

### tests/features/node_management.feature
```gherkin
Feature: Blockchain Node Management
  As a privacy-conscious user
  I want to manage local blockchain nodes
  So that I can sync wallets without exposing addresses to third parties

  Scenario: Check if system meets node requirements
    When I run "cryptofolio node check-requirements"
    Then I should see system information:
      | Requirement      | Status        |
      | macOS Version    | ✓ 14.0+       |
      | Free Disk Space  | ✓ 50GB        |
      | Homebrew         | ✓ Installed   |

  Scenario: Install Bitcoin Core in pruned mode
    When I run "cryptofolio node install bitcoin --pruned"
    Then it should install Bitcoin Core via Homebrew
    And it should create a bitcoin.conf file with pruning enabled
    And it should generate a secure RPC password
    And the RPC password should be stored in macOS Keychain
    And it should start Bitcoin Core as a launchd service
    And I should see "✓ Bitcoin Core installed successfully"

  Scenario: Check node status while syncing
    Given I have installed Bitcoin Core
    And the node is 50% synced
    When I run "cryptofolio node status bitcoin"
    Then I should see:
      """
      Bitcoin
        Status:     ✓ Running (local node)
        Height:     425,123 / 835,421 (50.9%)
        Sync ETA:   ~18 hours
        Disk:       5.2GB / 10GB (pruned)
      """

  Scenario: Automatic fallback to public API during sync
    Given I have a Bitcoin node that is 20% synced
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then I should see "⚠️ Local node syncing (20%), using blockchain.info"
    And the sync should succeed using the public API

  Scenario: Use local node after sync completes
    Given I have a Bitcoin node that is 100% synced
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then it should use the local node
    And I should see "✓ Synced from local node"
    And no external API calls should be made

  Scenario: Configure node with custom RPC settings
    When I run:
      """
      cryptofolio node set bitcoin \
        --type local \
        --rpc-url http://localhost:8332 \
        --rpc-user custom_user \
        --rpc-password custom_password
      """
    Then the node configuration should be saved
    And the RPC password should be stored in Keychain

  Scenario: Stop and restart node service
    Given I have a running Bitcoin node
    When I run "cryptofolio node stop bitcoin"
    Then the Bitcoin Core process should stop
    When I run "cryptofolio node start bitcoin"
    Then the Bitcoin Core process should start
    And it should resume syncing from the last block
```

### tests/features/multi_chain.feature
```gherkin
Feature: Multi-Chain Portfolio Tracking
  As a diversified crypto investor
  I want to track holdings across multiple blockchains
  So that I can see my complete portfolio in one place

  Background:
    Given a fresh cryptofolio installation

  Scenario: Track Bitcoin, Ethereum, Solana, and Cardano
    Given I add wallets for each blockchain:
      | Wallet     | Blockchain | Address           |
      | Ledger     | Bitcoin    | bc1qxy2kg...      |
      | MetaMask   | Ethereum   | 0x742d35C...      |
      | Solflare   | Solana     | ABC123xyz...      |
      | Eternl     | Cardano    | addr1qxy2kg...    |
    When I run "cryptofolio wallet sync --all"
    Then all 4 wallets should sync successfully
    And I should see balances for all chains

  Scenario: Portfolio view aggregates all chains
    Given I have holdings on all 4 blockchains
    When I run "cryptofolio portfolio"
    Then I should see aggregated portfolio value
    And holdings should be grouped by blockchain:
      """
      Bitcoin (Chain: BTC)
        Ledger: 0.5000 BTC

      Ethereum (Chain: ETH)
        MetaMask: 2.5000 ETH
        MetaMask: 5,000.00 USDT (ERC-20)
        MetaMask: 15.234 rETH (ERC-20)

      Solana (Chain: SOL)
        Solflare: 100.00 SOL

      Cardano (Chain: ADA)
        Eternl: 5,000.00 ADA
      """

  Scenario: Audit works across all chains
    Given I have holdings across all blockchains
    And I have exchange holdings
    When I run "cryptofolio audit"
    Then the audit should include all chains
    And it should show coverage per blockchain

  Scenario: Sync respects per-chain node configuration
    Given I have Bitcoin Core running locally
    And I'm using Alchemy API for Ethereum
    And I'm using public RPC for Solana
    When I run "cryptofolio wallet sync --all"
    Then Bitcoin should sync from local node
    And Ethereum should sync from Alchemy
    And Solana should sync from public RPC
```

---

## 🔨 Step Definitions

### tests/step_definitions/wallet_steps.rs
```rust
use cucumber::{given, when, then};
use crate::support::world::CryptofolioWorld;
use std::process::Command;

#[given("a fresh cryptofolio installation")]
async fn fresh_installation(world: &mut CryptofolioWorld) {
    world.setup_test_db().await.unwrap();
}

#[given("the database is initialized")]
async fn database_initialized(world: &mut CryptofolioWorld) {
    // Migrations already run in setup_test_db
    assert!(world.pool.is_some());
}

#[when(regex = r#"^I run "(.*)"$"#)]
async fn run_command(world: &mut CryptofolioWorld, command: String) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let output = Command::new(parts[0])
        .args(&parts[1..])
        .env("DATABASE_URL", "sqlite::memory:")
        .output()
        .expect("Failed to execute command");

    world.output = Some(String::from_utf8_lossy(&output.stdout).to_string());
    world.exit_code = output.status.code();
}

#[then("the command should succeed")]
fn command_succeeds(world: &mut CryptofolioWorld) {
    assert_eq!(world.exit_code, Some(0), "Command failed with non-zero exit code");
}

#[then(regex = r#"^I should see "(.*)"$"#)]
fn should_see_output(world: &mut CryptofolioWorld, expected: String) {
    let output = world.output.as_ref().expect("No command output");
    assert!(
        output.contains(&expected),
        "Expected to see '{}' in output, but got: {}",
        expected,
        output
    );
}

#[then(regex = r#"^the wallet "(.*)" should exist in the database$"#)]
async fn wallet_exists(world: &mut CryptofolioWorld, wallet_name: String) {
    let pool = world.get_pool();
    let result = sqlx::query!("SELECT id FROM wallet_addresses WHERE label = ?", wallet_name)
        .fetch_one(pool)
        .await;

    assert!(result.is_ok(), "Wallet '{}' not found in database", wallet_name);
}
```

### tests/step_definitions/bitcoin_steps.rs
```rust
use cucumber::{given, when, then};
use crate::support::world::CryptofolioWorld;
use crate::support::blockchain_mock::MockBitcoinBlockchain;

#[given(regex = r#"^the Bitcoin blockchain has the following data for my address:$"#)]
async fn mock_bitcoin_data(world: &mut CryptofolioWorld, table: cucumber::Table) {
    // Parse table and set up mock responses
    for row in table.rows {
        let balance = row[0];
        let tx_count = row[1];

        let mock = MockBitcoinBlockchain::new()
            .with_balance(balance)
            .with_transaction_count(tx_count.parse().unwrap());

        // Store mock in world for later use
        world.bitcoin_mock = Some(mock);
    }
}

#[given(regex = r#"^I am using (.*) as the provider$"#)]
fn set_provider(world: &mut CryptofolioWorld, provider: String) {
    world.provider = Some(provider);
}

#[then(regex = r#"^the wallet balance should be "(.*)" (.*)$"#)]
async fn check_balance(world: &mut CryptofolioWorld, amount: String, asset: String) {
    let pool = world.get_pool();
    let result = sqlx::query!(
        "SELECT quantity FROM holdings WHERE asset = ?",
        asset
    )
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(result.quantity, amount);
}
```

---

## 🚀 Running BDD Tests

### Run All Features
```bash
cargo test --test bdd

# Output:
# Feature: Wallet Management
#   Scenario: Add a Bitcoin wallet with single address ✓
#   Scenario: Add a Bitcoin wallet with xpub ✓
#   Scenario: Add an Ethereum wallet ✓
#   ...
#
# 25 scenarios (25 passed)
# 127 steps (127 passed)
```

### Run Specific Feature
```bash
cargo test --test bdd -- tests/features/bitcoin_sync.feature

# Or with cucumber args:
cargo test --test bdd -- --tags @bitcoin
```

### Run with Tags
```gherkin
@bitcoin @critical
Scenario: Sync Bitcoin wallet from local node
  ...
```

```bash
# Run only @critical scenarios
cargo test --test bdd -- --tags @critical

# Run all except @slow scenarios
cargo test --test bdd -- --tags "not @slow"
```

---

## 📊 Development Workflow

### 1. **Write Feature First** (Outside-In)
```gherkin
# tests/features/bitcoin_sync.feature
Scenario: Sync Bitcoin wallet from public API
  Given I have added a Bitcoin wallet
  When I sync the wallet
  Then the balance should be updated
```

### 2. **Run Test (Should Fail)**
```bash
cargo test --test bdd

# Output:
# Scenario: Sync Bitcoin wallet from public API ✗
#   Step "I have added a Bitcoin wallet" - undefined
```

### 3. **Implement Step Definitions**
```rust
#[given("I have added a Bitcoin wallet")]
async fn add_bitcoin_wallet(world: &mut CryptofolioWorld) {
    // Implementation
}
```

### 4. **Implement Feature Code**
```rust
// src/blockchain/bitcoin/client.rs
impl BitcoinClient {
    pub async fn get_balance(&self, address: &str) -> Result<Decimal> {
        // Implementation
    }
}
```

### 5. **Test Passes (Green)**
```bash
cargo test --test bdd

# Output:
# Scenario: Sync Bitcoin wallet from public API ✓
```

### 6. **Refactor & Repeat**

---

## 🎯 Integration with Existing Tests

### Keep Both Unit and BDD Tests
```rust
// Unit tests (fast, isolated)
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_bitcoin_address() {
        // Fast, focused test
    }
}

// BDD tests (slower, end-to-end)
// tests/features/bitcoin_sync.feature
Scenario: Parse and validate Bitcoin address
  # User-facing behavior test
```

### Test Pyramid
```
        /\
       /  \  E2E BDD Tests (Slow, Complete)
      /    \
     /------\ Integration Tests (Medium)
    /--------\
   /----------\ Unit Tests (Fast, Focused)
  /------------\
```

---

## 📝 Writing Good Scenarios

### Good Scenario (User Perspective)
```gherkin
Scenario: Sync Bitcoin wallet
  Given I have a Bitcoin wallet with 0.5 BTC
  When I sync the wallet
  Then my portfolio should show 0.5 BTC
```

### Bad Scenario (Implementation Details)
```gherkin
Scenario: Call Bitcoin RPC and parse response
  Given a BitcoinClient instance
  When I call get_balance with address "bc1q..."
  Then it should return Decimal(0.5)
```

### Tips
- ✅ **Focus on behavior**, not implementation
- ✅ **Use business language**, not technical terms
- ✅ **One scenario = one behavior**
- ✅ **Make scenarios readable by stakeholders**
- ❌ Don't test internal APIs directly

---

## 📅 BDD Implementation Timeline

### Week 1: Setup + Bitcoin Features
- [ ] Day 1-2: Set up cucumber-rs, world, mocks
- [ ] Day 3-4: Write Bitcoin sync features
- [ ] Day 5: Implement step definitions + code

### Week 2: Ethereum + Audit Features
- [ ] Day 1-2: Ethereum sync features
- [ ] Day 3: ERC-20 token features
- [ ] Day 4-5: Portfolio audit features

### Week 3: Multi-Chain + Node Management
- [ ] Day 1-2: Solana + Cardano features
- [ ] Day 3-4: Node management features
- [ ] Day 5: Polish and documentation

---

## 🎉 Benefits Summary

### For Development
- ✅ Clear requirements before coding
- ✅ Confidence in refactoring
- ✅ Fewer bugs in production
- ✅ Better code design (testable)

### For Stakeholders (You)
- ✅ Can review scenarios in plain English
- ✅ Can suggest changes before coding starts
- ✅ Features match expectations
- ✅ Living documentation stays up-to-date

### For Maintenance
- ✅ Tests describe "why", not just "how"
- ✅ Easy to add new scenarios
- ✅ Regression safety
- ✅ Onboarding documentation

---

## 🚀 Next Steps

1. **Review sample features** - Any scenarios to add/change?
2. **Set up cucumber-rs** - Install dependencies
3. **Write first feature** - Start with wallet management
4. **Implement step definitions** - Make tests pass
5. **Iterate** - Add scenarios as we go

Ready to start with BDD? Which feature should we tackle first?
