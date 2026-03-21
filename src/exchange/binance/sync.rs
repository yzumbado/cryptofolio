#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use super::client::BinanceClient;
use super::import::{ImportResult, TransactionImporter};
use crate::db::SyncStateRepository;
use crate::error::Result;

// =============================================================================
// Report
// =============================================================================

/// Summary returned after a sync run.
#[derive(Debug, Default, Clone)]
pub struct SyncReport {
    pub trades_created: usize,
    pub trades_skipped: usize,
    pub deposits_created: usize,
    pub deposits_skipped: usize,
    pub withdrawals_created: usize,
    pub withdrawals_skipped: usize,
    pub fiat_orders_created: usize,
    pub fiat_orders_skipped: usize,
    pub transfers_created: usize,
    pub transfers_skipped: usize,
    /// Non-fatal errors encountered during the sync.
    pub errors: Vec<String>,
}

impl SyncReport {
    pub fn total_created(&self) -> usize {
        self.trades_created
            + self.deposits_created
            + self.withdrawals_created
            + self.fiat_orders_created
            + self.transfers_created
    }

    pub fn total_skipped(&self) -> usize {
        self.trades_skipped
            + self.deposits_skipped
            + self.withdrawals_skipped
            + self.fiat_orders_skipped
            + self.transfers_skipped
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

// =============================================================================
// Options
// =============================================================================

/// Controls what a sync run fetches.
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Trading symbols to sync (e.g. ["BTCUSDT", "ETHUSDT"]).
    /// If empty, no trades are synced.
    pub symbols: Vec<String>,
    /// Fetch trade history
    pub sync_trades: bool,
    /// Fetch deposit history
    pub sync_deposits: bool,
    /// Fetch withdrawal history
    pub sync_withdrawals: bool,
    /// Fetch fiat purchase history
    pub sync_fiat: bool,
    /// Fetch internal transfer history (Spot ↔ Earn, bots, etc.)
    pub sync_transfers: bool,
    /// Transfer types to fetch (e.g. "MAIN_UMFUTURE", "UMFUTURE_MAIN").
    /// Only relevant when sync_transfers = true.
    pub transfer_types: Vec<String>,
    /// If true, ignore the last-sync watermark and fetch from the beginning.
    pub full_history: bool,
    /// If true, show what would be imported without writing anything.
    pub dry_run: bool,
    /// Optional override for the earliest timestamp to fetch (millis since epoch).
    pub start_time: Option<DateTime<Utc>>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            symbols: vec![],
            sync_trades: true,
            sync_deposits: true,
            sync_withdrawals: true,
            sync_fiat: true,
            sync_transfers: true,
            // Note: Only include transfer types that are commonly available
            // MAIN_UMFUTURE, UMFUTURE_MAIN have decoding issues
            // MAIN_C2C, C2C_MAIN not available for all accounts
            transfer_types: vec![
                // Disabled by default - uncomment if you have futures/C2C enabled:
                // "MAIN_UMFUTURE".to_string(),
                // "UMFUTURE_MAIN".to_string(),
                // "MAIN_C2C".to_string(),
                // "C2C_MAIN".to_string(),
            ],
            full_history: false,
            dry_run: false,
            start_time: None,
        }
    }
}

// =============================================================================
// Orchestrator
// =============================================================================

/// Coordinates fetching Binance history, importing records, and persisting
/// sync-state watermarks so subsequent runs are incremental.
pub struct SyncOrchestrator<'a> {
    pool: &'a SqlitePool,
    client: &'a BinanceClient,
    importer: TransactionImporter<'a>,
    sync_state_repo: SyncStateRepository<'a>,
}

impl<'a> SyncOrchestrator<'a> {
    pub fn new(pool: &'a SqlitePool, client: &'a BinanceClient) -> Self {
        Self {
            pool,
            client,
            importer: TransactionImporter::new(pool),
            sync_state_repo: SyncStateRepository::new(pool),
        }
    }

