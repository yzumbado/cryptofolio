Feature: Wallet Management
  As a crypto portfolio tracker
  I want to add and manage wallet addresses
  So that I can track my holdings across different blockchains

  Background:
    Given I have a clean test database

  Scenario: Add a Bitcoin wallet with single address
    When I run "cryptofolio wallet add 'My BTC Wallet' --blockchain bitcoin --address bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    Then the command should succeed
    And I should see "✓ Added wallet"
    And I should have 1 wallets
    And the wallet should be for blockchain "bitcoin"

  Scenario: Add an Ethereum wallet
    When I run "cryptofolio wallet add 'My ETH Wallet' --blockchain ethereum --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should succeed
    And I should see "✓ Added wallet"

  Scenario: Add a Bitcoin HD wallet with xpub
    When I run "cryptofolio wallet add 'Sparrow Wallet' --blockchain bitcoin --xpub zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvphXR5ePCqYAN5qRbNnCLanT9qDKnNT4yKYr8j6L51HvvPahBJPJJZpNAQTwD"
    Then the command should succeed
    And I should see "✓ Added HD wallet"

  Scenario: List all wallets
    Given I have added a Bitcoin wallet "My BTC Wallet"
    And I have added an Ethereum wallet "My ETH Wallet"
    When I run "cryptofolio wallet list"
    Then the command should succeed
    And I should see "My BTC Wallet"
    And I should see "My ETH Wallet"
    And I should see "bitcoin"
    And I should see "ethereum"

  Scenario: Reject invalid Bitcoin address
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain bitcoin --address invalid_address"
    Then the command should fail
    And I should see "Invalid Bitcoin address"

  Scenario: Reject duplicate wallet address
    Given I have added a Bitcoin wallet "Wallet 1"
    When I run "cryptofolio wallet add 'Wallet 2' --blockchain bitcoin --address bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    Then the command should fail
    And I should see "already exists"
