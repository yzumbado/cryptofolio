#![allow(dead_code)]

use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;

use super::models::{BinanceDeposit, BinanceFiatOrder, BinanceTrade, BinanceTransfer, BinanceWithdrawal};
use crate::core::pnl::{CostBasisMethod, PnLCalculator};
use crate::core::transaction::{Transaction, TransactionType};
use crate::db::{HoldingRepository, TransactionRepository};
use crate::error::{CryptofolioError, Result};

// =============================================================================
// Public types
// =============================================================================

/// Result of attempting to import a single Binance record.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportResult {
    /// Record was new and a transaction was created with this ID.
    Created(i64),
    /// Record already existed (duplicate external_id) — skipped.
    Skipped(String),
}

impl ImportResult {
    pub fn is_created(&self) -> bool {
        matches!(self, ImportResult::Created(_))
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, ImportResult::Skipped(_))
    }
}

// =============================================================================
// Importer
// =============================================================================

/// Converts Binance API records into local Cryptofolio transactions, updating
/// holdings and P&L as it goes.
pub struct TransactionImporter<'a> {
    pool: &'a SqlitePool,
    tx_repo: TransactionRepository<'a>,
    holding_repo: HoldingRepository<'a>,
    pnl_calc: PnLCalculator<'a>,
}

