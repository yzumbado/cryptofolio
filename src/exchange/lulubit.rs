//! Lulubit statement importer — fiat on/off-ramp (USD) and direct USDT receipts.
//!
//! Ingests a small CSV of dated operations and records them as the same
//! buy/swap/sell transactions the interactive `tx` command would, so cost basis,
//! holdings, and P&L stay correct. Rows are deduped by a stable `external_id`
//! (the UNIQUE constraint skips re-imports).
//!
//! `source` is `manual` (the schema's CHECK has no `lulubit` value; statements
//! are manual entry) and `trust_level` is `manual`. Notes carry the operation
//! so the fiat boundary is queryable.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::core::pnl::{CostBasisMethod, PnLCalculator};
use crate::core::transaction::{Transaction, TransactionType};
use crate::db::{HoldingRepository, TransactionRepository};
use crate::error::{CryptofolioError, Result};

/// One Lulubit statement operation, reduced to the three ledger primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum LulubitOp {
    /// Receive `asset` at `price_usd` per unit (fiat deposit or loan repayment).
    Receive {
        asset: String,
        quantity: Decimal,
        price_usd: Decimal,
        date: DateTime<Utc>,
    },
    /// Swap `from_asset` -> `to_asset`.
    Swap {
        from_asset: String,
        from_quantity: Decimal,
        to_asset: String,
        to_quantity: Decimal,
        date: DateTime<Utc>,
    },
    /// Dispose `asset` at `price_usd` per unit (fiat withdrawal or fee).
    Dispose {
        asset: String,
        quantity: Decimal,
        price_usd: Decimal,
        date: DateTime<Utc>,
    },
    /// Move `asset` between two accounts (e.g. Lulubit -> Binance).
    Transfer {
        from_account: String,
        to_account: String,
        asset: String,
        quantity: Decimal,
        date: DateTime<Utc>,
    },
    /// Send `asset` out of the tracked system (external withdrawal, no tracked
    /// destination). The destination's on-chain receipt may already be in the
    /// ledger via blockchain sync, so this only records the source side.
    Send {
        asset: String,
        quantity: Decimal,
        date: DateTime<Utc>,
    },
    /// Record a tx-sum-only correction (no holdings update). Used to reconcile a
    /// stale import whose transaction history no longer matches the raw source.
    /// A `from_*` leg reduces the computed balance; a `to_*` leg increases it.
    Correction {
        from_asset: Option<String>,
        from_quantity: Option<Decimal>,
        to_asset: Option<String>,
        to_quantity: Option<Decimal>,
        date: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportResult {
    /// New transaction created with this id.
    Created(i64),
    /// Duplicate `external_id` — skipped.
    Skipped,
}

pub struct LulubitImporter<'a> {
    tx_repo: TransactionRepository<'a>,
    holding_repo: HoldingRepository<'a>,
    pnl_calc: PnLCalculator<'a>,
}

