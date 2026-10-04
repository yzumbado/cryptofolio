//! Ledger self-reconciliation: compare each account/asset's recorded balance
//! (`holdings.quantity`) against the net flow implied by the transaction
//! history.
//!
//! A non-zero delta means the transaction history does not explain the recorded
//! balance — a transfer was missed, a row was duplicated, or an amount was
//! entered wrong. This module is a DIAGNOSTIC only: it changes nothing and
//! imposes no policy. `status` is the mechanical `verified` (delta == 0) /
//! `unreconciled` (delta != 0); the "partial" tier and any dust-tolerance
//! policy are deliberately left out until the owner decides from real deltas.
//!
//! `onchain_balance` here is the *recorded* balance (the last-known value in
//! `holdings`, which sync sets from the chain). This diagnostic performs no
//! fresh chain read; a live sync is the source of a truly on-chain value.

use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

use crate::core::holdings::Holding;
use crate::core::transaction::Transaction;

/// One account/asset reconciliation reading.
#[derive(Debug, Clone, PartialEq)]
pub struct ReconciliationRow {
    pub account_id: String,
    pub account_name: String,
    pub asset: String,
    /// Recorded balance from `holdings` (last-known; sync sets it from chain).
    pub onchain_balance: Decimal,
    /// Net flow from the transaction history (inflows minus outflows).
    pub computed_balance: Decimal,
    /// `onchain_balance - computed_balance`.
    pub delta: Decimal,
    /// `"verified"` when delta is zero, else `"unreconciled"`.
    pub status: &'static str,
}

/// Compute reconciliation rows for every `(account, asset)` present in either
/// the transaction history or the holdings table.
///
/// `computed_balance` sums `to_quantity` for rows where this account receives
/// the asset and subtracts `from_quantity` for rows where it sends the asset.
/// This covers buys, sells, swaps, transfers, and dated `receive` rows, and is
/// independent of `tx_type` so it cannot drift when the enum gains a variant.
pub fn reconcile(
    transactions: &[Transaction],
    holdings: &[Holding],
    account_names: &HashMap<String, String>,
) -> Vec<ReconciliationRow> {
    let mut computed: HashMap<(String, String), Decimal> = HashMap::new();
    for t in transactions {
        if let (Some(acc), Some(asset), Some(qty)) = (&t.to_account_id, &t.to_asset, &t.to_quantity)
        {
            *computed.entry((acc.clone(), asset.clone())).or_default() += qty;
        }
        if let (Some(acc), Some(asset), Some(qty)) =
            (&t.from_account_id, &t.from_asset, &t.from_quantity)
        {
            *computed.entry((acc.clone(), asset.clone())).or_default() -= qty;
        }
    }

    let mut onchain: HashMap<(String, String), Decimal> = HashMap::new();
    for h in holdings {
        onchain.insert((h.account_id.clone(), h.asset.clone()), h.quantity);
    }

    let mut keys: HashSet<(String, String)> = computed.keys().cloned().collect();
    keys.extend(onchain.keys().cloned());

    let mut rows: Vec<ReconciliationRow> = keys
        .into_iter()
        .map(|(account_id, asset)| {
            let onchain_balance = onchain
                .get(&(account_id.clone(), asset.clone()))
                .copied()
                .unwrap_or(Decimal::ZERO);
            let computed_balance = computed
                .get(&(account_id.clone(), asset.clone()))
                .copied()
                .unwrap_or(Decimal::ZERO);
            let delta = onchain_balance - computed_balance;
            let status = if delta.is_zero() {
                "verified"
            } else {
                "unreconciled"
            };
            let account_name = account_names
                .get(&account_id)
                .cloned()
                .unwrap_or_else(|| account_id.clone());
            ReconciliationRow {
                account_id,
                account_name,
                asset,
                onchain_balance,
                computed_balance,
                delta,
                status,
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        (&a.account_name, &a.asset, &a.account_id).cmp(&(&b.account_name, &b.asset, &b.account_id))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn now() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    fn holding(account: &str, asset: &str, qty: &str) -> Holding {
        Holding {
            id: 0,
            account_id: account.to_string(),
            asset: asset.to_string(),
            quantity: dec(qty),
            avg_cost_basis: None,
            cost_basis_currency: Some("USD".to_string()),
            avg_cost_basis_base: None,
            updated_at: Utc::now(),
        }
    }

    fn names() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("a1".to_string(), "Binance".to_string());
        m.insert("a2".to_string(), "Ledger".to_string());
        m
    }

    #[test]
    fn verified_when_history_matches_holdings() {
        let txs = vec![
            Transaction::new_buy("a1", "BTC", dec("2.0"), dec("50000"), now()),
            Transaction::new_sell("a1", "BTC", dec("0.5"), dec("60000"), now()),
        ];
        let holdings = vec![holding("a1", "BTC", "1.5")];
        let rows = reconcile(&txs, &holdings, &names());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].asset, "BTC");
        assert_eq!(rows[0].computed_balance, dec("1.5"));
        assert_eq!(rows[0].delta, dec("0"));
        assert_eq!(rows[0].status, "verified");
    }

    #[test]
    fn unreconciled_when_history_does_not_match() {
        let txs = vec![Transaction::new_buy(
            "a1",
            "BTC",
            dec("1.2"),
            dec("50000"),
            now(),
        )];
        let holdings = vec![holding("a1", "BTC", "1.5")];
        let rows = reconcile(&txs, &holdings, &names());
        assert_eq!(rows[0].delta, dec("0.3"));
        assert_eq!(rows[0].status, "unreconciled");
    }

    #[test]
    fn transfer_moves_balance_between_accounts() {
        let txs = vec![Transaction::new_transfer(
            "a1",
            "a2",
            "ETH",
            dec("1.0"),
            now(),
        )];
        let holdings = vec![holding("a2", "ETH", "1.0")];
        let rows = reconcile(&txs, &holdings, &names());
        let a1 = rows
            .iter()
            .find(|r| r.account_id == "a1" && r.asset == "ETH")
            .unwrap();
        let a2 = rows
            .iter()
            .find(|r| r.account_id == "a2" && r.asset == "ETH")
            .unwrap();
        assert_eq!(a1.computed_balance, dec("-1.0"));
        assert_eq!(a1.delta, dec("1.0")); // ledger says 0, history says -1
        assert_eq!(a2.computed_balance, dec("1.0"));
        assert_eq!(a2.delta, dec("0"));
        assert_eq!(a2.status, "verified");
    }

    #[test]
    fn swap_reduces_source_and_increases_destination() {
        let txs = vec![Transaction::new_swap(
            "a1",
            "USDC",
            dec("1000"),
            "ETH",
            dec("0.25"),
            now(),
        )];
        let holdings = vec![holding("a1", "ETH", "0.25")];
        let rows = reconcile(&txs, &holdings, &names());
        let eth = rows.iter().find(|r| r.asset == "ETH").unwrap();
        let usdc = rows.iter().find(|r| r.asset == "USDC").unwrap();
        assert_eq!(eth.computed_balance, dec("0.25"));
        assert_eq!(eth.status, "verified");
        assert_eq!(usdc.computed_balance, dec("-1000"));
        assert_eq!(usdc.status, "unreconciled");
    }

    #[test]
    fn holding_with_no_history_is_fully_unreconciled() {
        let holdings = vec![holding("a1", "DOGE", "100")];
        let rows = reconcile(&[], &holdings, &names());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].computed_balance, dec("0"));
        assert_eq!(rows[0].delta, dec("100"));
        assert_eq!(rows[0].status, "unreconciled");
    }
}
