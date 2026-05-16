/// SyncEngine — parallel blockchain wallet sync with progress bars and watermarks.
///
/// Spawns one `tokio` task per address via `JoinSet`. A single address failure
/// does not abort the others. Block-height watermarks are persisted in the
/// `wallet_sync_state` table so incremental syncs only fetch new transactions.
use std::sync::Arc;
use std::time::Instant;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sqlx::SqlitePool;
use tokio::task::JoinSet;

use crate::blockchain::provider::ProviderRegistry;
use crate::blockchain::types::{Chain, WalletTransaction};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Options controlling a sync run.
#[derive(Debug, Clone, Default)]
pub struct SyncOptions {
    /// Only fetch transactions at or above this block height.
    /// Derived from `wallet_sync_state` on incremental syncs.
    pub since_block: Option<u64>,
    /// If true, clear watermarks and re-fetch full history.
    pub full_history: bool,
    /// If true, do not write anything to the DB.
    pub dry_run: bool,
}

/// Per-address sync result.
#[derive(Debug)]
pub struct AddressSyncResult {
    pub address: String,
    pub chain: Chain,
    pub provider: String,
    pub balances_updated: usize,
    pub transactions_new: usize,
    pub highest_block: Option<u64>,
    pub duration_ms: u64,
}

/// Aggregate report from a full wallet sync.
#[derive(Debug, Default)]
pub struct SyncReport {
    pub addresses_synced: usize,
    pub balances_updated: usize,
    pub transactions_new: usize,
    pub errors: Vec<SyncError>,
    pub duration_ms: u64,
}

impl SyncReport {
    fn merge(&mut self, result: AddressSyncResult) {
        self.addresses_synced += 1;
        self.balances_updated += result.balances_updated;
        self.transactions_new += result.transactions_new;
    }
}

#[derive(Debug)]
pub struct SyncError {
    pub address: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// SyncEngine
// ---------------------------------------------------------------------------

pub struct SyncEngine {
    registry: Arc<ProviderRegistry>,
    pool: SqlitePool,
}

impl SyncEngine {
    pub fn new(registry: Arc<ProviderRegistry>, pool: SqlitePool) -> Self {
        Self { registry, pool }
    }

