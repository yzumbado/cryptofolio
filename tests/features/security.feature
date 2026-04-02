Feature: Private key security guard
  As a watch-only portfolio tracker
  I want the system to reject private key material at the wallet add boundary
  So that private keys are never written to the database

  Background:
    Given I have a clean test database

  Scenario: Reject Bitcoin WIF private key
    When I run "cryptofolio wallet add 'Bad Wallet' --blockchain bitcoin --address 5HpHagT65TZzG1PH3CSu63k8DbpvD8s5ip4nEB3kEsreAnchuDf"
    Then the command should fail
    And I should see "watch-only"

  Scenario: Reject Ethereum raw private key
    When I run "cryptofolio wallet add 'Bad ETH Wallet' --blockchain ethereum --address 4c0883a69102937d6231471b5dbb6e538eba2ef62a0b8fe5b30f24a6f2b5f7a8"
    Then the command should fail
    And I should see "watch-only"

  Scenario: Reject BIP39 seed phrase
    When I run "cryptofolio wallet add 'Seed Wallet' --blockchain bitcoin --address 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about'"
    Then the command should fail
    And I should see "watch-only"

  Scenario: Valid Bitcoin address is accepted
    When I run "cryptofolio wallet add 'Legit Wallet' --blockchain bitcoin --address bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    Then the command should succeed
    And I should see "Added wallet"

  Scenario: Valid Ethereum address is accepted
    When I run "cryptofolio wallet add 'Legit ETH' --blockchain ethereum --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
    Then the command should succeed
    And I should see "Added wallet"
