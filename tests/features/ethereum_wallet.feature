Feature: Ethereum Wallet Tracking
  As a cryptocurrency investor
  I want to track my Ethereum wallets and ERC-20 tokens
  So that I can monitor my ETH and token holdings

  Background:
    Given I have a clean test database

  Scenario: Add Ethereum wallet with valid address
    When I run "cryptofolio wallet add 'My ETH Wallet' --blockchain ethereum --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should succeed
    And I should see "✓ Added wallet"
    And the wallet should be for blockchain "ethereum"

  Scenario: Reject invalid Ethereum address - wrong length
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain ethereum --address 0x123"
    Then the command should fail
    And I should see "Invalid Ethereum address"

  Scenario: Reject invalid Ethereum address - no 0x prefix
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain ethereum --address 742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should fail
    And I should see "Invalid Ethereum address"

  Scenario: Reject invalid Ethereum address - bad characters
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain ethereum --address 0xZZZZ35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should fail
    And I should see "Invalid Ethereum address"

  Scenario: Sync Ethereum wallet from blockchain
    Given I have added an Ethereum wallet "My ETH Wallet"
    And the Ethereum blockchain shows balance of 1.5 ETH
    When I run "cryptofolio wallet sync 'My ETH Wallet'"
    Then the command should succeed
    And I should see "✓ Synced ETH balance: 1.5000"

  Scenario: Detect ERC-20 tokens automatically
    Given I have added an Ethereum wallet "My ETH Wallet"
    And the Ethereum blockchain shows 3 ERC-20 tokens
    When I run "cryptofolio wallet sync 'My ETH Wallet'"
    Then the command should succeed
    And I should see "✓ Found 3 tokens"
    And I should see "USDT"
    And I should see "USDC"

  Scenario: Sync wallet with no tokens
    Given I have added an Ethereum wallet "My ETH Wallet"
    And the Ethereum blockchain shows balance of 0.5 ETH
    And the Ethereum blockchain shows 0 ERC-20 tokens
    When I run "cryptofolio wallet sync 'My ETH Wallet'"
    Then the command should succeed
    And I should see "✓ Synced ETH balance: 0.5000"
    And I should see "No tokens found"

  Scenario: Import transaction history with gas tracking
    Given I have added an Ethereum wallet "Active Wallet"
    And the blockchain shows 5 incoming and 3 outgoing transactions
    When I run "cryptofolio wallet sync 'Active Wallet' --import-history"
    Then the command should succeed
    And I should see "✓ Imported 8 transactions"
    And I should have 8 transactions in the database

  Scenario: List wallets shows Ethereum addresses
    Given I have added an Ethereum wallet "Wallet 1"
    And I have added an Ethereum wallet "Wallet 2"
    When I run "cryptofolio wallet list --blockchain ethereum"
    Then the command should succeed
    And I should see "Wallet 1"
    And I should see "Wallet 2"
    And I should see "ethereum"

  Scenario: Cannot add duplicate Ethereum address
    Given I have added an Ethereum wallet "Wallet 1"
    When I run "cryptofolio wallet add 'Wallet 2' --blockchain ethereum --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should fail
    And I should see "already exists"

  Scenario: Sync wallet shows ERC-20 token balances
    Given I have added an Ethereum wallet "Token Holder"
    And the wallet has 1000 USDT ERC-20 tokens
    And the wallet has 500 USDC ERC-20 tokens
    When I run "cryptofolio wallet sync 'Token Holder'"
    Then the command should succeed
    And I should see "USDT: 1000.00"
    And I should see "USDC: 500.00"

  Scenario: Track testnet Ethereum wallet
    When I run "cryptofolio wallet add 'Sepolia Testnet' --blockchain ethereum --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0 --network testnet"
    Then the command should succeed
    And I should see "✓ Added wallet"
    And I should see "[TESTNET]"
