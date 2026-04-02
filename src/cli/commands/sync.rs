use chrono::{NaiveDate, TimeZone, Utc};
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
use crate::exchange::binance::sync::{SyncOptions, SyncOrchestrator};
use crate::exchange::{BinanceClient, Exchange};

pub async fn handle_sync_command(
    account: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
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
            // Use a fallback style if the template fails
            let style = ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner());
            pb.set_style(style);
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
            success(&format!(
                "Synced {} assets from '{}'",
                synced_count, acc.name
            ));
        }
    }

    Ok(())
}

// =============================================================================
// sync-history command
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub async fn handle_sync_history_command(
    account: String,
    symbols: String,
    full_history: bool,
    from: Option<String>,
    no_trades: bool,
    no_deposits: bool,
    no_withdrawals: bool,
    no_fiat: bool,
    no_transfers: bool,
    dry_run: bool,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let config = AppConfig::load()?;

    if !config.has_binance_credentials() {
        return Err(CryptofolioError::AuthRequired(
            "Binance API credentials not configured. Use 'cryptofolio config set-secret binance.api_key' and '...api_secret'.".into(),
        ));
    }

    // Resolve account
    let account_repo = AccountRepository::new(pool);
    let acc = account_repo
        .get_account(&account)
        .await?
        .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

    if !matches!(acc.account_type, AccountType::Exchange) {
        return Err(CryptofolioError::Other(format!(
            "Account '{}' is not an exchange account.",
            account
        )));
    }

    // Parse symbols list
    let symbol_list: Vec<String> = symbols
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    // Parse optional start date
    let start_time = from
        .as_deref()
        .map(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| {
                    CryptofolioError::Other(format!("Invalid date '{}'. Expected YYYY-MM-DD.", s))
                })
                .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).expect("0:00:00 is always valid")))
        })
        .transpose()?;

    let is_testnet = opts.testnet || acc.config.is_testnet || config.general.use_testnet;

    let client = BinanceClient::new(
        is_testnet,
        config.binance.api_key.clone(),
        config.binance.api_secret.clone(),
    );

    let sync_opts = SyncOptions {
        symbols: symbol_list.clone(),
        sync_trades: !no_trades,
        sync_deposits: !no_deposits,
        sync_withdrawals: !no_withdrawals,
        sync_fiat: !no_fiat,
        sync_transfers: !no_transfers,
        full_history,
        dry_run,
        start_time,
        ..Default::default()
    };

    // Print header
    if !opts.quiet {
        if dry_run {
            println!("{}", "  [dry-run — no changes will be written]".yellow());
        }
        if full_history {
            info("Syncing full history from the beginning...");
        } else {
            info("Syncing new transactions since last sync...");
        }
        if !symbol_list.is_empty() {
            println!("  Symbols: {}", symbol_list.join(", ").cyan());
        }
        println!();
    }

    // Show spinner
    let spinner = if !opts.quiet && !dry_run {
        let pb = ProgressBar::new_spinner();
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
        pb.set_style(style);
        pb.set_message("Fetching history from Binance...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };

    let orchestrator = SyncOrchestrator::new(pool, &client);
    let report = orchestrator.sync(&acc.id, &sync_opts).await?;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Print results
    if !opts.quiet {
        println!();
        let verb = if dry_run { "would import" } else { "imported" };

        if report.trades_created > 0 || report.trades_skipped > 0 {
            println!(
                "  {} Trades:      {} {}, {} skipped",
                "✓".green(),
                report.trades_created,
                verb,
                report.trades_skipped
            );
        }
        if report.deposits_created > 0 || report.deposits_skipped > 0 {
            println!(
                "  {} Deposits:    {} {}, {} skipped",
                "✓".green(),
                report.deposits_created,
                verb,
                report.deposits_skipped
            );
        }
        if report.withdrawals_created > 0 || report.withdrawals_skipped > 0 {
            println!(
                "  {} Withdrawals: {} {}, {} skipped",
                "✓".green(),
                report.withdrawals_created,
                verb,
                report.withdrawals_skipped
            );
        }
        if report.fiat_orders_created > 0 || report.fiat_orders_skipped > 0 {
            println!(
                "  {} Fiat orders: {} {}, {} skipped",
                "✓".green(),
                report.fiat_orders_created,
                verb,
                report.fiat_orders_skipped
            );
        }
        if report.transfers_created > 0 || report.transfers_skipped > 0 {
            println!(
                "  {} Transfers:   {} {}, {} skipped",
                "✓".green(),
                report.transfers_created,
                verb,
                report.transfers_skipped
            );
        }

        println!();

        if report.has_errors() {
            println!("{}", "  Errors encountered:".red());
            for err in &report.errors {
                println!("    {} {}", "✗".red(), err);
            }
            println!();
        }

        let label = if dry_run {
            "[dry-run] Would import"
        } else {
            "Imported"
        };
        success(&format!(
            "{} {} transactions from '{}' ({} skipped)",
            label,
            report.total_created(),
            account,
            report.total_skipped(),
        ));

        if report.total_created() > 0 && !dry_run {
            println!();
            println!("  Run {} to see results.", "cryptofolio tx list".cyan());
            println!(
                "  Run {} to see updated P&L.",
                "cryptofolio pnl summary".cyan()
            );
        }
    }

    Ok(())
}
