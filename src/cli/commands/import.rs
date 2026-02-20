use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use crate::cli::output::{error, info, success, suggest_next};
use crate::cli::GlobalOptions;
use crate::core::transaction::{Transaction, TransactionType};
use crate::db::{AccountRepository, HoldingRepository, TransactionRepository};
use crate::error::{CryptofolioError, Result};

#[derive(Debug, Deserialize)]
struct CsvTransaction {
    date: String,
    #[serde(rename = "type")]
    tx_type: String,
    asset: String,
    quantity: String,
    #[serde(default)]
    price_usd: Option<String>,
    #[serde(default)]
    fee: Option<String>,
    #[serde(default)]
    fee_asset: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    to_asset: Option<String>,
    #[serde(default)]
    to_quantity: Option<String>,
}

pub async fn handle_import_command(
    file: String,
    account: String,
    format: String,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    if format != "csv" {
        return Err(CryptofolioError::Config(format!("Unsupported format: {}. Only 'csv' is supported.", format)));
    }

    let account_repo = AccountRepository::new(pool);
    let holding_repo = HoldingRepository::new(pool);
    let tx_repo = TransactionRepository::new(pool);

    // Get account
    let acc = account_repo.get_account(&account).await?
        .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

    // Check file exists
    let path = Path::new(&file);
    if !path.exists() {
        return Err(CryptofolioError::Config(format!("File not found: {}", file)));
    }

    if !opts.quiet {
        info(&format!("Importing from '{}' into '{}'...", file, account));
    }

    // Parse CSV
    let file = File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);

    // Count total rows for progress (read once, then reset)
    let total_rows = reader.records().count();
    let file = File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);

    let progress = if !opts.quiet && total_rows > 0 {
        let pb = ProgressBar::new(total_rows as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        Some(pb)
    } else {
        None
    };

    let mut imported = 0;
    let mut errors = 0;

    for (line_num, result) in reader.deserialize().enumerate() {
        let line = line_num + 2; // +1 for header, +1 for 1-based

        match process_row(result, &acc.id, &holding_repo, &tx_repo).await {
            Ok(_) => {
                imported += 1;
            }
            Err(e) => {
                error(&format!("Line {}: {}", line, e));
                errors += 1;
            }
        }

        if let Some(ref pb) = progress {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    if errors > 0 {
        println!();
        success(&format!("Imported {} transactions ({} errors)", imported, errors));
    } else {
        success(&format!("Imported {} transactions", imported));
    }

    if !opts.quiet {
        suggest_next("cryptofolio tx list", "View imported transactions");
    }

    Ok(())
}

async fn process_row(
    result: std::result::Result<CsvTransaction, csv::Error>,
    account_id: &str,
    holding_repo: &HoldingRepository<'_>,
    tx_repo: &TransactionRepository<'_>,
) -> Result<()> {
    let row = result.map_err(|e| CryptofolioError::Csv(e))?;

    // Parse transaction type
    let tx_type = TransactionType::from_str(&row.tx_type)
        .ok_or_else(|| CryptofolioError::Other(format!("Invalid transaction type: {}", row.tx_type)))?;

    // Parse date
    let timestamp = DateTime::parse_from_rfc3339(&row.date)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            // Try alternative formats
            DateTime::parse_from_str(&row.date, "%Y-%m-%d %H:%M:%S")
                .map(|dt| dt.with_timezone(&Utc))
        })
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        })
        .map_err(|_| CryptofolioError::Other(format!("Invalid date format: {}", row.date)))?;

    // Parse quantity
    let quantity = Decimal::from_str(&row.quantity)
        .map_err(|_| CryptofolioError::InvalidAmount(row.quantity.clone()))?;

    // Parse optional fields
    let price_usd = row.price_usd
        .filter(|s| !s.is_empty())
        .map(|s| Decimal::from_str(&s))
        .transpose()
        .map_err(|_| CryptofolioError::InvalidAmount("price_usd".to_string()))?;

    let fee = row.fee
        .filter(|s| !s.is_empty())
        .map(|s| Decimal::from_str(&s))
        .transpose()
        .map_err(|_| CryptofolioError::InvalidAmount("fee".to_string()))?;

    let to_quantity = row.to_quantity
        .filter(|s| !s.is_empty())
        .map(|s| Decimal::from_str(&s))
        .transpose()
        .map_err(|_| CryptofolioError::InvalidAmount("to_quantity".to_string()))?;

    // Update holdings based on transaction type
    match tx_type {
        TransactionType::Buy | TransactionType::Receive => {
            holding_repo.add_quantity(account_id, &row.asset, quantity, price_usd).await?;
        }
        TransactionType::Sell => {
            holding_repo.remove_quantity(account_id, &row.asset, quantity).await?;
        }
        TransactionType::TransferIn => {
            holding_repo.add_quantity(account_id, &row.asset, quantity, price_usd).await?;
        }
        TransactionType::TransferOut => {
            holding_repo.remove_quantity(account_id, &row.asset, quantity).await?;
        }
        TransactionType::Swap => {
            if let (Some(to_asset), Some(to_qty)) = (&row.to_asset, to_quantity) {
                holding_repo.remove_quantity(account_id, &row.asset, quantity).await?;
                holding_repo.add_quantity(account_id, to_asset, to_qty, None).await?;
            }
        }
        _ => {}
    }

    // Build transaction record
    let tx = Transaction {
        id: 0,
        tx_type,
        from_account_id: match tx_type {
            TransactionType::Sell | TransactionType::TransferOut | TransactionType::Swap => {
                Some(account_id.to_string())
            }
            _ => None,
        },
        from_asset: match tx_type {
            TransactionType::Sell | TransactionType::TransferOut | TransactionType::Swap => {
                Some(row.asset.to_uppercase())
            }
            _ => None,
        },
        from_quantity: match tx_type {
            TransactionType::Sell | TransactionType::TransferOut | TransactionType::Swap => {
                Some(quantity)
            }
            _ => None,
        },
        to_account_id: match tx_type {
            TransactionType::Buy | TransactionType::Receive | TransactionType::TransferIn | TransactionType::Swap => {
                Some(account_id.to_string())
            }
            _ => None,
        },
        to_asset: match tx_type {
            TransactionType::Buy | TransactionType::Receive | TransactionType::TransferIn => {
                Some(row.asset.to_uppercase())
            }
            TransactionType::Swap => row.to_asset.map(|s| s.to_uppercase()),
            _ => None,
        },
        to_quantity: match tx_type {
            TransactionType::Buy | TransactionType::Receive | TransactionType::TransferIn => {
                Some(quantity)
            }
            TransactionType::Swap => to_quantity,
            _ => None,
        },
        price_usd,
        price_currency: None,
        price_amount: None,
        exchange_rate: None,
        exchange_rate_pair: None,
        fee,
        fee_asset: row.fee_asset,
        external_id: None,
        notes: row.notes,
        timestamp,
        created_at: Utc::now(),
    };

    tx_repo.insert(&tx).await?;

    Ok(())
}
