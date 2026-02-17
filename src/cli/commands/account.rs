use chrono::Utc;
use colored::Colorize;
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::cli::{AccountCommands, AccountTypeArg, AddressCommands, GlobalOptions};
use crate::cli::output::{print_header, print_kv, print_row, success, suggest_next};
use crate::core::account::{Account, AccountConfig, AccountType};
use crate::db::AccountRepository;
use crate::error::{CryptofolioError, Result};

#[derive(Serialize)]
struct AccountListOutput {
    name: String,
    account_type: String,
    category: String,
    sync_enabled: bool,
    is_testnet: bool,
}

#[derive(Serialize)]
struct AccountShowOutput {
    name: String,
    account_type: String,
    category: String,
    is_testnet: bool,
    sync_enabled: bool,
    created_at: String,
    addresses: Vec<AddressOutput>,
}

#[derive(Serialize)]
struct AddressOutput {
    blockchain: String,
    address: String,
    label: Option<String>,
}

pub async fn handle_account_command(command: AccountCommands, pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let _ = opts; // Will be used for JSON output
    let repo = AccountRepository::new(pool);

    match command {
        AccountCommands::List => {
            let accounts = repo.list_accounts().await?;

            if accounts.is_empty() {
                if opts.json {
                    println!("[]");
                } else {
                    println!("No accounts configured. Use 'cryptofolio account add' to create one.");
                }
                return Ok(());
            }

            if opts.json {
                let mut output = Vec::new();
                for account in accounts {
                    let category = repo.get_category(&account.category_id).await?;
                    let category_name = category.map(|c| c.name).unwrap_or_else(|| "-".to_string());

                    output.push(AccountListOutput {
                        name: account.name.clone(),
                        account_type: account.account_type.display_name().to_string(),
                        category: category_name,
                        sync_enabled: account.sync_enabled,
                        is_testnet: account.config.is_testnet,
                    });
                }
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
            } else {
                print_header(&[("Name", 20), ("Type", 18), ("Category", 15), ("Sync", 6)]);

                for account in accounts {
                    let category = repo.get_category(&account.category_id).await?;
                    let category_name = category.map(|c| c.name).unwrap_or_else(|| "-".to_string());

                    let sync_status = if account.sync_enabled {
                        "Yes".green().to_string()
                    } else {
                        "No".dimmed().to_string()
                    };

                    let type_display = if account.config.is_testnet {
                        format!("{} (testnet)", account.account_type.display_name())
                    } else {
                        account.account_type.display_name().to_string()
                    };

                    print_row(&[
                        (&account.name, 20),
                        (&type_display, 18),
                        (&category_name, 15),
                        (&sync_status, 6),
                    ]);
                }
            }
        }

        AccountCommands::Add {
            name,
            account_type,
            category,
            testnet,
            sync,
        } => {
            // Convert AccountTypeArg to AccountType
            let acc_type = match account_type {
                AccountTypeArg::Exchange => AccountType::Exchange,
                AccountTypeArg::HardwareWallet => AccountType::HardwareWallet,
                AccountTypeArg::SoftwareWallet => AccountType::SoftwareWallet,
                AccountTypeArg::CustodialService => AccountType::CustodialService,
            };

            // Find or validate category
            let cat = repo.get_category(&category).await?
                .or_else(|| None);

            let category_id = if let Some(c) = cat {
                c.id
            } else {
                // Try by name
                let cat_by_name = repo.get_category_by_name(&category).await?;
                if let Some(c) = cat_by_name {
                    c.id
                } else {
                    return Err(CryptofolioError::CategoryNotFound(category));
                }
            };

            let account = Account {
                id: Uuid::new_v4().to_string(),
                name: name.clone(),
                category_id,
                account_type: acc_type,
                config: AccountConfig {
                    is_testnet: testnet,
                },
                sync_enabled: sync,
                created_at: Utc::now(),
            };

            repo.create_account(&account).await?;

            let testnet_note = if testnet { " (testnet)" } else { "" };
            success(&format!("Account '{}'{} created successfully", name, testnet_note));

            // Suggest next steps
            if !opts.quiet {
                suggest_next(
                    &format!("cryptofolio holdings add BTC 1.0 --account \"{}\"", name),
                    "Add holdings to this account",
                );
            }
        }

        AccountCommands::Remove { name, yes } => {
            if !yes {
                // Show confirmation prompt
                println!("This will delete account '{}' and all its holdings.", name);
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

            repo.delete_account(&name).await?;
            success(&format!("Account '{}' removed", name));
        }

        AccountCommands::Show { name } => {
            let account = repo.get_account(&name).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(name.clone()))?;

            let category = repo.get_category(&account.category_id).await?;
            let addresses = repo.list_addresses(&account.id).await?;

            if opts.json {
                let output = AccountShowOutput {
                    name: account.name.clone(),
                    account_type: account.account_type.display_name().to_string(),
                    category: category.map(|c| c.name).unwrap_or_else(|| "-".to_string()),
                    is_testnet: account.config.is_testnet,
                    sync_enabled: account.sync_enabled,
                    created_at: account.created_at.to_rfc3339(),
                    addresses: addresses.iter().map(|a| AddressOutput {
                        blockchain: a.blockchain.clone(),
                        address: a.address.clone(),
                        label: a.label.clone(),
                    }).collect(),
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
            } else {
                println!();
                println!("{}", account.name.bold());
                println!();

                print_kv("Type", account.account_type.display_name());
                print_kv("Category", &category.map(|c| c.name).unwrap_or_else(|| "-".to_string()));
                print_kv("Testnet", if account.config.is_testnet { "Yes" } else { "No" });
                print_kv("Sync Enabled", if account.sync_enabled { "Yes" } else { "No" });
                print_kv("Created", &account.created_at.format("%Y-%m-%d %H:%M").to_string());

                if !addresses.is_empty() {
                    println!();
                    println!("{}", "Wallet Addresses:".bold());
                    for addr in addresses {
                        let label = addr.label.map(|l| format!(" ({})", l)).unwrap_or_default();
                        println!("  {} {}{}", addr.blockchain.dimmed(), addr.address, label);
                    }
                }

                println!();
            }
        }

        AccountCommands::Address { command } => {
            handle_address_command(command, pool).await?;
        }
    }

    Ok(())
}

async fn handle_address_command(command: AddressCommands, pool: &SqlitePool) -> Result<()> {
    let repo = AccountRepository::new(pool);

    match command {
        AddressCommands::Add {
            account,
            blockchain,
            address,
            label,
        } => {
            let acc = repo.get_account(&account).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            repo.add_address(&acc.id, &blockchain, &address, label.as_deref()).await?;
            success(&format!("Address added to '{}'", account));
        }

        AddressCommands::List { account } => {
            let acc = repo.get_account(&account).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            let addresses = repo.list_addresses(&acc.id).await?;

            if addresses.is_empty() {
                println!("No addresses configured for '{}'", account);
                return Ok(());
            }

            print_header(&[("Blockchain", 12), ("Address", 45), ("Label", 15)]);

            for addr in addresses {
                print_row(&[
                    (&addr.blockchain, 12),
                    (&addr.address, 45),
                    (&addr.label.unwrap_or_default(), 15),
                ]);
            }
        }

        AddressCommands::Remove { account, address } => {
            let acc = repo.get_account(&account).await?
                .ok_or_else(|| CryptofolioError::AccountNotFound(account.clone()))?;

            repo.remove_address(&acc.id, &address).await?;
            success(&format!("Address removed from '{}'", account));
        }
    }

    Ok(())
}
