use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::cli::output::{format_quantity, info, success, warning};
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::core::account::AccountType;
use crate::core::holdings::Holding;
use crate::db::{AccountRepository, HoldingRepository};
use crate::error::{CryptofolioError, Result};
use crate::exchange::{BinanceClient, Exchange};
use chrono::Utc;

pub async fn handle_sync_command(account: Option<String>, pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?;
    let account_repo = AccountRepository::new(pool);
    let holding_repo = HoldingRepository::new(pool);

    // Check if we have credentials
    if !config.has_binance_credentials() {
        return Err(CryptofolioError::AuthRequired(
            "Binance API credentials not configured. Use 'cryptofolio config set binance.api_key <key>' and 'cryptofolio config set binance.api_secret <secret>'".into()
        ));
    }

    // Get accounts to sync
    let accounts = account_repo.list_accounts().await?;
    let accounts_to_sync: Vec<_> = accounts
        .into_iter()
        .filter(|a| {
            // Filter by account name if specified
            if let Some(ref name) = account {
                if a.name.to_lowercase() != name.to_lowercase() {
                    return false;
                }
            }

            // Only sync exchange accounts with sync enabled
            matches!(a.account_type, AccountType::Exchange) && a.sync_enabled
        })
        .collect();

    if accounts_to_sync.is_empty() {
        if account.is_some() {
            warning("Specified account is not an exchange account or sync is not enabled.");
        } else {
            warning("No exchange accounts with sync enabled found.");
        }
        println!("Use 'cryptofolio account add <name> --type exchange --category trading --sync' to create one.");
        return Ok(());
    }

    for acc in accounts_to_sync {
        if !opts.quiet {
            info(&format!("Syncing '{}'...", acc.name));
        }

        // Use account-specific testnet setting or global (CLI flag takes precedence)
        let is_testnet = opts.testnet || acc.config.is_testnet || config.general.use_testnet;

        if is_testnet && !opts.quiet {
            println!("  {}", "[Testnet Mode]".yellow());
        }

        let client = BinanceClient::new(
            is_testnet,
            config.binance.api_key.clone(),
            config.binance.api_secret.clone(),
        );

        // Show progress spinner
        let spinner = if !opts.quiet {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap());
            pb.set_message("Fetching balances...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };

        // Fetch balances
        let balances = client.get_balances().await?;

        if let Some(pb) = &spinner {
            pb.finish_and_clear();
        }

        // Clear existing holdings for this account
        holding_repo.delete_all_for_account(&acc.id).await?;

        // Insert new holdings
        let mut synced_count = 0;
        for balance in balances {
            if balance.total() > Decimal::ZERO {
                let holding = Holding {
                    id: 0,
                    account_id: acc.id.clone(),
                    asset: balance.asset.clone(),
                    quantity: balance.total(),
                    avg_cost_basis: None, // Exchange doesn't provide cost basis
                    cost_basis_currency: None,
                    avg_cost_basis_base: None,
                    updated_at: Utc::now(),
                };

                holding_repo.upsert(&holding).await?;
                synced_count += 1;

                if !opts.quiet {
                    println!(
                        "  {} {} {}",
                        "+".green(),
                        balance.asset,
                        format_quantity(balance.total())
                    );
                }
            }
        }

        if !opts.quiet {
            success(&format!("Synced {} assets from '{}'", synced_count, acc.name));
        }
    }

    Ok(())
}
