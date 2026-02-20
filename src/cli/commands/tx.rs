use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::SqlitePool;
use std::fs::File;
use std::str::FromStr;

use crate::cli::{TxCommands, GlobalOptions};
use crate::cli::output::{format_quantity, format_usd, info, print_header, print_row, success};
use crate::core::transaction::Transaction;
use crate::core::currency::ExchangeRate;
use crate::db::{AccountRepository, HoldingRepository, TransactionRepository, currencies};
use crate::error::{CryptofolioError, Result};

#[derive(Serialize)]
struct TransactionOutput {
    id: i64,
    timestamp: String,
    tx_type: String,
    from_account_id: Option<String>,
    to_account_id: Option<String>,
    from_asset: Option<String>,
    from_quantity: Option<String>,
    to_asset: Option<String>,
    to_quantity: Option<String>,
    price_usd: Option<String>,
    fee: Option<String>,
    fee_asset: Option<String>,
    notes: Option<String>,
}

#[derive(Serialize)]
struct CsvExportRecord {
    date: String,
    #[serde(rename = "type")]
    tx_type: String,
    asset: String,
    quantity: String,
    price_usd: String,
    fee: String,
    fee_asset: String,
    notes: String,
    to_asset: String,
    to_quantity: String,
}

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
                if opts.json {
                    println!("[]");
                } else {
                    println!("No transactions found.");
                }
                return Ok(());
            }

            if opts.json {
                let output: Vec<TransactionOutput> = transactions.iter().map(|tx| TransactionOutput {
                    id: tx.id,
                    timestamp: tx.timestamp.to_rfc3339(),
                    tx_type: tx.tx_type.display_name().to_string(),
                    from_account_id: tx.from_account_id.clone(),
                    to_account_id: tx.to_account_id.clone(),
                    from_asset: tx.from_asset.clone(),
                    from_quantity: tx.from_quantity.map(|q| q.to_string()),
                    to_asset: tx.to_asset.clone(),
                    to_quantity: tx.to_quantity.map(|q| q.to_string()),
                    price_usd: tx.price_usd.map(|p| p.to_string()),
                    fee: tx.fee.map(|f| f.to_string()),
                    fee_asset: tx.fee_asset.clone(),
                    notes: tx.notes.clone(),
                }).collect();
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
            } else {
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
            rate,
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

            // Check if both assets are fiat currencies
            let from_currency = currencies::get_currency(pool, &from_asset.to_uppercase()).await?;
            let to_currency = currencies::get_currency(pool, &to_asset.to_uppercase()).await?;

            let is_fiat_swap = from_currency.as_ref().map(|c| c.is_fiat()).unwrap_or(false)
                            && to_currency.as_ref().map(|c| c.is_fiat()).unwrap_or(false);

            // Calculate and store exchange rate for fiat swaps
            let (exchange_rate, exchange_rate_pair) = if is_fiat_swap {
                // Use manual rate if provided, otherwise calculate from quantities
                let rate_value = if let Some(ref manual_rate) = rate {
                    Decimal::from_str(manual_rate)
                        .map_err(|_| CryptofolioError::InvalidInput(format!("Invalid rate: {}", manual_rate)))?
                } else if to_qty > Decimal::ZERO {
                    from_qty / to_qty
                } else {
                    Decimal::ZERO
                };

                let pair = format!("{}/{}", from_asset.to_uppercase(), to_asset.to_uppercase());

                // Store the exchange rate for future reference
                let rate_record = ExchangeRate::new_manual(
                    &from_asset.to_uppercase(),
                    &to_asset.to_uppercase(),
                    rate_value,
                    Utc::now()
                );
                currencies::add_exchange_rate(pool, &rate_record).await?;

                info(&format!(
                    "Recorded exchange rate: {} {} = 1 {}",
                    rate_value,
                    from_asset.to_uppercase(),
                    to_asset.to_uppercase()
                ));

                (Some(rate_value), Some(pair))
            } else {
                (None, None)
            };

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
            tx.exchange_rate = exchange_rate;
            tx.exchange_rate_pair = exchange_rate_pair;
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

        TxCommands::Export {
            file,
            account,
            asset,
            from,
            to,
            limit,
        } => {
            handle_export_command(file, account, asset, from, to, limit, pool, opts).await?;
        }
    }

    Ok(())
}