    /// Run a full sync for `account_id` according to `opts`.
    pub async fn sync(
        &self,
        account_id: &str,
        opts: &SyncOptions,
    ) -> Result<SyncReport> {
        let mut report = SyncReport::default();

        // Ensure a sync-state row exists for this account
        let state = self.sync_state_repo.get_or_create(account_id).await?;

        // Determine start_time for each endpoint
        let trade_start = if opts.full_history {
            opts.start_time.map(|dt| dt.timestamp_millis())
        } else {
            opts.start_time
                .or(state.last_trade_sync)
                .map(|dt| dt.timestamp_millis())
        };

        let deposit_start = if opts.full_history {
            opts.start_time.map(|dt| dt.timestamp_millis())
        } else {
            opts.start_time
                .or(state.last_deposit_sync)
                .map(|dt| dt.timestamp_millis())
        };

        let withdrawal_start = if opts.full_history {
            opts.start_time.map(|dt| dt.timestamp_millis())
        } else {
            opts.start_time
                .or(state.last_withdrawal_sync)
                .map(|dt| dt.timestamp_millis())
        };

        let fiat_start = if opts.full_history {
            opts.start_time.map(|dt| dt.timestamp_millis())
        } else {
            opts.start_time
                .or(state.last_fiat_sync)
                .map(|dt| dt.timestamp_millis())
        };

        let transfer_start = if opts.full_history {
            opts.start_time.map(|dt| dt.timestamp_millis())
        } else {
            opts.start_time
                .or(state.last_transfer_sync)
                .map(|dt| dt.timestamp_millis())
        };

        let now = Utc::now();

        // --- Trades ---
        if opts.sync_trades && !opts.symbols.is_empty() {
            self.sync_trades(
                account_id,
                &opts.symbols,
                trade_start,
                opts.dry_run,
                &mut report,
            )
            .await;

            if !opts.dry_run && report.errors.is_empty() {
                let _ = self
                    .sync_state_repo
                    .update_trade_sync(account_id, now, None, None)
                    .await;
            }
        }

        // --- Deposits ---
        if opts.sync_deposits {
            self.sync_deposits(account_id, deposit_start, opts.dry_run, &mut report)
                .await;

            if !opts.dry_run {
                let _ = self
                    .sync_state_repo
                    .update_deposit_sync(account_id, now)
                    .await;
            }
        }

        // --- Withdrawals ---
        if opts.sync_withdrawals {
            self.sync_withdrawals(account_id, withdrawal_start, opts.dry_run, &mut report)
                .await;

            if !opts.dry_run {
                let _ = self
                    .sync_state_repo
                    .update_withdrawal_sync(account_id, now)
                    .await;
            }
        }

        // --- Fiat orders ---
        if opts.sync_fiat {
            self.sync_fiat(account_id, fiat_start, opts.dry_run, &mut report)
                .await;

            if !opts.dry_run {
                let _ = self
                    .sync_state_repo
                    .update_fiat_sync(account_id, now)
                    .await;
            }
        }

        // --- Internal transfers ---
        if opts.sync_transfers {
            for transfer_type in &opts.transfer_types {
                self.sync_transfers_for_type(
                    account_id,
                    transfer_type,
                    transfer_start,
                    opts.dry_run,
                    &mut report,
                )
                .await;
            }

            if !opts.dry_run {
                let _ = self
                    .sync_state_repo
                    .update_transfer_sync(account_id, now)
                    .await;
            }
        }

        Ok(report)
    }

    // -------------------------------------------------------------------------
    // Private: endpoint-specific sync methods
    // -------------------------------------------------------------------------