    /// Sync a list of addresses in parallel.
    ///
    /// Each address gets its own task and its own progress bar.
    /// Tasks are isolated — one failure does not cancel others.
    pub async fn sync_addresses(
        &self,
        account_id: &str,
        addresses: Vec<(String, Chain)>,
        opts: SyncOptions,
    ) -> SyncReport {
        let mp = MultiProgress::new();
        let spinner_style =
            ProgressStyle::with_template("{spinner:.cyan} [{elapsed_precise}] {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner());

        let mut set: JoinSet<std::result::Result<AddressSyncResult, SyncError>> = JoinSet::new();
        let start = Instant::now();

        for (address, chain) in addresses {
            let registry = Arc::clone(&self.registry);
            let pool = self.pool.clone();
            let opts = opts.clone();
            let account_id = account_id.to_string();

            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_message(format!(
                "Fetching {} {} ...",
                chain.native_asset(),
                redact_address(&address)
            ));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            set.spawn(async move {
                let result = sync_single_address(
                    registry,
                    pool,
                    account_id,
                    address.clone(),
                    chain.clone(),
                    opts,
                    pb.clone(),
                )
                .await;

                match result {
                    Ok(r) => {
                        pb.finish_with_message(format!(
                            "✓ {} {} — {} new txs",
                            chain.native_asset(),
                            redact_address(&address),
                            r.transactions_new
                        ));
                        Ok(r)
                    }
                    Err(e) => {
                        pb.finish_with_message(format!(
                            "✗ {} {} — {}",
                            chain.native_asset(),
                            redact_address(&address),
                            e.message
                        ));
                        Err(e)
                    }
                }
            });
        }

        let mut report = SyncReport::default();

        while let Some(task_result) = set.join_next().await {
            match task_result {
                Ok(Ok(r)) => report.merge(r),
                Ok(Err(e)) => report.errors.push(e),
                Err(join_err) => report.errors.push(SyncError {
                    address: "unknown".to_string(),
                    message: format!("Task panicked: {}", join_err),
                }),
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        report
    }
}

// ---------------------------------------------------------------------------
// Single-address sync (runs inside a JoinSet task)
// ---------------------------------------------------------------------------

async fn sync_single_address(
    registry: Arc<ProviderRegistry>,
    pool: SqlitePool,
    account_id: String,
    address: String,
    chain: Chain,
    opts: SyncOptions,
    pb: ProgressBar,
) -> std::result::Result<AddressSyncResult, SyncError> {
    let task_start = Instant::now();

    // Resolve provider
    let client = registry.get(&chain).await.map_err(|e| SyncError {
        address: address.clone(),
        message: e.to_string(),
    })?;

    let provider_name = client.provider_name().to_string();

    // Audit log — record sync start
    write_audit_log(
        &pool,
        &account_id,
        &address,
        &chain,
        &provider_name,
        "sync_start",
        None,
        None,
        None,
        0,
    )
    .await;

    // Determine since_block from watermark (unless full_history requested)
    let since_block = if opts.full_history {
        // Clear existing watermark
        sqlx::query("DELETE FROM wallet_sync_state WHERE address = ? AND chain = ?")
            .bind(&address)
            .bind(chain.as_str())
            .execute(&pool)
            .await
            .ok();
        None
    } else if let Some(explicit) = opts.since_block {
        Some(explicit)
    } else {
        load_watermark(&pool, &address, &chain).await
    };

    pb.set_message(format!(
        "Syncing {} {} (since block {:?}) ...",
        chain.native_asset(),
        redact_address(&address),
        since_block
    ));

    // Fetch address summary (balance)
    let summary = client.get_address_summary(&address).await.map_err(|e| {
        let msg = format!("get_address_summary failed: {}", e);
        SyncError {
            address: address.clone(),
            message: msg,
        }
    })?;

    // Persist balances
    let balances_updated = if !opts.dry_run {
        persist_balances(&pool, &account_id, &summary.balances).await
    } else {
        summary.balances.len()
    };

    pb.set_message(format!(
        "Fetching {} transactions ...",
        chain.native_asset()
    ));

    // Fetch transactions
    let txs = client
        .get_transactions(&address, since_block)
        .await
        .map_err(|e| {
            let msg = format!("get_transactions failed: {}", e);
            SyncError {
                address: address.clone(),
                message: msg,
            }
        })?;

    let highest_block = txs.iter().filter_map(|tx| tx.block_height).max();

    // Persist transactions
    let transactions_new = if !opts.dry_run {
        persist_transactions(&pool, &account_id, &address, &chain, &txs).await
    } else {
        txs.len()
    };

    // Update watermark
    if !opts.dry_run {
        if let Some(block) = highest_block {
            save_watermark(&pool, &address, &chain, block).await;
        }
    }

    let duration_ms = task_start.elapsed().as_millis() as u64;

    // Audit log — record sync completion
    if !opts.dry_run {
        write_audit_log(
            &pool,
            &account_id,
            &address,
            &chain,
            &provider_name,
            "sync_complete",
            Some(txs.len() as i64),
            Some(transactions_new as i64),
            None,
            duration_ms,
        )
        .await;
    }

    Ok(AddressSyncResult {
        address,
        chain,
        provider: provider_name,
        balances_updated,
        transactions_new,
        highest_block,
        duration_ms,
    })
}

// ---------------------------------------------------------------------------
// Watermark helpers
// ---------------------------------------------------------------------------

async fn load_watermark(pool: &SqlitePool, address: &str, chain: &Chain) -> Option<u64> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT last_block FROM wallet_sync_state WHERE address = ? AND chain = ?")
            .bind(address)
            .bind(chain.as_str())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    row.map(|(b,)| (b + 1) as u64) // start from next block
}

async fn save_watermark(pool: &SqlitePool, address: &str, chain: &Chain, block: u64) {
    sqlx::query(
        "INSERT INTO wallet_sync_state (address, chain, last_block, last_sync_at, updated_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
         ON CONFLICT(address) DO UPDATE SET
             last_block   = excluded.last_block,
             last_sync_at = excluded.last_sync_at,
             updated_at   = excluded.updated_at",
    )
    .bind(address)
    .bind(chain.as_str())
    .bind(block as i64)
    .execute(pool)
    .await
    .ok();
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

async fn persist_balances(
    pool: &SqlitePool,
    account_id: &str,
    balances: &[crate::blockchain::types::WalletBalance],
) -> usize {
    let mut updated = 0;
    for balance in balances {
        let result = sqlx::query(
            "INSERT INTO holdings (account_id, asset, quantity, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(account_id, asset) DO UPDATE SET
                 quantity   = excluded.quantity,
                 updated_at = excluded.updated_at",
        )
        .bind(account_id)
        .bind(&balance.asset)
        .bind(balance.quantity.to_string())
        .execute(pool)
        .await;

        if result.is_ok() {
            updated += 1;
        }
    }
    updated
}

async fn persist_transactions(
    pool: &SqlitePool,
    account_id: &str,
    address: &str,
    chain: &Chain,
    txs: &[WalletTransaction],
) -> usize {
    use crate::blockchain::types::TransactionDirection;

    let mut new_count = 0;
    for tx in txs {
        let external_id = format!("{}-{}", chain.as_str(), tx.external_id);

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM transactions WHERE external_id = ?)")
                .bind(&external_id)
                .fetch_one(pool)
                .await
                .unwrap_or(true); // on error, skip to avoid duplicates

        if exists {
            continue;
        }

        // Use canonical TransactionType strings and correct asset/quantity columns.
        // Incoming:  asset arrives in account → to_asset / to_quantity
        // Outgoing:  asset leaves account     → from_asset / from_quantity
        // Internal:  stays within account     → from_asset / from_quantity
        let (tx_type, from_id, to_id, from_asset, from_qty, to_asset, to_qty) =
            match tx.direction {
                TransactionDirection::Incoming => (
                    "receive",
                    None::<&str>,
                    Some(account_id),
                    None::<&str>,
                    None::<String>,
                    Some(tx.asset.as_str()),
                    Some(tx.amount.to_string()),
                ),
                TransactionDirection::Outgoing => (
                    "transfer_out",
                    Some(account_id),
                    None::<&str>,
                    Some(tx.asset.as_str()),
                    Some(tx.amount.to_string()),
                    None::<&str>,
                    None::<String>,
                ),
                TransactionDirection::Internal => (
                    "transfer_internal",
                    Some(account_id),
                    Some(account_id),
                    Some(tx.asset.as_str()),
                    Some(tx.amount.to_string()),
                    None::<&str>,
                    None::<String>,
                ),
            };

        sqlx::query(
            "INSERT INTO transactions
             (tx_type, from_account_id, to_account_id,
              from_asset, from_quantity, to_asset, to_quantity,
              fee, fee_asset, external_id, notes, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tx_type)
        .bind(from_id)
        .bind(to_id)
        .bind(from_asset)
        .bind(from_qty)
        .bind(to_asset)
        .bind(to_qty)
        .bind(tx.fee.map(|f| f.to_string()))
        .bind(tx.fee_asset.as_deref())
        .bind(&external_id)
        .bind(address)
        .bind(tx.timestamp.to_rfc3339())
        .execute(pool)
        .await
        .ok();

        new_count += 1;
    }
    new_count
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

/// Write one row to `sync_audit_log`. Fire-and-forget — failures are silently
/// ignored so a log write never aborts a sync task.
#[allow(clippy::too_many_arguments)]
async fn write_audit_log(
    pool: &SqlitePool,
    account_id: &str,
    address: &str,
    chain: &Chain,
    provider: &str,
    action: &str,
    records_in: Option<i64>,
    records_new: Option<i64>,
    error: Option<&str>,
    duration_ms: u64,
) {
    sqlx::query(
        "INSERT INTO sync_audit_log
         (account_id, address, chain, provider, action,
          records_in, records_new, error, duration_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(account_id)
    .bind(address)
    .bind(chain.as_str())
    .bind(provider)
    .bind(action)
    .bind(records_in)
    .bind(records_new)
    .bind(error)
    .bind(duration_ms as i64)
    .execute(pool)
    .await
    .ok();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Redact an address for display: show first 8 + last 4 chars.
fn redact_address(address: &str) -> String {
    if address.len() <= 12 {
        return address.to_string();
    }
    format!("{}...{}", &address[..8], &address[address.len() - 4..])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_address_long() {
        let addr = "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf";
        let redacted = redact_address(addr);
        assert!(redacted.starts_with("1A1zP1eP"));
        assert!(redacted.ends_with("Divf"));
        assert!(redacted.contains("..."));
    }

    #[test]
    fn test_redact_address_short() {
        let addr = "short";
        assert_eq!(redact_address(addr), "short");
    }

    #[test]
    fn test_sync_report_merge() {
        let mut report = SyncReport::default();
        report.merge(AddressSyncResult {
            address: "addr1".to_string(),
            chain: Chain::Bitcoin,
            provider: "Blockstream".to_string(),
            balances_updated: 1,
            transactions_new: 5,
            highest_block: Some(800_000),
            duration_ms: 120,
        });
        report.merge(AddressSyncResult {
            address: "addr2".to_string(),
            chain: Chain::Ethereum,
            provider: "Etherscan".to_string(),
            balances_updated: 3,
            transactions_new: 10,
            highest_block: Some(19_000_000),
            duration_ms: 200,
        });
        assert_eq!(report.addresses_synced, 2);
        assert_eq!(report.balances_updated, 4);
        assert_eq!(report.transactions_new, 15);
    }

    #[test]
    fn test_sync_options_default() {
        let opts = SyncOptions::default();
        assert!(!opts.full_history);
        assert!(!opts.dry_run);
        assert!(opts.since_block.is_none());
    }
}