impl<'a> LulubitImporter<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            tx_repo: TransactionRepository::new(pool),
            holding_repo: HoldingRepository::new(pool),
            pnl_calc: PnLCalculator::new(pool),
        }
    }

    /// Fail fast if `account_id` lacks `quantity` of `asset`, BEFORE any
    /// transaction is written. Without this, an underfunded row would insert the
    /// transaction and then fail on the holdings update, leaving an orphaned
    /// (tx-without-holdings) record — the import must be all-or-nothing.
    async fn ensure_balance(&self, account_id: &str, asset: &str, quantity: Decimal) -> Result<()> {
        match self.holding_repo.get(account_id, asset).await? {
            Some(h) if h.quantity >= quantity => Ok(()),
            Some(h) => Err(CryptofolioError::InsufficientBalance {
                available: h.quantity.to_string(),
                required: quantity.to_string(),
            }),
            None => Err(CryptofolioError::AssetNotFound(asset.to_string())),
        }
    }

    /// Record one operation against `account_id`, deduped by `external_id`.
    ///
    /// Returns `Skipped` if a transaction with that `external_id` already exists
    /// (UNIQUE constraint), so re-running an import is idempotent.
    pub async fn import(
        &self,
        account_id: &str,
        external_id: &str,
        note: Option<&str>,
        op: LulubitOp,
    ) -> Result<ImportResult> {
        match op {
            LulubitOp::Receive {
                asset,
                quantity,
                price_usd,
                date,
            } => {
                let mut tx = Transaction::new_buy(account_id, &asset, quantity, price_usd, date);
                tx.external_id = Some(external_id.to_string());
                tx.source = "manual".to_string();
                tx.trust_level = "manual".to_string();
                tx.notes = note.map(str::to_string);

                let tx_id = match self.tx_repo.insert(&tx).await {
                    Ok(id) => id,
                    Err(e) if is_unique_violation(&e) => return Ok(ImportResult::Skipped),
                    Err(e) => return Err(e),
                };

                self.holding_repo
                    .add_quantity(account_id, &asset, quantity, Some(price_usd))
                    .await?;
                let _ = self
                    .pnl_calc
                    .process_acquisition(
                        tx_id,
                        account_id,
                        &asset,
                        quantity,
                        price_usd,
                        date,
                        CostBasisMethod::Fifo,
                    )
                    .await;

                Ok(ImportResult::Created(tx_id))
            }

            LulubitOp::Dispose {
                asset,
                quantity,
                price_usd,
                date,
            } => {
                self.ensure_balance(account_id, &asset, quantity).await?;
                let mut tx = Transaction::new_sell(account_id, &asset, quantity, price_usd, date);
                tx.external_id = Some(external_id.to_string());
                tx.source = "manual".to_string();
                tx.trust_level = "manual".to_string();
                tx.notes = note.map(str::to_string);

                let tx_id = match self.tx_repo.insert(&tx).await {
                    Ok(id) => id,
                    Err(e) if is_unique_violation(&e) => return Ok(ImportResult::Skipped),
                    Err(e) => return Err(e),
                };

                self.holding_repo
                    .remove_quantity(account_id, &asset, quantity)
                    .await?;
                let _ = self
                    .pnl_calc
                    .process_disposal(
                        tx_id,
                        account_id,
                        &asset,
                        quantity,
                        price_usd,
                        date,
                        CostBasisMethod::Fifo,
                    )
                    .await;

                Ok(ImportResult::Created(tx_id))
            }

            LulubitOp::Swap {
                from_asset,
                from_quantity,
                to_asset,
                to_quantity,
                date,
            } => {
                self.ensure_balance(account_id, &from_asset, from_quantity)
                    .await?;
                // The from-asset leaves at its own average cost basis. Without a
                // market price for the source leg, realising zero gain/loss is
                // the correct fallback (previously the to-asset's per-unit cost
                // was reused as the source proceeds, inflating P&L ~thousands of
                // times on e.g. a USDT->ETH swap).
                let disposal_price = self
                    .holding_repo
                    .get(account_id, &from_asset)
                    .await?
                    .and_then(|h| h.avg_cost_basis);

                // The to-asset's cost per unit = the from-asset's total cost
                // spread over the amount acquired, so the on-ramp spread becomes
                // real cost basis on the destination leg.
                let acquisition_price =
                    disposal_price.map(|cost| cost * from_quantity / to_quantity);

                let mut tx = Transaction::new_swap(
                    account_id,
                    &from_asset,
                    from_quantity,
                    &to_asset,
                    to_quantity,
                    date,
                );
                tx.external_id = Some(external_id.to_string());
                tx.source = "manual".to_string();
                tx.trust_level = "manual".to_string();
                tx.notes = note.map(str::to_string);

                let tx_id = match self.tx_repo.insert(&tx).await {
                    Ok(id) => id,
                    Err(e) if is_unique_violation(&e) => return Ok(ImportResult::Skipped),
                    Err(e) => return Err(e),
                };

                if let Some(price) = disposal_price {
                    let _ = self
                        .pnl_calc
                        .process_disposal(
                            tx_id,
                            account_id,
                            &from_asset,
                            from_quantity,
                            price,
                            date,
                            CostBasisMethod::Fifo,
                        )
                        .await;
                }
                if let Some(price) = acquisition_price {
                    let _ = self
                        .pnl_calc
                        .process_acquisition(
                            tx_id,
                            account_id,
                            &to_asset,
                            to_quantity,
                            price,
                            date,
                            CostBasisMethod::Fifo,
                        )
                        .await;
                }

                self.holding_repo
                    .remove_quantity(account_id, &from_asset, from_quantity)
                    .await?;
                self.holding_repo
                    .add_quantity(account_id, &to_asset, to_quantity, acquisition_price)
                    .await?;

                Ok(ImportResult::Created(tx_id))
            }
            LulubitOp::Transfer {
                from_account,
                to_account,
                asset,
                quantity,
                date,
            } => {
                self.ensure_balance(&from_account, &asset, quantity).await?;
                let mut tx =
                    Transaction::new_transfer(&from_account, &to_account, &asset, quantity, date);
                tx.external_id = Some(external_id.to_string());
                tx.source = "manual".to_string();
                tx.trust_level = "manual".to_string();
                tx.notes = note.map(str::to_string);

                let tx_id = match self.tx_repo.insert(&tx).await {
                    Ok(id) => id,
                    Err(e) if is_unique_violation(&e) => return Ok(ImportResult::Skipped),
                    Err(e) => return Err(e),
                };

                // Carry the source account's cost basis into the destination.
                let basis = self
                    .holding_repo
                    .get(&from_account, &asset)
                    .await?
                    .and_then(|h| h.avg_cost_basis);

                self.holding_repo
                    .remove_quantity(&from_account, &asset, quantity)
                    .await?;
                self.holding_repo
                    .add_quantity(&to_account, &asset, quantity, basis)
                    .await?;

                Ok(ImportResult::Created(tx_id))
            }
            LulubitOp::Send {
                asset,
                quantity,
                date,
            } => {
                self.ensure_balance(account_id, &asset, quantity).await?;
                let tx = Transaction {
                    id: 0,
                    tx_type: TransactionType::TransferOut,
                    from_account_id: Some(account_id.to_string()),
                    from_asset: Some(asset.clone()),
                    from_quantity: Some(quantity),
                    to_account_id: None,
                    to_asset: None,
                    to_quantity: None,
                    price_usd: None,
                    price_currency: None,
                    price_amount: None,
                    exchange_rate: None,
                    exchange_rate_pair: None,
                    fee: None,
                    fee_asset: None,
                    tx_hash: None,
                    external_id: Some(external_id.to_string()),
                    source: "manual".to_string(),
                    trust_level: "manual".to_string(),
                    notes: note.map(str::to_string),
                    timestamp: date,
                    created_at: Utc::now(),
                };
                let tx_id = match self.tx_repo.insert(&tx).await {
                    Ok(id) => id,
                    Err(e) if is_unique_violation(&e) => return Ok(ImportResult::Skipped),
                    Err(e) => return Err(e),
                };
                self.holding_repo
                    .remove_quantity(account_id, &asset, quantity)
                    .await?;
                Ok(ImportResult::Created(tx_id))
            }
            LulubitOp::Correction {
                from_asset,
                from_quantity,
                to_asset,
                to_quantity,
                date,
            } => {
                // A correction adjusts the tx-sum only (so `audit reconciliation`
                // sees the right computed balance) and deliberately does NOT touch
                // holdings — the recorded balance is already correct.
                let tx = Transaction {
                    id: 0,
                    tx_type: TransactionType::Correction,
                    from_account_id: from_asset.as_ref().map(|_| account_id.to_string()),
                    from_asset,
                    from_quantity,
                    to_account_id: to_asset.as_ref().map(|_| account_id.to_string()),
                    to_asset,
                    to_quantity,
                    price_usd: None,
                    price_currency: None,
                    price_amount: None,
                    exchange_rate: None,
                    exchange_rate_pair: None,
                    fee: None,
                    fee_asset: None,
                    tx_hash: None,
                    external_id: Some(external_id.to_string()),
                    source: "manual".to_string(),
                    trust_level: "manual".to_string(),
                    notes: note.map(str::to_string),
                    timestamp: date,
                    created_at: Utc::now(),
                };
                let tx_id = match self.tx_repo.insert(&tx).await {
                    Ok(id) => id,
                    Err(e) if is_unique_violation(&e) => return Ok(ImportResult::Skipped),
                    Err(e) => return Err(e),
                };
                Ok(ImportResult::Created(tx_id))
            }
        }
    }
}

