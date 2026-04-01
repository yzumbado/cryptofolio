# @wip — scenarios require Binance API wiremock setup; excluded from CI until implemented
@wip
Feature: Binance History Sync
  As a Binance trader
  I want to sync my trading history from Binance
  So that I can track my trades, deposits, and withdrawals

  Background:
    Given I have a clean test database
    And I have configured Binance API credentials

  Scenario: Sync trade history for a single symbol
    Given the Binance API shows 10 trades for BTCUSDT
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run"
    Then the command should succeed
    And I should see "Trades: 10 would import"

  Scenario: Sync deposit history
    Given the Binance API shows 5 USDT deposits
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run"
    Then the command should succeed
    And I should see "Deposits: 5 would import"

  Scenario: Sync withdrawal history with datetime format
    Given the Binance API shows 3 withdrawals with datetime strings
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run"
    Then the command should succeed
    And I should see "Withdrawals: 3 would import"
    And the withdrawal timestamps should be parsed correctly

  Scenario: Sync multiple symbols at once
    Given the Binance API shows trades for BTCUSDT, ETHUSDT, and BNBUSDT
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT,ETHUSDT,BNBUSDT --dry-run"
    Then the command should succeed
    And I should see trades for all three symbols

  Scenario: Full history sync with --full-history flag
    Given I have previously synced Binance history
    And there are 5 new trades since last sync
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT --full-history --dry-run"
    Then the command should succeed
    And I should see all historical trades, not just new ones

  Scenario: Incremental sync only fetches new data
    Given I have synced Binance history previously
    And sync state shows last trade ID is 12345
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT --dry-run"
    Then the command should succeed
    And I should only see trades after ID 12345

  Scenario: Duplicate detection prevents re-importing
    Given I have imported 10 trades from Binance
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT"
    Then the command should succeed
    And I should see "0 imported, 10 skipped"

  Scenario: Real import without dry-run
    Given the Binance API shows 5 new trades
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT"
    Then the command should succeed
    And I should see "5 imported, 0 skipped"
    And the database should contain 5 new transactions

  Scenario: Handle API errors gracefully
    Given the Binance API returns an error
    When I run "cryptofolio sync-history --account Binance --symbols BTCUSDT"
    Then the command should fail
    And I should see a helpful error message with the API error details

  Scenario: Parse withdrawal datetime format correctly
    Given the Binance API returns a withdrawal with applyTime "2026-03-30 05:17:23"
    When I sync the withdrawal history
    Then the withdrawal should be imported with the correct timestamp
    And the timestamp should be in UTC timezone