impl<'a> TransactionImporter<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            tx_repo: TransactionRepository::new(pool),
            holding_repo: HoldingRepository::new(pool),
            pnl_calc: PnLCalculator::new(pool),
        }
    }

    // -------------------------------------------------------------------------
    // Trade import
    // -------------------------------------------------------------------------

    /// Import a single Binance spot trade as a Buy or Sell transaction.
    ///
    /// A "buy" trade (is_buyer = true) means the account received the base
    /// asset (e.g. BTC in BTCUSDT). A "sell" trade means the account gave
    /// away the base asset.
    pub async fn import_trade(
        &self,
        account_id: &str,
        trade: &BinanceTrade,
        dry_run: bool,
    ) -> Result<ImportResult> {
        let external_id = format!("binance-trade-{}", trade.id);

        if self.is_duplicate(&external_id).await? {
            return Ok(ImportResult::Skipped(format!(
                "Trade {} already imported",
                trade.id
            )));
        }

        let (base_asset, quote_asset) = parse_symbol(&trade.symbol)?;
        let timestamp = ms_to_datetime(trade.time)?;

        // Use trade price as USD price when quote is a stablecoin
        let price_usd = if is_usd_equivalent(&quote_asset) {
            Some(trade.price)
        } else {
            None
        };

        if dry_run {
            // Return Created(0) to indicate "would create" in reporting
            return Ok(ImportResult::Created(0));
        }

        let tx_id = if trade.is_buyer {
            // Buying base asset (e.g. buying BTC with USDT)
            let mut tx = Transaction::new_buy(account_id, &base_asset, trade.qty, trade.price, timestamp);
            tx.fee = Some(trade.commission);
            tx.fee_asset = Some(trade.commission_asset.clone());
            tx.external_id = Some(external_id);
            tx.notes = Some(format!(
                "Binance trade #{} — {} {}",
                trade.id, trade.symbol, if trade.is_maker { "maker" } else { "taker" }
            ));
            tx.price_usd = price_usd;

            let id = self.tx_repo.insert(&tx).await?;

            // Update holdings
            self.holding_repo
                .add_quantity(account_id, &base_asset, trade.qty, price_usd)
                .await?;

            // Create tax lot for P&L tracking
            self.pnl_calc
                .process_acquisition(
                    id,
                    account_id,
                    &base_asset,
                    trade.qty,
                    trade.price,
                    timestamp,
                    CostBasisMethod::Fifo,
                )
                .await?;

            id
        } else {
            // Selling base asset (e.g. selling BTC for USDT)
            let mut tx = Transaction::new_sell(account_id, &base_asset, trade.qty, trade.price, timestamp);
            tx.fee = Some(trade.commission);
            tx.fee_asset = Some(trade.commission_asset.clone());
            tx.external_id = Some(external_id);
            tx.notes = Some(format!(
                "Binance trade #{} — {} {}",
                trade.id, trade.symbol, if trade.is_maker { "maker" } else { "taker" }
            ));
            tx.price_usd = price_usd;

            let id = self.tx_repo.insert(&tx).await?;

            // Update holdings
            // Ignore insufficient-balance errors on import (holdings might not
            // be fully seeded from before the sync window)
            let _ = self
                .holding_repo
                .remove_quantity(account_id, &base_asset, trade.qty)
                .await;

            // Record P&L
            let _ = self
                .pnl_calc
                .process_disposal(
                    id,
                    account_id,
                    &base_asset,
                    trade.qty,
                    trade.price,
                    timestamp,
                    CostBasisMethod::Fifo,
                )
                .await;

            id
        };

        Ok(ImportResult::Created(tx_id))
    }

    // -------------------------------------------------------------------------
    // Deposit import
    // -------------------------------------------------------------------------

    /// Import a Binance crypto deposit as a TransferIn transaction.
    ///
    /// Only imports deposits with status = 1 (success). Pending or failed
    /// deposits are skipped.
    pub async fn import_deposit(
        &self,
        account_id: &str,
        deposit: &BinanceDeposit,
        dry_run: bool,
    ) -> Result<ImportResult> {
        // Only import successful deposits
        if deposit.status != 1 && deposit.status != 6 {
            return Ok(ImportResult::Skipped(format!(
                "Deposit {} has non-success status {}",
                deposit.id, deposit.status
            )));
        }

        let external_id = format!("binance-deposit-{}", deposit.id);

        if self.is_duplicate(&external_id).await? {
            return Ok(ImportResult::Skipped(format!(
                "Deposit {} already imported",
                deposit.id
            )));
        }

        let timestamp = ms_to_datetime(deposit.insert_time)?;

        if dry_run {
            // Return Created(0) to indicate "would create" in reporting
            return Ok(ImportResult::Created(0));
        }

        // Build a transfer_in transaction (no cost basis — it's a transfer)
        let mut tx = Transaction {
            id: 0,
            tx_type: TransactionType::TransferIn,
            from_account_id: None,
            from_asset: None,
            from_quantity: None,
            to_account_id: Some(account_id.to_string()),
            to_asset: Some(deposit.coin.to_uppercase()),
            to_quantity: Some(deposit.amount),
            price_usd: None,
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: Some(external_id),
            notes: Some(format!(
                "Binance deposit #{} via {} network{}",
                deposit.id,
                deposit.network,
                deposit
                    .tx_id
                    .as_ref()
                    .map(|txid| format!(" (txid: {})", &txid[..txid.len().min(16)]))
                    .unwrap_or_default(),
            )),
            timestamp,
            created_at: Utc::now(),
        };

        let tx_id = self.tx_repo.insert(&tx).await?;

        // Update holdings (no cost basis for deposits)
        self.holding_repo
            .add_quantity(account_id, &deposit.coin.to_uppercase(), deposit.amount, None)
            .await?;

        tx.id = tx_id;
        Ok(ImportResult::Created(tx_id))
    }

    // -------------------------------------------------------------------------
    // Withdrawal import
    // -------------------------------------------------------------------------

    /// Import a Binance crypto withdrawal as a TransferOut transaction.
    ///
    /// Only imports withdrawals with status = 6 (completed).
    pub async fn import_withdrawal(
        &self,
        account_id: &str,
        withdrawal: &BinanceWithdrawal,
        dry_run: bool,
    ) -> Result<ImportResult> {
        // Only import completed withdrawals (status 6)
        if withdrawal.status != 6 {
            return Ok(ImportResult::Skipped(format!(
                "Withdrawal {} has non-success status {}",
                withdrawal.id, withdrawal.status
            )));
        }

        let external_id = format!("binance-withdrawal-{}", withdrawal.id);

        if self.is_duplicate(&external_id).await? {
            return Ok(ImportResult::Skipped(format!(
                "Withdrawal {} already imported",
                withdrawal.id
            )));
        }

        let timestamp = parse_binance_datetime(&withdrawal.apply_time)?;

        if dry_run {
            // Return Created(0) to indicate "would create" in reporting
            return Ok(ImportResult::Created(0));
        }

        // Total sent = amount + fee
        let total_sent = withdrawal.amount + withdrawal.transaction_fee;

        let tx = Transaction {
            id: 0,
            tx_type: TransactionType::TransferOut,
            from_account_id: Some(account_id.to_string()),
            from_asset: Some(withdrawal.coin.to_uppercase()),
            from_quantity: Some(total_sent),
            to_account_id: None,
            to_asset: None,
            to_quantity: None,
            price_usd: None,
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: Some(withdrawal.transaction_fee),
            fee_asset: Some(withdrawal.coin.to_uppercase()),
            external_id: Some(external_id),
            notes: Some(format!(
                "Binance withdrawal #{} via {} to {}{}",
                withdrawal.id,
                withdrawal.network,
                &withdrawal.address[..withdrawal.address.len().min(16)],
                withdrawal
                    .tx_id
                    .as_ref()
                    .map(|txid| format!(" (txid: {})", &txid[..txid.len().min(16)]))
                    .unwrap_or_default(),
            )),
            timestamp,
            created_at: Utc::now(),
        };

        let tx_id = self.tx_repo.insert(&tx).await?;

        // Update holdings (silently ignore if not enough balance)
        let _ = self
            .holding_repo
            .remove_quantity(account_id, &withdrawal.coin.to_uppercase(), total_sent)
            .await;

        Ok(ImportResult::Created(tx_id))
    }

    // -------------------------------------------------------------------------
    // Fiat order import
    // -------------------------------------------------------------------------

    /// Import a completed Binance fiat deposit order (credit card → crypto).
    ///
    /// Only "Completed" orders are imported; others are skipped.
    pub async fn import_fiat_order(
        &self,
        account_id: &str,
        order: &BinanceFiatOrder,
        dry_run: bool,
    ) -> Result<ImportResult> {
        if order.status != "Completed" {
            return Ok(ImportResult::Skipped(format!(
                "Fiat order {} has status '{}', skipping",
                order.order_no, order.status
            )));
        }

        let external_id = format!("binance-fiat-{}", order.order_no);

        if self.is_duplicate(&external_id).await? {
            return Ok(ImportResult::Skipped(format!(
                "Fiat order {} already imported",
                order.order_no
            )));
        }

        let timestamp = ms_to_datetime(order.create_time)?;

        if dry_run {
            // Return Created(0) to indicate "would create" in reporting
            return Ok(ImportResult::Created(0));
        }

        // Effective price per unit: fiat_amount / crypto_amount
        let price_usd = if order.amount > Decimal::ZERO {
            Some(order.indicated_amount / order.amount)
        } else {
            None
        };

        // Model as a Buy (fiat → crypto)
        let mut tx = Transaction::new_buy(
            account_id,
            &order.crypto_currency.to_uppercase(),
            order.amount,
            price_usd.unwrap_or(Decimal::ZERO),
            timestamp,
        );
        tx.fee = if order.total_fee > Decimal::ZERO {
            Some(order.total_fee)
        } else {
            None
        };
        tx.fee_asset = Some(order.fiat_currency.clone());
        tx.external_id = Some(external_id);
        tx.notes = Some(format!(
            "Binance fiat purchase #{} — {} {} via {} (paid {} {})",
            order.order_no,
            order.amount,
            order.crypto_currency,
            order.method,
            order.indicated_amount,
            order.fiat_currency,
        ));
        tx.price_usd = price_usd;

        let tx_id = self.tx_repo.insert(&tx).await?;

        // Update holdings
        self.holding_repo
            .add_quantity(
                account_id,
                &order.crypto_currency.to_uppercase(),
                order.amount,
                price_usd,
            )
            .await?;

        // Create tax lot
        if let Some(price) = price_usd {
            let _ = self
                .pnl_calc
                .process_acquisition(
                    tx_id,
                    account_id,
                    &order.crypto_currency.to_uppercase(),
                    order.amount,
                    price,
                    timestamp,
                    CostBasisMethod::Fifo,
                )
                .await;
        }

        Ok(ImportResult::Created(tx_id))
    }

    // -------------------------------------------------------------------------
    // Internal transfer import
    // -------------------------------------------------------------------------

    /// Import a Binance internal transfer (e.g. Spot ↔ Earn, bot allocations).
    ///
    /// Only "CONFIRMED" transfers are imported. The transfer is modelled as a
    /// TransferInternal within the same account (same account_id, different
    /// sub-wallet — we don't track sub-wallets separately).
    pub async fn import_transfer(
        &self,
        account_id: &str,
        transfer: &BinanceTransfer,
        dry_run: bool,
    ) -> Result<ImportResult> {
        if transfer.status != "CONFIRMED" {
            return Ok(ImportResult::Skipped(format!(
                "Transfer {} has status '{}', skipping",
                transfer.tran_id, transfer.status
            )));
        }

        let external_id = format!("binance-transfer-{}", transfer.tran_id);

        if self.is_duplicate(&external_id).await? {
            return Ok(ImportResult::Skipped(format!(
                "Transfer {} already imported",
                transfer.tran_id
            )));
        }

        let timestamp = ms_to_datetime(transfer.timestamp)?;

        if dry_run {
            // Return Created(0) to indicate "would create" in reporting
            return Ok(ImportResult::Created(0));
        }

        // Internal transfer — same account on both sides (sub-wallet not tracked)
        let mut tx = Transaction::new_transfer(
            account_id,
            account_id,
            &transfer.asset.to_uppercase(),
            transfer.amount,
            timestamp,
        );
        tx.external_id = Some(external_id);
        tx.notes = Some(format!(
            "Binance internal transfer #{} type: {}",
            transfer.tran_id, transfer.transfer_type
        ));

        let tx_id = self.tx_repo.insert(&tx).await?;

        // Holdings don't change (same account both sides)

        Ok(ImportResult::Created(tx_id))
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Returns true if a transaction with the given external_id already exists.
    async fn is_duplicate(&self, external_id: &str) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM transactions WHERE external_id = ? LIMIT 1",
        )
        .bind(external_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.is_some())
    }
}

