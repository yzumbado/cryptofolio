Feature: Bitcoin Wallet Sync
  As a Bitcoin holder
  I want to sync my wallet from the blockchain
  So that I can track my balance and transaction history

  Background:
    Given I have a clean test database

  Scenario: Sync Bitcoin wallet from public API
    Given I have added a Bitcoin wallet "My Wallet"
    And the Bitcoin blockchain shows balance of 0.5 BTC
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then the command should succeed
    And I should see "✓ Synced BTC balance: 0.5000"
    And the wallet balance should be "0.5" BTC

  Scenario: Sync Bitcoin HD wallet with xpub
    Given I have added a Bitcoin wallet "Sparrow" with xpub
    And the Bitcoin blockchain shows 3 addresses with transactions
    When I run "cryptofolio wallet sync 'Sparrow'"
    Then the command should succeed
    And I should see "✓ Found 3 addresses with activity"
    And I should see "✓ Imported 15 transactions"

  Scenario: Sync with local Bitcoin Core node
    Given I have a local Bitcoin Core node running
    And I have added a Bitcoin wallet "Cold Storage"
    When I run "cryptofolio wallet sync 'Cold Storage' --use-local-node"
    Then the command should succeed
    And I should see "Using local node"
    And I should not see "Using public API"

  Scenario: Fallback to public API when local node unavailable
    Given I have a Bitcoin wallet "My Wallet"
    And my local Bitcoin Core node is not running
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then the command should succeed
    And I should see "Local node unavailable, using public API"

  Scenario: Import transaction history
    Given I have added a Bitcoin wallet "Active Wallet"
    And the blockchain shows 5 incoming and 2 outgoing Bitcoin transactions
    When I run "cryptofolio wallet sync 'Active Wallet' --import-history"
    Then the command should succeed
    And I should see "✓ Imported 7 transactions"
    And I should have 7 transactions in the database

  Scenario: Incremental sync (only new transactions)
    Given I have synced a Bitcoin wallet "My Wallet" previously
    And there are 2 new transactions since last sync
    When I run "cryptofolio wallet sync 'My Wallet'"
    Then the command should succeed
    And I should see "✓ Imported 2 new transactions"
    And I should not see "already exists"