    /// Fetch and import trades for all requested symbols.
    /// Uses fromId pagination within each symbol to get > 1000 trades.
    async fn sync_trades(
        &self,
        account_id: &str,
        symbols: &[String],
        start_time: Option<i64>,
        dry_run: bool,
        report: &mut SyncReport,
    ) {
        for symbol in symbols {
            let mut from_id: Option<i64> = None;

            loop {
                let trades = match self
                    .client
                    .get_my_trades(symbol, from_id, start_time, None, Some(1000))
                    .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        report.errors.push(format!(
                            "Failed to fetch trades for {}: {}",
                            symbol, e
                        ));
                        break;
                    }
                };

                let fetched = trades.len();
                let mut last_id: Option<i64> = None;

                for trade in &trades {
                    last_id = Some(trade.id);
                    match self
                        .importer
                        .import_trade(account_id, trade, dry_run)
                        .await
                    {
                        Ok(ImportResult::Created(_)) => report.trades_created += 1,
                        Ok(ImportResult::Skipped(_)) => report.trades_skipped += 1,
                        Err(e) => report.errors.push(format!(
                            "Error importing trade #{}: {}",
                            trade.id, e
                        )),
                    }
                }

                // If we got a full page, paginate using fromId = last_id + 1
                if fetched == 1000 {
                    from_id = last_id.map(|id| id + 1);
                } else {
                    break;
                }
            }
        }
    }

    /// Fetch and import deposit history with offset pagination.
    async fn sync_deposits(
        &self,
        account_id: &str,
        start_time: Option<i64>,
        dry_run: bool,
        report: &mut SyncReport,
    ) {
        let mut offset = 0i32;
        const PAGE: i32 = 1000;

        loop {
            let deposits = match self
                .client
                .get_deposit_history(None, None, start_time, None, Some(offset), Some(PAGE))
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to fetch deposits: {}", e));
                    break;
                }
            };

            let fetched = deposits.len();

            for deposit in &deposits {
                match self
                    .importer
                    .import_deposit(account_id, deposit, dry_run)
                    .await
                {
                    Ok(ImportResult::Created(_)) => report.deposits_created += 1,
                    Ok(ImportResult::Skipped(_)) => report.deposits_skipped += 1,
                    Err(e) => report.errors.push(format!(
                        "Error importing deposit #{}: {}",
                        deposit.id, e
                    )),
                }
            }

            if fetched as i32 == PAGE {
                offset += PAGE;
            } else {
                break;
            }
        }
    }

    /// Fetch and import withdrawal history with offset pagination.
    async fn sync_withdrawals(
        &self,
        account_id: &str,
        start_time: Option<i64>,
        dry_run: bool,
        report: &mut SyncReport,
    ) {
        let mut offset = 0i32;
        const PAGE: i32 = 1000;

        loop {
            let withdrawals = match self
                .client
                .get_withdrawal_history(None, None, start_time, None, Some(offset), Some(PAGE))
                .await
            {
                Ok(w) => w,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to fetch withdrawals: {}", e));
                    break;
                }
            };

            let fetched = withdrawals.len();

            for withdrawal in &withdrawals {
                match self
                    .importer
                    .import_withdrawal(account_id, withdrawal, dry_run)
                    .await
                {
                    Ok(ImportResult::Created(_)) => report.withdrawals_created += 1,
                    Ok(ImportResult::Skipped(_)) => report.withdrawals_skipped += 1,
                    Err(e) => report.errors.push(format!(
                        "Error importing withdrawal #{}: {}",
                        withdrawal.id, e
                    )),
                }
            }

            if fetched as i32 == PAGE {
                offset += PAGE;
            } else {
                break;
            }
        }
    }

    /// Fetch and import fiat deposit orders with page-based pagination.
    async fn sync_fiat(
        &self,
        account_id: &str,
        start_time: Option<i64>,
        dry_run: bool,
        report: &mut SyncReport,
    ) {
        const PAGE_SIZE: i32 = 500;
        let mut page = 1i32;

        // Fetch deposit orders only (transaction_type "0")
        loop {
            let response = match self
                .client
                .get_fiat_orders("0", start_time, None, Some(page), Some(PAGE_SIZE))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to fetch fiat orders: {}", e));
                    break;
                }
            };

            let fetched = response.data.len();

            for order in &response.data {
                match self
                    .importer
                    .import_fiat_order(account_id, order, dry_run)
                    .await
                {
                    Ok(ImportResult::Created(_)) => report.fiat_orders_created += 1,
                    Ok(ImportResult::Skipped(_)) => report.fiat_orders_skipped += 1,
                    Err(e) => report.errors.push(format!(
                        "Error importing fiat order #{}: {}",
                        order.order_no, e
                    )),
                }
            }

            if fetched as i32 == PAGE_SIZE {
                page += 1;
            } else {
                break;
            }
        }
    }

    /// Fetch and import internal transfers of a given type.
    async fn sync_transfers_for_type(
        &self,
        account_id: &str,
        transfer_type: &str,
        start_time: Option<i64>,
        dry_run: bool,
        report: &mut SyncReport,
    ) {
        let mut page = 1i32;
        const PAGE_SIZE: i32 = 100;

        loop {
            let response = match self
                .client
                .get_transfer_history(transfer_type, start_time, None, Some(page), Some(PAGE_SIZE))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    report.errors.push(format!(
                        "Failed to fetch transfers (type {}): {}",
                        transfer_type, e
                    ));
                    break;
                }
            };

            let fetched = response.rows.len();

            for transfer in &response.rows {
                match self
                    .importer
                    .import_transfer(account_id, transfer, dry_run)
                    .await
                {
                    Ok(ImportResult::Created(_)) => report.transfers_created += 1,
                    Ok(ImportResult::Skipped(_)) => report.transfers_skipped += 1,
                    Err(e) => report.errors.push(format!(
                        "Error importing transfer #{}: {}",
                        transfer.tran_id, e
                    )),
                }
            }

            if fetched as i32 == PAGE_SIZE {
                page += 1;
            } else {
                break;
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_report_totals() {
        let mut report = SyncReport::default();
        report.trades_created = 5;
        report.deposits_created = 2;
        report.withdrawals_created = 1;
        report.fiat_orders_created = 3;
        report.transfers_created = 4;

        report.trades_skipped = 10;
        report.deposits_skipped = 1;

        assert_eq!(report.total_created(), 15);
        assert_eq!(report.total_skipped(), 11);
        assert!(!report.has_errors());
    }

    #[test]
    fn test_sync_report_has_errors() {
        let mut report = SyncReport::default();
        assert!(!report.has_errors());
        report.errors.push("something failed".to_string());
        assert!(report.has_errors());
    }

    #[test]
    fn test_sync_options_default() {
        let opts = SyncOptions::default();
        assert!(opts.sync_trades);
        assert!(opts.sync_deposits);
        assert!(opts.sync_withdrawals);
        assert!(opts.sync_fiat);
        assert!(opts.sync_transfers);
        assert!(!opts.full_history);
        assert!(!opts.dry_run);
        assert!(opts.symbols.is_empty());
        // Transfer types are empty by default (problematic types disabled)
        assert!(opts.transfer_types.is_empty());
    }

    #[test]
    fn test_sync_options_full_history() {
        let opts = SyncOptions {
            full_history: true,
            symbols: vec!["BTCUSDT".to_string()],
            ..Default::default()
        };
        assert!(opts.full_history);
        assert_eq!(opts.symbols.len(), 1);
    }
}