// =============================================================================
// Pure utility functions (pub for use by SyncOrchestrator)
// =============================================================================

/// Parse a Binance trading pair into (base_asset, quote_asset).
///
/// Tries known quote assets in order of length (longest first) to avoid
/// ambiguous matches (e.g. "BTCUSDT" vs "BTCUSD").
pub fn parse_symbol(symbol: &str) -> Result<(String, String)> {
    let s = symbol.to_uppercase();

    // Known quote assets, longest first to avoid prefix confusion
    const QUOTE_ASSETS: &[&str] = &[
        "USDT", "BUSD", "USDC", "TUSD", "USDP", "DAI", "BTC", "ETH", "BNB",
    ];

    for quote in QUOTE_ASSETS {
        if s.ends_with(quote) && s.len() > quote.len() {
            let base = s[..s.len() - quote.len()].to_string();
            return Ok((base, quote.to_string()));
        }
    }

    Err(CryptofolioError::Other(format!(
        "Cannot parse trading pair symbol: '{}'",
        symbol
    )))
}

/// Returns true if the asset is a USD-equivalent stablecoin or fiat.
pub fn is_usd_equivalent(asset: &str) -> bool {
    matches!(
        asset.to_uppercase().as_str(),
        "USDT" | "BUSD" | "USDC" | "TUSD" | "USDP" | "DAI" | "USD"
    )
}

