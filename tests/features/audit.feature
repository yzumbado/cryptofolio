Feature: Audit command
  As a portfolio user
  I want to audit sync activity and wallet coverage
  So that I can verify all my addresses are being tracked correctly

  Background:
    Given I have a clean test database

  Scenario: Audit coverage with no wallets
    When I run "cryptofolio audit coverage"
    Then the command should succeed
    And I should see "No wallet addresses found"

  Scenario: Audit coverage shows wallet address and never-synced status
    Given I have added a Bitcoin wallet "My BTC Wallet"
    When I run "cryptofolio audit coverage"
    Then the command should succeed
    And I should see "My BTC Wallet"
    And I should see "bitcoin"
    And I should see "never"

  Scenario: Audit coverage shows multiple chains
    Given I have added a Bitcoin wallet "My BTC Wallet"
    And I have added an Ethereum wallet "My ETH Wallet"
    When I run "cryptofolio audit coverage"
    Then the command should succeed
    And I should see "My BTC Wallet"
    And I should see "My ETH Wallet"
    And I should see "bitcoin"
    And I should see "ethereum"

  Scenario: Audit coverage filtered by chain
    Given I have added a Bitcoin wallet "My BTC Wallet"
    And I have added an Ethereum wallet "My ETH Wallet"
    When I run "cryptofolio audit coverage --chain bitcoin"
    Then the command should succeed
    And I should see "bitcoin"
    And I should not see "ethereum"

  Scenario: Audit sync with no log entries
    When I run "cryptofolio audit sync"
    Then the command should succeed
    And I should see "No sync audit entries found"

  Scenario: Audit errors with no errors
    When I run "cryptofolio audit errors"
    Then the command should succeed
    And I should see "No sync errors found"

  Scenario: Audit coverage JSON output
    Given I have added a Bitcoin wallet "My BTC Wallet"
    When I run "cryptofolio audit coverage --json"
    Then the command should succeed
    And I should see "bitcoin"
    And I should see "My BTC Wallet"
