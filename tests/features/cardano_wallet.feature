Feature: Cardano Wallet Tracking
  As a user
  I want to track my Cardano (ADA) wallets
  So that I can monitor my ADA balance and native tokens

  Scenario: Add Cardano wallet with valid address
    Given I have a clean test database
    When I run "cryptofolio wallet add 'My ADA Wallet' --blockchain cardano --address addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa"
    Then the command should succeed
    And I should see "✓ Added wallet"
    And the wallet should be for blockchain "cardano"

  Scenario: Reject invalid Cardano address - wrong prefix
    Given I have a clean test database
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain cardano --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should fail
    And I should see "Invalid Cardano address"

  Scenario: Reject invalid Cardano address - wrong length
    Given I have a clean test database
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain cardano --address addr1qxy2"
    Then the command should fail
    And I should see "Invalid Cardano address"

  Scenario: Reject invalid Cardano address - bad checksum
    Given I have a clean test database
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain cardano --address addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qzzz"
    Then the command should fail
    And I should see "Invalid Cardano address"

  Scenario: Sync Cardano wallet from blockchain
    Given I have a clean test database
    Given I have added a Cardano wallet "My ADA Wallet"
    And the Cardano blockchain shows balance of 1000.5 ADA
    When I run "cryptofolio wallet sync 'My ADA Wallet'"
    Then the command should succeed
    And I should see "✓ Synced ADA balance: 1000.5"

  Scenario: Detect native tokens automatically
    Given I have a clean test database
    Given I have added a Cardano wallet "My ADA Wallet"
    And the Cardano blockchain shows 3 native tokens
    When I run "cryptofolio wallet sync 'My ADA Wallet'"
    Then the command should succeed
    And I should see "✓ Found 3 tokens"
    And I should see "HOSKY"

  Scenario: Sync wallet with no tokens
    Given I have a clean test database
    Given I have added a Cardano wallet "My ADA Wallet"
    And the Cardano blockchain shows balance of 100.0 ADA
    And the Cardano blockchain shows 0 native tokens
    When I run "cryptofolio wallet sync 'My ADA Wallet'"
    Then the command should succeed
    And I should see "✓ Synced ADA balance: 100.0"
    And I should see "No tokens found"

  Scenario: Import transaction history
    Given I have a clean test database
    Given I have added a Cardano wallet "Active Wallet"
    And the blockchain shows 5 incoming and 3 outgoing Cardano transactions
    When I run "cryptofolio wallet sync 'Active Wallet' --import-history"
    Then the command should succeed
    And I should see "✓ Imported 8 transactions"

  Scenario: List wallets shows Cardano addresses
    Given I have a clean test database
    Given I have added a Cardano wallet "Wallet 1"
    And I have added a Cardano wallet "Wallet 2"
    When I run "cryptofolio wallet list --blockchain cardano"
    Then the command should succeed
    And I should see "Wallet 1"
    And I should see "Wallet 2"
    And I should see "cardano"

  Scenario: Cannot add duplicate Cardano address
    Given I have a clean test database
    Given I have added a Cardano wallet "Wallet 1"
    When I run "cryptofolio wallet add 'Wallet 2' --blockchain cardano --address addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa"
    Then the command should fail
    And I should see "already exists"

  Scenario: Sync wallet shows native token balances
    Given I have a clean test database
    Given I have added a Cardano wallet "Token Holder"
    And the wallet has 1000000 HOSKY native tokens
    And the wallet has 50 MIN native tokens
    When I run "cryptofolio wallet sync 'Token Holder'"
    Then the command should succeed
    And I should see "HOSKY: 1000000.00"
    And I should see "MIN: 50.00"

  Scenario: Track testnet Cardano wallet
    Given I have a clean test database
    When I run "cryptofolio wallet add 'Preprod Testnet' --blockchain cardano --address addr_test1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqgp3r9e --network testnet"
    Then the command should succeed
    And I should see "✓ Added wallet"
    And I should see "[TESTNET]"

  Scenario: Sync shows stake pool information
    Given I have a clean test database
    Given I have added a Cardano wallet "Staked Wallet"
    And the wallet is delegated to stake pool "BLOOM"
    When I run "cryptofolio wallet sync 'Staked Wallet'"
    Then the command should succeed
    And I should see "✓ Synced ADA balance"
    And I should see "Delegated to: BLOOM"