fn is_unique_violation(e: &crate::error::CryptofolioError) -> bool {
    // CryptofolioError wraps sqlx::Error via #[from]; reach the database error.
    if let crate::error::CryptofolioError::Database(err) = e {
        return err
            .as_database_error()
            .is_some_and(|db| db.is_unique_violation());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_pool;
    use chrono::TimeZone;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    async fn account(pool: &SqlitePool) -> String {
        // schema::create seeds the 'on-ramp' category already.
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) \
             VALUES ('lulubit', 'on-ramp', 'Lulubit', 'exchange')",
        )
        .execute(pool)
        .await
        .unwrap();
        "lulubit".to_string()
    }

    #[tokio::test]
    async fn on_ramp_deposit_and_swap_set_cost_basis() {
        let pool = init_memory_pool().await.unwrap();
        let acc = account(&pool).await;
        let importer = LulubitImporter::new(&pool);

        // $10,000 deposit -> 9,852.077922 USDT captures the ~1.5% spread.
        let r1 = importer
            .import(
                &acc,
                "lulubit-1",
                Some("fiat deposit"),
                LulubitOp::Receive {
                    asset: "USD".into(),
                    quantity: dec("10000"),
                    price_usd: dec("1"),
                    date: date(2026, 6, 1),
                },
            )
            .await
            .unwrap();
        assert!(matches!(r1, ImportResult::Created(_)));

        let r2 = importer
            .import(
                &acc,
                "lulubit-2",
                Some("on-ramp"),
                LulubitOp::Swap {
                    from_asset: "USD".into(),
                    from_quantity: dec("10000"),
                    to_asset: "USDT".into(),
                    to_quantity: dec("9852.077922"),
                    date: date(2026, 6, 2),
                },
            )
            .await
            .unwrap();
        assert!(matches!(r2, ImportResult::Created(_)));

        // USDT basis = 10000 / 9852.077922 ≈ 1.015014
        let usdt = importer
            .holding_repo
            .get(&acc, "USDT")
            .await
            .unwrap()
            .unwrap();
        assert!((usdt.avg_cost_basis.unwrap() - dec("1.015014")).abs() < dec("0.0001"));
    }

    #[tokio::test]
    async fn reimport_is_idempotent_via_external_id() {
        let pool = init_memory_pool().await.unwrap();
        let acc = account(&pool).await;
        let importer = LulubitImporter::new(&pool);

        let op = || LulubitOp::Receive {
            asset: "USDT".into(),
            quantity: dec("700"),
            price_usd: dec("1"),
            date: date(2026, 8, 15),
        };
        let first = importer
            .import(&acc, "lulubit-loan-700", Some("loan repayment"), op())
            .await
            .unwrap();
        let second = importer
            .import(&acc, "lulubit-loan-700", Some("loan repayment"), op())
            .await
            .unwrap();
        assert!(matches!(first, ImportResult::Created(_)));
        assert!(matches!(second, ImportResult::Skipped));
    }

    #[tokio::test]
    async fn transfer_moves_asset_and_carries_basis() {
        let pool = init_memory_pool().await.unwrap();
        let lulubit = account(&pool).await;
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) \
             VALUES ('binance', 'trading', 'Binance', 'exchange')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let importer = LulubitImporter::new(&pool);

        let _ = importer
            .import(
                &lulubit,
                "lulubit-loan-1",
                Some("loan"),
                LulubitOp::Receive {
                    asset: "USDT".into(),
                    quantity: dec("1000"),
                    price_usd: dec("1"),
                    date: date(2026, 1, 1),
                },
            )
            .await
            .unwrap();

        let r = importer
            .import(
                &lulubit,
                "lulubit-xfer-1",
                Some("to Binance"),
                LulubitOp::Transfer {
                    from_account: lulubit.clone(),
                    to_account: "binance".into(),
                    asset: "USDT".into(),
                    quantity: dec("400"),
                    date: date(2026, 1, 2),
                },
            )
            .await
            .unwrap();
        assert!(matches!(r, ImportResult::Created(_)));

        let src = importer
            .holding_repo
            .get(&lulubit, "USDT")
            .await
            .unwrap()
            .unwrap();
        let dst = importer
            .holding_repo
            .get("binance", "USDT")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(src.quantity, dec("600"));
        assert_eq!(dst.quantity, dec("400"));
        assert_eq!(dst.avg_cost_basis.unwrap(), dec("1"));
    }

    #[tokio::test]
    async fn insufficient_balance_fails_before_insert() {
        let pool = init_memory_pool().await.unwrap();
        let lulubit = account(&pool).await;
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) \
             VALUES ('binance', 'trading', 'Binance', 'exchange')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let importer = LulubitImporter::new(&pool);

        // No USDT was ever credited, so this transfer must fail...
        let r = importer
            .import(
                &lulubit,
                "lulubit-xfer-underfunded",
                None,
                LulubitOp::Transfer {
                    from_account: lulubit.clone(),
                    to_account: "binance".into(),
                    asset: "USDT".into(),
                    quantity: dec("100"),
                    date: date(2026, 1, 1),
                },
            )
            .await;
        assert!(r.is_err());

        // ...and it must NOT have left an orphaned transaction behind.
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE external_id = 'lulubit-xfer-underfunded'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count.0, 0);
    }
}
