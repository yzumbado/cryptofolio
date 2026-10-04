/// SyncEngine — parallel blockchain wallet sync with progress bars and watermarks.
///
/// Spawns one `tokio` task per address via `JoinSet`. A single address failure
/// does not abort the others. Block-height watermarks are persisted in the
/// `wallet_sync_state` table so incremental syncs only fetch new transactions.
use std::sync::Arc;
use std::time::Instant;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sqlx::{Sqlite, SqlitePool};
use tokio::task::JoinSet;

use crate::blockchain::provider::ProviderRegistry;
use crate::blockchain::types::{Chain, DatedReward, WalletTransaction};

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
    /// New dated reward rows booked as mining income.
    pub reward_rows_new: usize,
    /// Non-fatal per-address warnings (e.g. a reward row that failed to decode).
    pub warnings: Vec<String>,
    pub highest_block: Option<u64>,
    pub duration_ms: u64,
}

/// Aggregate report from a full wallet sync.
#[derive(Debug, Default)]
pub struct SyncReport {
    pub addresses_synced: usize,
    pub balances_updated: usize,
    pub transactions_new: usize,
    /// New dated reward (mining income) rows across all addresses.
    pub reward_rows_new: usize,
    pub errors: Vec<SyncError>,
    /// Non-fatal warnings: rows skipped with a reason, never silent drops.
    pub warnings: Vec<SyncError>,
    pub duration_ms: u64,
}