/// Convert a Binance millisecond timestamp to DateTime<Utc>.
pub fn ms_to_datetime(ms: i64) -> Result<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .ok_or_else(|| CryptofolioError::Other(format!("Invalid Binance timestamp: {}", ms)))
}

/// Parse Binance datetime string format "YYYY-MM-DD HH:MM:SS" to DateTime<Utc>.
pub fn parse_binance_datetime(datetime_str: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_str(&format!("{} +0000", datetime_str), "%Y-%m-%d %H:%M:%S %z")
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CryptofolioError::Other(format!(
            "Invalid Binance datetime format '{}': {}",
            datetime_str, e
        )))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_symbol ----

    #[test]
    fn test_parse_symbol_usdt_pair() {
        assert_eq!(parse_symbol("BTCUSDT").unwrap(), ("BTC".into(), "USDT".into()));
        assert_eq!(parse_symbol("ETHUSDT").unwrap(), ("ETH".into(), "USDT".into()));
        assert_eq!(parse_symbol("ADAUSDT").unwrap(), ("ADA".into(), "USDT".into()));
    }

    #[test]
    fn test_parse_symbol_btc_pair() {
        assert_eq!(parse_symbol("ETHBTC").unwrap(), ("ETH".into(), "BTC".into()));
        assert_eq!(parse_symbol("BNBBTC").unwrap(), ("BNB".into(), "BTC".into()));
    }

    #[test]
    fn test_parse_symbol_busd_pair() {
        assert_eq!(parse_symbol("BTCBUSD").unwrap(), ("BTC".into(), "BUSD".into()));
    }

    #[test]
    fn test_parse_symbol_case_insensitive() {
        assert_eq!(parse_symbol("btcusdt").unwrap(), ("BTC".into(), "USDT".into()));
        assert_eq!(parse_symbol("EthUsdt").unwrap(), ("ETH".into(), "USDT".into()));
    }

    #[test]
    fn test_parse_symbol_unknown_returns_error() {
        assert!(parse_symbol("BTCXXX").is_err());
        assert!(parse_symbol("").is_err());
    }

    // ---- is_usd_equivalent ----

    #[test]
    fn test_is_usd_equivalent() {
        assert!(is_usd_equivalent("USDT"));
        assert!(is_usd_equivalent("BUSD"));
        assert!(is_usd_equivalent("USDC"));
        assert!(is_usd_equivalent("usdt")); // case insensitive
        assert!(!is_usd_equivalent("BTC"));
        assert!(!is_usd_equivalent("ETH"));
    }

    // ---- ms_to_datetime ----

    #[test]
    fn test_ms_to_datetime_valid() {
        let dt = ms_to_datetime(1_499_865_549_590).unwrap();
        assert_eq!(dt.timestamp(), 1_499_865_549);
    }

    #[test]
    fn test_ms_to_datetime_zero() {
        // 0 ms = Unix epoch
        let dt = ms_to_datetime(0).unwrap();
        assert_eq!(dt.timestamp(), 0);
    }

    // ---- ImportResult helpers ----

    #[test]
    fn test_import_result_is_created() {
        assert!(ImportResult::Created(42).is_created());
        assert!(!ImportResult::Skipped("dup".into()).is_created());
    }

    #[test]
    fn test_import_result_is_skipped() {
        assert!(ImportResult::Skipped("dup".into()).is_skipped());
        assert!(!ImportResult::Created(1).is_skipped());
    }

    // ---- Integration-style tests using in-memory DB ----

    use crate::db::migrations;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    async fn setup_db() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        migrations::run(&pool).await.unwrap();
        sqlx::query("INSERT INTO categories (id, name) VALUES ('cat', 'Test')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('acc', 'cat', 'Binance', 'exchange')"
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn make_trade(id: i64, symbol: &str, is_buyer: bool) -> BinanceTrade {
        BinanceTrade {
            symbol: symbol.to_string(),
            id,
            order_id: 100 + id,
            price: Decimal::from_str("50000").unwrap(),
            qty: Decimal::from_str("0.1").unwrap(),
            quote_qty: Decimal::from_str("5000").unwrap(),
            commission: Decimal::from_str("0.0001").unwrap(),
            commission_asset: "BTC".to_string(),
            time: 1_700_000_000_000,
            is_buyer,
            is_maker: false,
        }
    }

    fn make_deposit(id: &str, coin: &str, status: i32) -> BinanceDeposit {
        BinanceDeposit {
            id: id.to_string(),
            amount: Decimal::from_str("0.5").unwrap(),
            coin: coin.to_string(),
            network: "BTC".to_string(),
            status,
            address: Some("1A2B3C".to_string()),
            tx_id: Some("abc123".to_string()),
            insert_time: 1_700_000_000_000,
            confirm_times: None,
        }
    }

    fn make_withdrawal(id: &str, coin: &str, status: i32) -> BinanceWithdrawal {
        BinanceWithdrawal {
            id: id.to_string(),
            amount: Decimal::from_str("0.4").unwrap(),
            transaction_fee: Decimal::from_str("0.0005").unwrap(),
            coin: coin.to_string(),
            status,
            address: "1A2B3Cdeadbeef".to_string(),
            tx_id: Some("def456".to_string()),
            apply_time: "2023-11-14 22:30:00".to_string(),  // Changed to string format
            network: "BTC".to_string(),
            withdraw_order_id: None,
            transfer_type: None,
            info: None,
            confirm_no: None,
            wallet_type: None,
            tx_key: None,
            complete_time: None,
        }
    }

    #[tokio::test]
    async fn test_import_buy_trade_creates_transaction() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);
        let trade = make_trade(1, "BTCUSDT", true);

        let result = importer.import_trade("acc", &trade, false).await.unwrap();
        assert!(result.is_created());

        // Holdings should reflect the purchase
        let holding_repo = HoldingRepository::new(&pool);
        let holding = holding_repo.get("acc", "BTC").await.unwrap().unwrap();
        assert_eq!(holding.quantity, Decimal::from_str("0.1").unwrap());
    }

    #[tokio::test]
    async fn test_import_trade_duplicate_skipped() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);
        let trade = make_trade(2, "BTCUSDT", true);

        let r1 = importer.import_trade("acc", &trade, false).await.unwrap();
        let r2 = importer.import_trade("acc", &trade, false).await.unwrap();

        assert!(r1.is_created());
        assert!(r2.is_skipped());
    }

    #[tokio::test]
    async fn test_import_trade_dry_run_no_side_effects() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);
        let trade = make_trade(3, "ETHUSDT", true);

        let result = importer.import_trade("acc", &trade, true).await.unwrap();
        // Dry-run returns Created(0) to indicate "would create" in reporting
        assert!(result.is_created());

        // No holdings should have been created
        let holding_repo = HoldingRepository::new(&pool);
        let holding = holding_repo.get("acc", "ETH").await.unwrap();
        assert!(holding.is_none());
    }

    #[tokio::test]
    async fn test_import_deposit_success() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);
        let deposit = make_deposit("dep-1", "BTC", 1);

        let result = importer.import_deposit("acc", &deposit, false).await.unwrap();
        assert!(result.is_created());

        let holding_repo = HoldingRepository::new(&pool);
        let holding = holding_repo.get("acc", "BTC").await.unwrap().unwrap();
        assert_eq!(holding.quantity, Decimal::from_str("0.5").unwrap());
    }

    #[tokio::test]
    async fn test_import_deposit_pending_skipped() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);
        let deposit = make_deposit("dep-2", "ETH", 0); // 0 = pending

        let result = importer.import_deposit("acc", &deposit, false).await.unwrap();
        assert!(result.is_skipped());
    }

    #[tokio::test]
    async fn test_import_withdrawal_success() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);

        // First give the account some BTC to withdraw
        let holding_repo = HoldingRepository::new(&pool);
        holding_repo
            .add_quantity("acc", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await
            .unwrap();

        let withdrawal = make_withdrawal("wd-1", "BTC", 6);
        let result = importer.import_withdrawal("acc", &withdrawal, false).await.unwrap();
        assert!(result.is_created());
    }

    #[tokio::test]
    async fn test_import_withdrawal_non_success_skipped() {
        let pool = setup_db().await;
        let importer = TransactionImporter::new(&pool);
        let withdrawal = make_withdrawal("wd-2", "ETH", 2); // 2 = awaiting approval

        let result = importer.import_withdrawal("acc", &withdrawal, false).await.unwrap();
        assert!(result.is_skipped());
    }
}
