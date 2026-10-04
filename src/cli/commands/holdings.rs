use chrono::Utc;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::cli::output::{
    format_quantity, format_usd, print_header, print_row, success, suggest_next,
};
use crate::cli::{GlobalOptions, HoldingsCommands};
use crate::config::AppConfig;
use crate::core::holdings::Holding;
use crate::core::transaction::Transaction;
use crate::db::{AccountRepository, HoldingRepository, TransactionRepository};
use crate::error::{CryptofolioError, Result};

/// Drop holdings whose symbol is hidden by `[display] hidden_assets`.
///
/// Presentation-only: the repository rows (and the ledger) are untouched; the
/// filtered vector is used solely for rendering.
fn filter_hidden_holdings(holdings: Vec<Holding>, config: &AppConfig) -> Vec<Holding> {
    holdings
        .into_iter()
        .filter(|h| !config.display.is_hidden(&h.asset))
        .collect()
}

#[derive(Serialize)]
struct HoldingOutput {
    asset: String,
    quantity: String,
    cost_basis: Option<String>,
    account: String,
    account_id: String,
}

pub async fn handle_holdings_command(
    command: HoldingsCommands,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_repo = AccountRepository::new(pool);
    let holding_repo = HoldingRepository::new(pool);
    let tx_repo = TransactionRepository::new(pool);

    match command {
        HoldingsCommands::List { account } => {
            let holdings = if let Some(account_name) = account {
                let acc = account_repo
                    .get_account(&account_name)
                    .await?
                    .ok_or_else(|| CryptofolioError::AccountNotFound(account_name.clone()))?;
                holding_repo.list_by_account(&acc.id).await?
            } else {
                holding_repo.list_all().await?
            };

            // Display-only filter: `[display] hidden_assets` symbols are skipped
            // for both human and --json output, so automation matches the view.
            let config = AppConfig::load()?;
            let holdings = filter_hidden_holdings(holdings, &config);

            if holdings.is_empty() {
                if opts.json {
                    println!("[]");
                } else {
                    println!("No holdings found.");
                }
                return Ok(());
            }

            if opts.json {
                let mut output = Vec::new();
                for holding in holdings {
                    let account = account_repo.get_account_by_id(&holding.account_id).await?;
                    let account_name = account.map(|a| a.name).unwrap_or_else(|| "-".to_string());

                    output.push(HoldingOutput {
                        asset: holding.asset.clone(),
                        quantity: holding.quantity.to_string(),
                        cost_basis: holding.avg_cost_basis.map(|c| c.to_string()),
                        account: account_name,
                        account_id: holding.account_id.clone(),
                    });
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_default()
                );
            } else {
                // Group by account if showing all
                print_header(&[
                    ("Asset", 8),
                    ("Quantity", 18),
                    ("Cost Basis", 12),
                    ("Account", 20),
                ]);

                for holding in holdings {
                    let account = account_repo.get_account_by_id(&holding.account_id).await?;
                    let account_name = account.map(|a| a.name).unwrap_or_else(|| "-".to_string());

                    let cost_str = holding
                        .avg_cost_basis
                        .map(format_usd)
                        .unwrap_or_else(|| "-".to_string());

                    print_row(&[
                        (&holding.asset, 8),
                        (&format_quantity(holding.quantity), 18),
                        (&cost_str, 12),
                        (&account_name, 20),
                    ]);
                }
            }
        }

        HoldingsCommands::Add {
            asset,
            quantity,
            account,
            cost,
        } => {
            let acc = account_repo
                .get_account(&account)
                .await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            let cost_per_unit = cost
                .map(|c| Decimal::from_str(&c))
                .transpose()
                .map_err(|_| CryptofolioError::InvalidAmount("cost".to_string()))?;

            holding_repo
                .add_quantity(&acc.id, &asset, qty, cost_per_unit)
                .await?;

            // Record transaction
            let mut tx = Transaction::new_buy(
                &acc.id,
                &asset,
                qty,
                cost_per_unit.unwrap_or(Decimal::ZERO),
                Utc::now(),
            );
            tx.notes = Some("Manual holding addition".to_string());
            tx_repo.insert(&tx).await?;

            success(&format!(
                "Added {} {} to '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                account
            ));

            if !opts.quiet {
                suggest_next("cryptofolio portfolio", "View your portfolio");
            }
        }

        HoldingsCommands::Remove {
            asset,
            quantity,
            account,
            yes,
        } => {
            let acc = account_repo
                .get_account(&account)
                .await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            if !yes {
                println!(
                    "This will remove {} {} from '{}'.",
                    format_quantity(qty),
                    asset.to_uppercase(),
                    account
                );
                print!("Are you sure? [y/N] ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            holding_repo.remove_quantity(&acc.id, &asset, qty).await?;

            success(&format!(
                "Removed {} {} from '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                account
            ));
        }

        HoldingsCommands::Set {
            asset,
            quantity,
            account,
            cost,
        } => {
            let acc = account_repo
                .get_account(&account)
                .await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            let cost_per_unit = cost
                .map(|c| Decimal::from_str(&c))
                .transpose()
                .map_err(|_| CryptofolioError::InvalidAmount("cost".to_string()))?;

            holding_repo
                .set_quantity(&acc.id, &asset, qty, cost_per_unit)
                .await?;

            success(&format!(
                "Set {} {} in '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                account
            ));
        }

        HoldingsCommands::Move {
            asset,
            quantity,
            from,
            to,
            yes,
        } => {
            let from_acc = account_repo
                .get_account(&from)
                .await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(from.clone()))?;

            let to_acc = account_repo
                .get_account(&to)
                .await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(to.clone()))?;

            let qty = Decimal::from_str(&quantity)
                .map_err(|_| CryptofolioError::InvalidAmount(quantity.clone()))?;

            if !yes {
                println!(
                    "This will move {} {} from '{}' to '{}'.",
                    format_quantity(qty),
                    asset.to_uppercase(),
                    from,
                    to
                );
                print!("Are you sure? [y/N] ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            // Get current holding to preserve cost basis
            let holding = holding_repo
                .get(&from_acc.id, &asset)
                .await?
                .ok_or_else(|| CryptofolioError::AssetNotFound(asset.clone()))?;

            // Remove from source
            holding_repo
                .remove_quantity(&from_acc.id, &asset, qty)
                .await?;

            // Add to destination (with same cost basis)
            holding_repo
                .add_quantity(&to_acc.id, &asset, qty, holding.avg_cost_basis)
                .await?;

            // Record transfer transaction
            let tx = Transaction::new_transfer(&from_acc.id, &to_acc.id, &asset, qty, Utc::now());
            tx_repo.insert(&tx).await?;

            success(&format!(
                "Moved {} {} from '{}' to '{}'",
                format_quantity(qty),
                asset.to_uppercase(),
                from,
                to
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Synthetic placeholder symbols only — never real airdrop tokens.
    fn holding(asset: &str) -> Holding {
        Holding {
            id: 1,
            account_id: "acc".to_string(),
            asset: asset.to_string(),
            quantity: Decimal::ONE,
            avg_cost_basis: None,
            cost_basis_currency: None,
            avg_cost_basis_base: None,
            updated_at: Utc::now(),
        }
    }

    fn config_with_hidden(symbols: &[&str]) -> AppConfig {
        let mut config = AppConfig::default();
        config.display.hidden_assets = symbols.iter().map(|s| s.to_string()).collect();
        config
    }

    #[test]
    fn filter_hidden_holdings_drops_hidden_case_insensitively() {
        let holdings = vec![holding("scamtokena"), holding("SCAMTOKENB"), holding("BTC")];
        let config = config_with_hidden(&["SCAMTOKENA"]);
        let visible: Vec<_> = filter_hidden_holdings(holdings, &config)
            .into_iter()
            .map(|h| h.asset)
            .collect();
        assert_eq!(visible, vec!["SCAMTOKENB".to_string(), "BTC".to_string()]);
    }

    #[test]
    fn filter_hidden_holdings_empty_list_is_a_no_op() {
        let holdings = vec![holding("SCAMTOKENA"), holding("SCAMTOKENB")];
        let config = AppConfig::default();
        assert_eq!(filter_hidden_holdings(holdings, &config).len(), 2);
    }

    #[test]
    fn filter_hidden_holdings_can_remove_every_row() {
        let holdings = vec![holding("SCAMTOKENA"), holding("SCAMTOKENB")];
        let config = config_with_hidden(&["scamtokena", "scamtokenb"]);
        assert!(filter_hidden_holdings(holdings, &config).is_empty());
    }
}
