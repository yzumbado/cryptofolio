use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::cli::{TxCommands, GlobalOptions};
use crate::cli::output::{format_quantity, format_usd, info, print_header, print_row, success};
use crate::core::transaction::Transaction;
use crate::db::{AccountRepository, HoldingRepository, TransactionRepository};
use crate::error::{CryptofolioError, Result};

pub async fn handle_tx_command(command: TxCommands, pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let _ = opts; // Used for quiet mode
    let account_repo = AccountRepository::new(pool);
    let holding_repo = HoldingRepository::new(pool);
    let tx_repo = TransactionRepository::new(pool);

    match command {
        TxCommands::List { account, limit } => {
            let transactions = if let Some(account_name) = account {
                let acc = account_repo.get_account(&account_name).await?
                    .ok_or_else(|| CryptofolioError::AccountNotFound(account_name.clone()))?;
                tx_repo.list_by_account(&acc.id, Some(limit)).await?
            } else {
                tx_repo.list(Some(limit)).await?
            };

            if transactions.is_empty() {
                println!("No transactions found.");
                return Ok(());
            }

            print_header(&[("Date", 12), ("Type", 10), ("Asset", 8), ("Quantity", 14), ("Price", 12)]);

            for tx in transactions {
                let date = tx.timestamp.format("%Y-%m-%d").to_string();
                let asset = tx.to_asset.or(tx.from_asset).unwrap_or_else(|| "-".to_string());
                let qty = tx.to_quantity.or(tx.from_quantity)
                    .map(|q| format_quantity(q))
                    .unwrap_or_else(|| "-".to_string());
                let price = tx.price_usd
                    .map(|p| format_usd(p))
                    .unwrap_or_else(|| "-".to_string());

                print_row(&[
                    (&date, 12),
                    (tx.tx_type.display_name(), 10),
                    (&asset, 8),
                    (&qty, 14),
                    (&price, 12),
                ]);
            }
        }

        TxCommands::Buy {
            asset,
            quantity,
            account,
            price,
            notes,
            dry_run,
        } => {
            let acc = account_repo.get_account(&account).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            let price_usd = Decimal::from_str(&price)
                .map_err(|_| CryptofolioError::InvalidAmount(price.clone()))?;

            if dry_run {
                info(&format!(
                    "[DRY RUN] Would record buy: {} {} @ {} in '{}' (total: {})",
                    format_quantity(qty),
                    asset.to_uppercase(),
                    format_usd(price_usd),
                    account,
                    format_usd(qty * price_usd)
                ));
                return Ok(());
            }

            // Update holdings
            holding_repo.add_quantity(&acc.id, &asset, qty, Some(price_usd)).await?;

            // Record transaction
            let mut tx = Transaction::new_buy(&acc.id, &asset, qty, price_usd, Utc::now());
            tx.notes = notes;
            tx_repo.insert(&tx).await?;

            success(&format!(
                "Recorded buy: {} {} @ {} in '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                format_usd(price_usd),
                account
            ));
        }

        TxCommands::Sell {
            asset,
            quantity,
            account,
            price,
            notes,
            dry_run,
        } => {
            let acc = account_repo.get_account(&account).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            let price_usd = Decimal::from_str(&price)
                .map_err(|_| CryptofolioError::InvalidAmount(price.clone()))?;

            if dry_run {
                info(&format!(
                    "[DRY RUN] Would record sell: {} {} @ {} from '{}' (total: {})",
                    format_quantity(qty),
                    asset.to_uppercase(),
                    format_usd(price_usd),
                    account,
                    format_usd(qty * price_usd)
                ));
                return Ok(());
            }

            // Update holdings
            holding_repo.remove_quantity(&acc.id, &asset, qty).await?;

            // Record transaction
            let mut tx = Transaction::new_sell(&acc.id, &asset, qty, price_usd, Utc::now());
            tx.notes = notes;
            tx_repo.insert(&tx).await?;

            success(&format!(
                "Recorded sell: {} {} @ {} from '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                format_usd(price_usd),
                account
            ));
        }

        TxCommands::Transfer {
            asset,
            quantity,
            from,
            to,
            fee,
            notes,
            dry_run,
        } => {
            let from_acc = account_repo.get_account(&from).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(from.clone()))?;

            let to_acc = account_repo.get_account(&to).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(to.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            let fee_amount = fee
                .map(|f| Decimal::from_str(&f))
                .transpose()
                .map_err(|_| CryptofolioError::InvalidAmount("fee".to_string()))?;

            if dry_run {
                let fee_str = fee_amount
                    .map(|f| format!(" (fee: {} {})", format_quantity(f), asset.to_uppercase()))
                    .unwrap_or_default();
                info(&format!(
                    "[DRY RUN] Would transfer: {} {} from '{}' to '{}'{}",
                    format_quantity(qty),
                    asset.to_uppercase(),
                    from,
                    to,
                    fee_str
                ));
                return Ok(());
            }

            // Get cost basis to preserve
            let holding = holding_repo.get(&from_acc.id, &asset).await?
                .ok_or_else(|| CryptofolioError::AssetNotFound(asset.clone()))?;

            // Update holdings
            holding_repo.remove_quantity(&from_acc.id, &asset, qty).await?;

            let transfer_qty = if let Some(f) = fee_amount {
                qty - f
            } else {
                qty
            };

            holding_repo.add_quantity(&to_acc.id, &asset, transfer_qty, holding.avg_cost_basis).await?;

            // Record transaction
            let mut tx = Transaction::new_transfer(&from_acc.id, &to_acc.id, &asset, qty, Utc::now());
            tx.fee = fee_amount;
            tx.fee_asset = Some(asset.to_uppercase());
            tx.notes = notes;
            tx_repo.insert(&tx).await?;

            success(&format!(
                "Recorded transfer: {} {} from '{}' to '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                from,
                to
            ));
        }

        TxCommands::Swap {
            from_asset,
            from_quantity,
            to_asset,
            to_quantity,
            account,
            notes,
            dry_run,
        } => {
            let acc = account_repo.get_account(&account).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let from_qty = Decimal::from_str(&from_quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(from_quantity.clone()))?;

            let to_qty = Decimal::from_str(&to_quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(to_quantity.clone()))?;

            if dry_run {
                info(&format!(
                    "[DRY RUN] Would swap: {} {} -> {} {} in '{}'",
                    format_quantity(from_qty),
                    from_asset.to_uppercase(),
                    format_quantity(to_qty),
                    to_asset.to_uppercase(),
                    account
                ));
                return Ok(());
            }

            // Update holdings
            holding_repo.remove_quantity(&acc.id, &from_asset, from_qty).await?;

            // Calculate implied price for cost basis
            let implied_price = if from_qty > Decimal::ZERO {
                // Get current holding to calculate USD value
                let from_holding = holding_repo.get(&acc.id, &from_asset).await?;
                from_holding.and_then(|h| h.avg_cost_basis).map(|cost| cost * from_qty / to_qty)
            } else {
                None
            };

            holding_repo.add_quantity(&acc.id, &to_asset, to_qty, implied_price).await?;

            // Record transaction
            let mut tx = Transaction::new_swap(&acc.id, &from_asset, from_qty, &to_asset, to_qty, Utc::now());
            tx.notes = notes;
            tx_repo.insert(&tx).await?;

            success(&format!(
                "Recorded swap: {} {} -> {} {} in '{}'",
                format_quantity(from_qty),
                from_asset.to_uppercase(),
                format_quantity(to_qty),
                to_asset.to_uppercase(),
                account
            ));
        }
    }

    Ok(())
}