async fn handle_export_command(
    file: String,
    account_filter: Option<String>,
    asset_filter: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    limit: i64,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_repo = AccountRepository::new(pool);
    let tx_repo = TransactionRepository::new(pool);

    // Parse date filters if provided
    let from_timestamp = if let Some(date_str) = from_date {
        Some(parse_date_filter(&date_str)?)
    } else {
        None
    };

    let to_timestamp = if let Some(date_str) = to_date {
        Some(parse_date_filter(&date_str)?)
    } else {
        None
    };

    // Get account ID if filter specified
    let account_id = if let Some(account_name) = &account_filter {
        let acc = account_repo.get_account(account_name).await?
            .ok_or_else(|| CryptofolioError::AccountNotFound(account_name.clone()))?;
        Some(acc.id)
    } else {
        None
    };

    // Fetch transactions
    let mut transactions = if let Some(acc_id) = &account_id {
        if limit > 0 {
            tx_repo.list_by_account(acc_id, Some(limit)).await?
        } else {
            tx_repo.list_by_account(acc_id, None).await?
        }
    } else {
        if limit > 0 {
            tx_repo.list(Some(limit)).await?
        } else {
            tx_repo.list(None).await?
        }
    };

    // Apply filters
    if let Some(from_ts) = from_timestamp {
        transactions.retain(|tx| tx.timestamp >= from_ts);
    }

    if let Some(to_ts) = to_timestamp {
        transactions.retain(|tx| tx.timestamp <= to_ts);
    }

    if let Some(asset_sym) = &asset_filter {
        let asset_upper = asset_sym.to_uppercase();
        transactions.retain(|tx| {
            tx.from_asset.as_ref().map(|a| a == &asset_upper).unwrap_or(false)
                || tx.to_asset.as_ref().map(|a| a == &asset_upper).unwrap_or(false)
        });
    }

    if transactions.is_empty() {
        if !opts.quiet {
            info("No transactions match the specified filters");
        }
        return Ok(());
    }

    // Convert transactions to CSV format
    let csv_records: Vec<CsvExportRecord> = transactions.iter()
        .map(|tx| transaction_to_csv_record(tx))
        .collect();

    // Write to CSV file
    if !opts.quiet {
        info(&format!("Exporting {} transactions to '{}'...", csv_records.len(), file));
    }

    let file_handle = File::create(&file)?;
    let mut writer = csv::Writer::from_writer(file_handle);

    for record in csv_records {
        writer.serialize(&record)?;
    }

    writer.flush()?;

    success(&format!("Exported {} transactions to '{}'", transactions.len(), file));

    Ok(())
}

fn transaction_to_csv_record(tx: &Transaction) -> CsvExportRecord {
    use crate::core::transaction::TransactionType;

    // Determine primary asset and quantity based on transaction type
    let (asset, quantity) = match tx.tx_type {
        TransactionType::Buy | TransactionType::Receive | TransactionType::TransferIn => {
            (
                tx.to_asset.clone().unwrap_or_default(),
                tx.to_quantity.map(|q| q.to_string()).unwrap_or_default(),
            )
        }
        TransactionType::Sell | TransactionType::TransferOut | TransactionType::Fee => {
            (
                tx.from_asset.clone().unwrap_or_default(),
                tx.from_quantity.map(|q| q.to_string()).unwrap_or_default(),
            )
        }
        TransactionType::TransferInternal => {
            // For internal transfers, use the asset being transferred
            (
                tx.from_asset.clone().or_else(|| tx.to_asset.clone()).unwrap_or_default(),
                tx.from_quantity.or(tx.to_quantity).map(|q| q.to_string()).unwrap_or_default(),
            )
        }
        TransactionType::Swap => {
            (
                tx.from_asset.clone().unwrap_or_default(),
                tx.from_quantity.map(|q| q.to_string()).unwrap_or_default(),
            )
        }
    };

    CsvExportRecord {
        date: tx.timestamp.to_rfc3339(),
        tx_type: tx.tx_type.as_str().to_string(),
        asset,
        quantity,
        price_usd: tx.price_usd.map(|p| p.to_string()).unwrap_or_default(),
        fee: tx.fee.map(|f| f.to_string()).unwrap_or_default(),
        fee_asset: tx.fee_asset.clone().unwrap_or_default(),
        notes: tx.notes.clone().unwrap_or_default(),
        to_asset: if matches!(tx.tx_type, TransactionType::Swap) {
            tx.to_asset.clone().unwrap_or_default()
        } else {
            String::new()
        },
        to_quantity: if matches!(tx.tx_type, TransactionType::Swap) {
            tx.to_quantity.map(|q| q.to_string()).unwrap_or_default()
        } else {
            String::new()
        },
    }
}

fn parse_date_filter(date_str: &str) -> Result<DateTime<Utc>> {
    // Try RFC3339 format first
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try YYYY-MM-DD format
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        if let Some(naive_datetime) = naive_date.and_hms_opt(0, 0, 0) {
            return Ok(naive_datetime.and_utc());
        }
    }

    Err(CryptofolioError::Config(
        format!("Invalid date format: '{}'. Use YYYY-MM-DD or ISO 8601", date_str)
    ))
}