impl SyncReport {
    fn merge(&mut self, result: AddressSyncResult) {
        self.addresses_synced += 1;
        self.balances_updated += result.balances_updated;
        self.transactions_new += result.transactions_new;
        self.reward_rows_new += result.reward_rows_new;
        let address = result.address.clone();
        self.warnings
            .extend(result.warnings.into_iter().map(|message| SyncError {
                address: address.clone(),
                message,
            }));
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
            .map_err(|e| SyncError {
                address: address.clone(),
                message: format!("clear watermark failed: {}", e),
            })?;
        None
    } else if let Some(explicit) = opts.since_block {
        Some(explicit)
    } else {
        load_watermark(&pool, &address, &chain)
            .await
            .map_err(|e| SyncError {
                address: address.clone(),
                message: format!("load_watermark failed: {}", e),
            })?
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

    // Dated on-chain reward income (DePIN mined tokens). Generic across chains:
    // a client with no dated reward stream returns an empty batch. RPC failure
    // is an address-level SyncError (existing convention); an individual
    // undecodable reward row is a warning and does not drop the other rows.
    pb.set_message(format!(
        "Fetching {} reward history ...",
        chain.native_asset()
    ));

    let mut warnings: Vec<String> = Vec::new();

    let batch = client
        .get_dated_rewards(&address, since_block)
        .await
        .map_err(|e| SyncError {
            address: address.clone(),
            message: format!("get_dated_rewards failed: {}", e),
        })?;

    warnings.extend(
        batch
            .skipped
            .iter()
            .map(|skip| format!("reward {} skipped: {}", skip.signature, skip.reason)),
    );

    // Persist balances, transactions, reward rows, and the watermark in ONE
    // transaction: a mid-batch failure rolls back the whole address instead of
    // leaving earlier rows committed while the address reports an error.
    let (balances_updated, transactions_new, reward_rows_new) = if !opts.dry_run {
        let mut tx = pool.begin().await.map_err(|e| SyncError {
            address: address.clone(),
            message: format!("begin sync transaction failed: {}", e),
        })?;

        let bu = persist_balances(&mut tx, &account_id, &summary.balances)
            .await
            .map_err(|e| SyncError {
                address: address.clone(),
                message: format!("persist_balances failed: {}", e),
            })?;
        let tn = persist_transactions(&mut tx, &account_id, &address, &chain, &txs)
            .await
            .map_err(|e| SyncError {
                address: address.clone(),
                message: format!("persist_transactions failed: {}", e),
            })?;
        let rn = persist_rewards(&mut tx, &account_id, &chain, &batch.rewards)
            .await
            .map_err(|e| SyncError {
                address: address.clone(),
                message: format!("persist_rewards failed: {}", e),
            })?;

        if let Some(block) = highest_block {
            save_watermark(&mut tx, &address, &chain, block)
                .await
                .map_err(|e| SyncError {
                    address: address.clone(),
                    message: format!("save_watermark failed: {}", e),
                })?;
        }

        tx.commit().await.map_err(|e| SyncError {
            address: address.clone(),
            message: format!("commit sync transaction failed: {}", e),
        })?;

        (bu, tn, rn)
    } else {
        // Dry run still reports what would be booked, but writes nothing.
        (summary.balances.len(), txs.len(), batch.rewards.len())
    };

    let duration_ms = task_start.elapsed().as_millis() as u64;

    // Audit log — record sync completion. Per-row reward skips are surfaced in
    // the report's warnings; log the count of new reward rows as records_new.
    if !opts.dry_run {
        write_audit_log(
            &pool,
            &account_id,
            &address,
            &chain,
            &provider_name,
            "sync_complete",
            Some((txs.len() + reward_rows_new) as i64),
            Some((transactions_new + reward_rows_new) as i64),
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
        reward_rows_new,
        warnings,
        highest_block,
        duration_ms,
    })
}

// ---------------------------------------------------------------------------
// Watermark helpers
// ---------------------------------------------------------------------------

async fn load_watermark(
    pool: &SqlitePool,
    address: &str,
    chain: &Chain,
) -> crate::error::Result<Option<u64>> {
    let row: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT last_block FROM wallet_sync_state WHERE address = ? AND chain = ?")
            .bind(address)
            .bind(chain.as_str())
            .fetch_optional(pool)
            .await?;

    // No row, or a NULL watermark, means full history. Otherwise start from
    // the block after the last one we synced.
    Ok(row.and_then(|(b,)| b.map(|b| (b + 1) as u64)))
}

async fn save_watermark(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    address: &str,
    chain: &Chain,
    block: u64,
) -> crate::error::Result<()> {
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
    .execute(&mut **tx)
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

async fn persist_balances(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    account_id: &str,
    balances: &[crate::blockchain::types::WalletBalance],
) -> crate::error::Result<usize> {
    let mut updated = 0;
    for balance in balances {
        sqlx::query(
            "INSERT INTO holdings (account_id, asset, quantity, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(account_id, asset) DO UPDATE SET
                 quantity   = excluded.quantity,
                 updated_at = excluded.updated_at",
        )
        .bind(account_id)
        .bind(&balance.asset)
        .bind(balance.quantity.to_string())
        .execute(&mut **tx)
        .await?;

        updated += 1;
    }
    Ok(updated)
}

async fn persist_transactions(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    account_id: &str,
    address: &str,
    chain: &Chain,
    txs: &[WalletTransaction],
) -> crate::error::Result<usize> {
    use crate::blockchain::types::TransactionDirection;

    let mut new_count = 0;
    for event in txs {
        let external_id = format!("{}-{}", chain.as_str(), event.external_id);

        // Use canonical TransactionType strings and correct asset/quantity columns.
        // Incoming:  asset arrives in account → to_asset / to_quantity
        // Outgoing:  asset leaves account     → from_asset / from_quantity
        // Internal:  stays within account     → from_asset / from_quantity
        let (tx_type, from_id, to_id, from_asset, from_qty, to_asset, to_qty) =
            match event.direction {
                TransactionDirection::Incoming => (
                    "receive",
                    None::<&str>,
                    Some(account_id),
                    None::<&str>,
                    None::<String>,
                    Some(event.asset.as_str()),
                    Some(event.amount.to_string()),
                ),
                TransactionDirection::Outgoing => (
                    "transfer_out",
                    Some(account_id),
                    None::<&str>,
                    Some(event.asset.as_str()),
                    Some(event.amount.to_string()),
                    None::<&str>,
                    None::<String>,
                ),
                TransactionDirection::Internal => (
                    "transfer_internal",
                    Some(account_id),
                    Some(account_id),
                    Some(event.asset.as_str()),
                    Some(event.amount.to_string()),
                    None::<&str>,
                    None::<String>,
                ),
            };

        let result = sqlx::query(
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
        .bind(event.fee.map(|f| f.to_string()))
        .bind(event.fee_asset.as_deref())
        .bind(&external_id)
        .bind(address)
        .bind(event.timestamp.to_rfc3339())
        .execute(&mut **tx)
        .await;

        match result {
            Ok(_) => new_count += 1,
            // Dedup lives in the UNIQUE(external_id) constraint: a unique
            // violation means this chain event is already in the ledger.
            Err(e)
                if e.as_database_error()
                    .is_some_and(|db| db.is_unique_violation()) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(new_count)
}

/// Persist dated reward credits as ledger income rows.
///
/// Each row is the mining-income shape from `docs/MINING_ASSET_ACCOUNTING.md`:
/// a `receive` of the token into the wallet account at the block time, marked
/// `chain_verified`, with `external_id = {chain}-{signature}` so the UNIQUE
/// constraint dedups re-syncs. Source is `helius` — the Solana on-chain
/// provenance value this schema allows.
async fn persist_rewards(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    account_id: &str,
    chain: &Chain,
    rewards: &[DatedReward],
) -> crate::error::Result<usize> {
    let mut new_count = 0;
    for reward in rewards {
        let external_id = format!("{}-{}", chain.as_str(), reward.signature);
        let notes = format!("MINING REWARD: {}", external_id);

        let result = sqlx::query(
            "INSERT INTO transactions
             (tx_type, to_account_id, to_asset, to_quantity,
              external_id, source, trust_level, notes, timestamp)
             VALUES ('receive', ?, ?, ?, ?, 'helius', 'chain_verified', ?, ?)",
        )
        .bind(account_id)
        .bind(&reward.asset)
        .bind(reward.quantity.to_string())
        .bind(&external_id)
        .bind(&notes)
        .bind(reward.date.to_rfc3339())
        .execute(&mut **tx)
        .await;

        match result {
            Ok(_) => new_count += 1,
            Err(e)
                if e.as_database_error()
                    .is_some_and(|db| db.is_unique_violation()) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(new_count)
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
    use crate::blockchain::types::{TransactionDirection, WalletBalance};
    use crate::db::init_memory_pool;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    const ACCOUNT_ID: &str = "acc-1";

    async fn setup_db() -> SqlitePool {
        let pool = init_memory_pool().await.expect("in-memory pool");
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test-cat', 'Test')")
            .execute(&pool)
            .await
            .expect("insert category");
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) \
             VALUES (?, 'test-cat', 'Test Wallet', 'wallet')",
        )
        .bind(ACCOUNT_ID)
        .execute(&pool)
        .await
        .expect("insert account");
        pool
    }

    fn sample_tx(
        external_id: &str,
        direction: TransactionDirection,
        amount: &str,
    ) -> WalletTransaction {
        WalletTransaction {
            external_id: external_id.to_string(),
            direction,
            amount: Decimal::from_str(amount).expect("decimal"),
            asset: "BTC".to_string(),
            fee: None,
            fee_asset: None,
            block_height: Some(800_000),
            timestamp: Utc::now(),
            counterparty: None,
            memo: None,
        }
    }

    fn sample_balance(asset: &str, quantity: &str) -> WalletBalance {
        WalletBalance {
            asset: asset.to_string(),
            asset_id: None,
            quantity: Decimal::from_str(quantity).expect("decimal"),
            decimals: 8,
        }
    }

    fn sample_reward(signature: &str, quantity: &str) -> DatedReward {
        DatedReward {
            date: chrono::DateTime::from_timestamp(1_728_000_000, 0).expect("fixture block time"),
            asset: "GEOD".to_string(),
            quantity: Decimal::from_str(quantity).expect("decimal"),
            signature: signature.to_string(),
        }
    }

    #[tokio::test]
    async fn test_persist_transactions_dedups_on_unique_violation() {
        let pool = setup_db().await;
        let chain = Chain::Bitcoin;
        let address = "bc1qtestaddress";

        let txs = vec![
            sample_tx("tx-a", TransactionDirection::Incoming, "0.5"),
            sample_tx("tx-b", TransactionDirection::Outgoing, "0.25"),
        ];

        let mut tx = pool.begin().await.expect("begin tx");
        let first = persist_transactions(&mut tx, ACCOUNT_ID, address, &chain, &txs)
            .await
            .expect("first persist");
        assert_eq!(first, 2);

        // Re-persisting the same chain events must dedup via the UNIQUE
        // (external_id) constraint and report zero new rows.
        let second = persist_transactions(&mut tx, ACCOUNT_ID, address, &chain, &txs)
            .await
            .expect("second persist");
        assert_eq!(second, 0);

        // A mixed batch keeps the genuinely new rows and skips the duplicates.
        let mixed = vec![
            sample_tx("tx-a", TransactionDirection::Incoming, "0.5"),
            sample_tx("tx-c", TransactionDirection::Incoming, "0.75"),
        ];
        let third = persist_transactions(&mut tx, ACCOUNT_ID, address, &chain, &mixed)
            .await
            .expect("third persist");
        assert_eq!(third, 1);
        tx.commit().await.expect("commit tx");

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .expect("count transactions");
        assert_eq!(rows, 3);
    }

    #[tokio::test]
    async fn test_persist_transactions_propagates_non_unique_errors() {
        let pool = setup_db().await;
        let txs = vec![sample_tx("tx-fk", TransactionDirection::Incoming, "1")];

        // Unknown account → foreign key violation. It is not a duplicate, so it
        // must surface instead of being swallowed and counted as a new row.
        let mut tx = pool.begin().await.expect("begin tx");
        let outcome =
            persist_transactions(&mut tx, "missing-account", "addr", &Chain::Bitcoin, &txs).await;
        assert!(
            outcome.is_err(),
            "expected FK violation to propagate, got {outcome:?}"
        );
        drop(tx); // rollback + release the single pooled connection

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .expect("count transactions");
        assert_eq!(rows, 0);
    }

    #[tokio::test]
    async fn test_persist_rewards_dedups_and_tags_income_rows() {
        let pool = setup_db().await;
        let rewards = vec![sample_reward("sig-a", "12.5"), sample_reward("sig-b", "3")];

        let mut tx = pool.begin().await.expect("begin tx");
        let first = persist_rewards(&mut tx, ACCOUNT_ID, &Chain::Solana, &rewards)
            .await
            .expect("first persist");
        assert_eq!(first, 2);

        // Re-syncing the same signatures must dedup via UNIQUE(external_id).
        let second = persist_rewards(&mut tx, ACCOUNT_ID, &Chain::Solana, &rewards)
            .await
            .expect("second persist");
        assert_eq!(second, 0);
        tx.commit().await.expect("commit tx");

        // The rows are dated `receive` income, chain-verified, marked as mining.
        let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
            "SELECT tx_type, to_asset, to_quantity, external_id, trust_level, notes
             FROM transactions ORDER BY id ASC",
        )
        .fetch_all(&pool)
        .await
        .expect("select reward rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "receive");
        assert_eq!(rows[0].1, "GEOD");
        assert_eq!(rows[0].2, "12.5");
        assert_eq!(rows[0].3, "solana-sig-a");
        assert_eq!(rows[0].4, "chain_verified");
        assert!(rows[0].5.starts_with("MINING REWARD"));

        // `timestamp` is the block time, not sync time. 1_728_000_000 is
        // 2024-10-04T00:00:00Z — an exact, deterministic fixture value.
        let expected_ts = chrono::DateTime::from_timestamp(1_728_000_000, 0)
            .expect("valid fixture block time")
            .to_rfc3339();
        let ts: String = sqlx::query_scalar(
            "SELECT timestamp FROM transactions WHERE external_id = 'solana-sig-a'",
        )
        .fetch_one(&pool)
        .await
        .expect("timestamp");
        assert_eq!(ts, expected_ts);

        // Unknown account → FK violation, not a duplicate; must propagate.
        // Use an unseen signature so the UNIQUE dedup cannot mask the FK error.
        let fk_rewards = vec![sample_reward("sig-fk", "1")];
        let mut tx = pool.begin().await.expect("begin tx");
        let outcome =
            persist_rewards(&mut tx, "missing-account", &Chain::Solana, &fk_rewards).await;
        assert!(
            outcome.is_err(),
            "expected FK violation to propagate, got {outcome:?}"
        );
        drop(tx);
    }

    #[tokio::test]
    async fn test_persist_balances_upserts_and_propagates_errors() {
        let pool = setup_db().await;
        let balances = vec![sample_balance("BTC", "0.5"), sample_balance("USDC", "10")];

        let mut tx = pool.begin().await.expect("begin tx");
        let first = persist_balances(&mut tx, ACCOUNT_ID, &balances)
            .await
            .expect("first persist");
        assert_eq!(first, 2);

        // Upsert in place, not a second pair of rows.
        let second = persist_balances(&mut tx, ACCOUNT_ID, &balances)
            .await
            .expect("second persist");
        assert_eq!(second, 2);
        tx.commit().await.expect("commit tx");

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM holdings WHERE account_id = ?")
            .bind(ACCOUNT_ID)
            .fetch_one(&pool)
            .await
            .expect("count holdings");
        assert_eq!(rows, 2);

        // Unknown account → foreign key violation must propagate.
        let mut tx = pool.begin().await.expect("begin tx");
        let outcome = persist_balances(&mut tx, "missing-account", &balances).await;
        assert!(
            outcome.is_err(),
            "expected FK violation to propagate, got {outcome:?}"
        );
        drop(tx);
    }

    #[tokio::test]
    async fn test_watermark_roundtrip() {
        let pool = setup_db().await;
        let chain = Chain::Ethereum;
        let address = "0xtestaddress";

        // No watermark yet → full history.
        assert_eq!(
            load_watermark(&pool, address, &chain)
                .await
                .expect("load empty"),
            None
        );

        let mut tx = pool.begin().await.expect("begin tx");
        save_watermark(&mut tx, address, &chain, 19_000_000)
            .await
            .expect("save watermark");
        tx.commit().await.expect("commit tx");
        // Resume from the block after the last synced one.
        assert_eq!(
            load_watermark(&pool, address, &chain)
                .await
                .expect("load watermark"),
            Some(19_000_001)
        );

        let mut tx = pool.begin().await.expect("begin tx");
        save_watermark(&mut tx, address, &chain, 19_000_050)
            .await
            .expect("overwrite watermark");
        tx.commit().await.expect("commit tx");
        assert_eq!(
            load_watermark(&pool, address, &chain)
                .await
                .expect("load overwritten"),
            Some(19_000_051)
        );

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_sync_state")
            .fetch_one(&pool)
            .await
            .expect("count watermarks");
        assert_eq!(rows, 1);
    }

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
            reward_rows_new: 2,
            warnings: vec!["reward sig-a skipped: malformed".to_string()],
            highest_block: Some(800_000),
            duration_ms: 120,
        });
        report.merge(AddressSyncResult {
            address: "addr2".to_string(),
            chain: Chain::Ethereum,
            provider: "Etherscan".to_string(),
            balances_updated: 3,
            transactions_new: 10,
            reward_rows_new: 0,
            warnings: vec![],
            highest_block: Some(19_000_000),
            duration_ms: 200,
        });
        assert_eq!(report.addresses_synced, 2);
        assert_eq!(report.balances_updated, 4);
        assert_eq!(report.transactions_new, 15);
        assert_eq!(report.reward_rows_new, 2);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].address, "addr1");
    }

    #[test]
    fn test_sync_options_default() {
        let opts = SyncOptions::default();
        assert!(!opts.full_history);
        assert!(!opts.dry_run);
        assert!(opts.since_block.is_none());
    }
}
