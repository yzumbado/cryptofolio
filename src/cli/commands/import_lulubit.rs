//! `cryptofolio import-lulubit` — ingest a Lulubit statement CSV as ledger
//! transactions (fiat on/off-ramp + direct USDT receipts).

use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

use crate::cli::GlobalOptions;
use crate::core::account::{Account, AccountConfig, AccountType};
use crate::db::AccountRepository;
use crate::error::{CryptofolioError, Result};
use crate::exchange::lulubit::{ImportResult, LulubitImporter, LulubitOp};

/// CSV columns: id,date,op,from_asset,from_quantity,to_asset,to_quantity,price_usd,note
pub async fn handle_import_lulubit_command(
    file: String,
    account: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_id = resolve_account(pool, account.as_deref().unwrap_or("Lulubit")).await?;

    let path = Path::new(&file);
    let mut reader = csv::Reader::from_path(path)?;
    let importer = LulubitImporter::new(pool);

    let mut created = 0usize;
    let mut skipped = 0usize;

    for record in reader.records() {
        let rec = record?;
        let id = rec.get(0).unwrap_or("").trim().to_string();
        let date = parse_date(rec.get(1).unwrap_or(""))?;
        let op = rec.get(2).unwrap_or("").trim();
        let from_asset = rec.get(3).unwrap_or("").trim().to_string();
        let from_qty = rec.get(4).unwrap_or("").trim();
        let to_asset = rec.get(5).unwrap_or("").trim().to_string();
        let to_qty = rec.get(6).unwrap_or("").trim();
        let price = rec.get(7).unwrap_or("").trim();
        let note = rec.get(8).unwrap_or("").trim().to_string();
        let to_account_name = rec.get(9).unwrap_or("").trim().to_string();

        if id.is_empty() {
            return Err(CryptofolioError::InvalidInput(
                "Lulubit CSV row missing id (column 1)".to_string(),
            ));
        }

        let op = match op {
            "receive" => LulubitOp::Receive {
                asset: to_asset,
                quantity: dec(to_qty)?,
                price_usd: dec(price)?,
                date,
            },
            "swap" => LulubitOp::Swap {
                from_asset,
                from_quantity: dec(from_qty)?,
                to_asset,
                to_quantity: dec(to_qty)?,
                date,
            },
            "dispose" => LulubitOp::Dispose {
                asset: from_asset,
                quantity: dec(from_qty)?,
                price_usd: dec(price)?,
                date,
            },
            "transfer" => {
                let to_id = if to_account_name.is_empty() {
                    return Err(CryptofolioError::InvalidInput(
                        "transfer op requires to_account (column 10)".to_string(),
                    ));
                } else {
                    resolve_account(pool, &to_account_name).await?
                };
                LulubitOp::Transfer {
                    from_account: account_id.clone(),
                    to_account: to_id,
                    asset: from_asset,
                    quantity: dec(from_qty)?,
                    date,
                }
            }
            "send" => LulubitOp::Send {
                asset: from_asset,
                quantity: dec(from_qty)?,
                date,
            },
            other => {
                return Err(CryptofolioError::InvalidInput(format!(
                    "unknown Lulubit op '{}' (expected receive|swap|dispose|transfer|send)",
                    other
                )))
            }
        };

        let external_id = format!("lulubit-{}", id);
        match importer
            .import(&account_id, &external_id, Some(&note), op)
            .await?
        {
            ImportResult::Created(_) => created += 1,
            ImportResult::Skipped => skipped += 1,
        }
    }

    if !opts.quiet {
        println!(
            "Lulubit import complete: {} created, {} skipped (duplicates).",
            created, skipped
        );
    }
    Ok(())
}

/// Find an account by name, or create it (with an `on-ramp` category) if absent.
async fn resolve_account(pool: &SqlitePool, name: &str) -> Result<String> {
    let repo = AccountRepository::new(pool);
    if let Some(acc) = repo.get_account(name).await? {
        return Ok(acc.id);
    }

    // The schema seeds an 'on-ramp' category (id 'on-ramp', name 'On-Ramp').
    let cat_id = if let Some(c) = repo.get_category("on-ramp").await? {
        c.id
    } else {
        repo.create_category("on-ramp", "On-Ramp").await?;
        "on-ramp".to_string()
    };

    let account = Account {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        category_id: cat_id,
        account_type: AccountType::Exchange,
        config: AccountConfig { is_testnet: false },
        sync_enabled: false,
        created_at: Utc::now(),
    };
    repo.create_account(&account).await?;
    Ok(account.id)
}

fn parse_date(s: &str) -> Result<chrono::DateTime<Utc>> {
    let d = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| CryptofolioError::InvalidInput(format!("invalid date '{}'", s)))?;
    let midnight = d
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| CryptofolioError::InvalidInput(format!("invalid date '{}'", s)))?;
    Ok(Utc.from_utc_datetime(&midnight))
}

fn dec(s: &str) -> Result<Decimal> {
    Decimal::from_str(s.trim())
        .map_err(|_| CryptofolioError::InvalidInput(format!("invalid decimal '{}'", s)))
}
