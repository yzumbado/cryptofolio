use crate::blockchain;
use crate::cli::output::{info, success};
use crate::cli::{GlobalOptions, WalletCommands};
use crate::core::account::{Account, AccountConfig, AccountType, WalletAddress};
use crate::db::accounts::AccountRepository;
use crate::error::{CryptofolioError, Result};
use chrono::Utc;
use colored::Colorize;
use sqlx::SqlitePool;

pub async fn handle_wallet_command(
    command: WalletCommands,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    match command {
        WalletCommands::Add {
            name,
            blockchain,
            address,
            xpub,
            derivation_path,
            address_type,
            label,
        } => {
            handle_wallet_add(
                name,
                blockchain,
                address,
                xpub,
                derivation_path,
                address_type,
                label,
                pool,
                opts,
            )
            .await
        }

        WalletCommands::List { blockchain } => handle_wallet_list(blockchain, pool, opts).await,

        WalletCommands::Show { name } => handle_wallet_show(name, pool, opts).await,

        WalletCommands::Sync {
            name,
            all,
            import_history,
            use_local_node,
        } => handle_wallet_sync(name, all, import_history, use_local_node, pool, opts).await,

        WalletCommands::Remove { name, yes } => handle_wallet_remove(name, yes, pool, opts).await,
    }
}

async fn handle_wallet_add(
    name: String,
    blockchain: String,
    address: Option<String>,
    xpub: Option<String>,
    derivation_path: Option<String>,
    address_type: Option<String>,
    label: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_repo = AccountRepository::new(pool);

    // Validate input
    if address.is_none() && xpub.is_none() {
        return Err(CryptofolioError::Other(
            "Either --address or --xpub must be provided".to_string(),
        ));
    }

    // Validate address matches the blockchain
    if let Some(ref addr) = address {
        validate_address_for_blockchain(addr, &blockchain)?;
    }

    // Validate xpub for Bitcoin
    if let Some(ref xpub_val) = xpub {
        if !blockchain.eq_ignore_ascii_case("bitcoin") {
            return Err(CryptofolioError::Other(
                "xpub is only supported for Bitcoin blockchain".to_string(),
            ));
        }
        blockchain::bitcoin::validate_xpub(xpub_val)?;
    }

    // Create or get the account for this wallet
    let account_id = name.to_lowercase().replace(" ", "-");

    // Check if account already exists
    if account_repo.get_account(&name).await?.is_some() {
        return Err(CryptofolioError::Other(format!(
            "Wallet '{}' already exists",
            name
        )));
    }

    // Check for duplicate address
    if let Some(ref addr) = address {
        let all_accounts = account_repo.list_accounts().await?;
        for acc in all_accounts {
            let addresses = account_repo.list_addresses(&acc.id).await?;
            if addresses
                .iter()
                .any(|a| a.address == *addr && a.blockchain.eq_ignore_ascii_case(&blockchain))
            {
                return Err(CryptofolioError::Other(format!(
                    "Address already exists in wallet '{}'",
                    acc.name
                )));
            }
        }
    }

    // Create the account
    let account = Account {
        id: account_id.clone(),
        name: name.clone(),
        category_id: "hot-wallets".to_string(),
        account_type: if xpub.is_some() {
            AccountType::HardwareWallet
        } else {
            AccountType::SoftwareWallet
        },
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };

    account_repo.create_account(&account).await?;

    // Detect network (mainnet or testnet)
    let network = if blockchain.eq_ignore_ascii_case("bitcoin") {
        if let Some(ref addr) = address {
            if blockchain::bitcoin::is_testnet_address(addr) {
                Some("testnet")
            } else {
                Some("mainnet")
            }
        } else if let Some(ref xpub_val) = xpub {
            if blockchain::bitcoin::is_testnet_xpub(xpub_val) {
                Some("testnet")
            } else {
                Some("mainnet")
            }
        } else {
            Some("mainnet")
        }
    } else {
        Some("mainnet") // Default to mainnet for other chains
    };

    // Add the wallet address
    if let Some(addr) = address {
        account_repo
            .add_address_with_xpub(
                &account_id,
                &blockchain,
                &addr,
                label.as_deref(),
                None,
                None,
                address_type.as_deref(),
                network,
            )
            .await?;

        let network_label = if network == Some("testnet") {
            " [TESTNET]"
        } else {
            ""
        };

        if opts.json {
            println!(
                r#"{{"success": true, "wallet": "{}", "blockchain": "{}", "address": "{}", "network": "{}"}}"#,
                name,
                blockchain,
                addr,
                network.unwrap_or("mainnet")
            );
        } else {
            success(&format!(
                "✓ Added wallet '{}' ({} address{})",
                name, blockchain, network_label
            ));
            info(&format!("  Address: {}", addr));
            if network == Some("testnet") {
                info("  ⚠️  This is a TESTNET address");
            }
        }
    } else if let Some(xpub_val) = xpub {
        // For HD wallets, we'll derive the first address (for now just store xpub)
        // TODO: Implement xpub derivation
        account_repo
            .add_address_with_xpub(
                &account_id,
                &blockchain,
                &xpub_val[..20], // Temporary: use truncated xpub as placeholder
                label.as_deref(),
                Some(&xpub_val),
                derivation_path.as_deref(),
                address_type.as_deref(),
                network,
            )
            .await?;

        let network_label = if network == Some("testnet") {
            " [TESTNET]"
        } else {
            ""
        };

        if opts.json {
            println!(
                r#"{{"success": true, "wallet": "{}", "blockchain": "{}", "xpub": "{}...", "network": "{}"}}"#,
                name,
                blockchain,
                &xpub_val[..20],
                network.unwrap_or("mainnet")
            );
        } else {
            success(&format!(
                "✓ Added HD wallet '{}' ({} xpub{})",
                name, blockchain, network_label
            ));
            info(&format!("  xpub: {}...", &xpub_val[..20]));
            if let Some(path) = derivation_path {
                info(&format!("  Derivation path: {}", path));
            }
            if network == Some("testnet") {
                info("  ⚠️  This is a TESTNET xpub");
            }
        }
    }

    Ok(())
}

async fn handle_wallet_list(
    blockchain: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_repo = AccountRepository::new(pool);
    let accounts = account_repo.list_accounts().await?;

    // Filter wallet accounts
    let wallet_accounts: Vec<_> = accounts
        .into_iter()
        .filter(|a| {
            matches!(
                a.account_type,
                AccountType::SoftwareWallet | AccountType::HardwareWallet
            )
        })
        .collect();

    if wallet_accounts.is_empty() {
        if !opts.quiet {
            info("No wallets found. Add one with: cryptofolio wallet add");
        }
        return Ok(());
    }

    if opts.json {
        println!("[");
        for (i, account) in wallet_accounts.iter().enumerate() {
            let addresses = account_repo.list_addresses(&account.id).await?;
            let filtered_addresses: Vec<_> = if let Some(ref bc) = blockchain {
                addresses
                    .into_iter()
                    .filter(|addr| addr.blockchain.eq_ignore_ascii_case(bc))
                    .collect()
            } else {
                addresses
            };

            if !filtered_addresses.is_empty() {
                println!("  {{");
                println!(r#"    "name": "{}","#, account.name);
                println!(r#"    "id": "{}","#, account.id);
                println!(r#"    "type": "{}","#, account.account_type.as_str());
                println!(r#"    "addresses": ["#);
                for (j, addr) in filtered_addresses.iter().enumerate() {
                    println!("      {{");
                    println!(r#"        "blockchain": "{}","#, addr.blockchain);
                    println!(r#"        "address": "{}","#, addr.address);
                    if let Some(ref label) = addr.label {
                        println!(r#"        "label": "{}","#, label);
                    }
                    if let Some(ref xpub) = addr.xpub {
                        println!(r#"        "xpub": "{}","#, xpub);
                    }
                    println!(
                        "      }}{}",
                        if j < filtered_addresses.len() - 1 {
                            ","
                        } else {
                            ""
                        }
                    );
                }
                println!("    ]");
                println!(
                    "  }}{}",
                    if i < wallet_accounts.len() - 1 {
                        ","
                    } else {
                        ""
                    }
                );
            }
        }
        println!("]");
    } else {
        println!("{}", "━".repeat(80).bright_black());
        println!("{}", "Wallets".bold());
        println!("{}", "━".repeat(80).bright_black());

        for account in &wallet_accounts {
            let addresses = account_repo.list_addresses(&account.id).await?;
            let filtered_addresses: Vec<_> = if let Some(ref bc) = blockchain {
                addresses
                    .into_iter()
                    .filter(|addr| addr.blockchain.eq_ignore_ascii_case(bc))
                    .collect()
            } else {
                addresses
            };

            if filtered_addresses.is_empty() {
                continue;
            }

            println!(
                "\n{} ({})",
                account.name.bold(),
                account.account_type.as_str()
            );
            for addr in filtered_addresses {
                let blockchain_icon = match addr.blockchain.as_str() {
                    "bitcoin" => "₿",
                    "ethereum" => "Ξ",
                    "solana" => "◎",
                    "cardano" => "₳",
                    _ => "•",
                };

                print!("  {} {} {}", blockchain_icon, addr.blockchain, addr.address);
                if let Some(label) = addr.label {
                    print!(" ({})", label.dimmed());
                }
                if addr.xpub.is_some() {
                    print!(" {}", "[HD]".bright_blue());
                }
                if addr.network.as_deref() == Some("testnet") {
                    print!(" {}", "[TESTNET]".yellow());
                }
                println!();
            }
        }
        println!();
    }

    Ok(())
}

async fn handle_wallet_show(
    _name: String,
    _pool: &SqlitePool,
    _opts: &GlobalOptions,
) -> Result<()> {
    // TODO: Implement wallet show
    info("Wallet show command not yet implemented");
    Ok(())
}

async fn handle_wallet_sync(
    name: Option<String>,
    all: bool,
    import_history: bool,
    _use_local_node: bool,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_repo = AccountRepository::new(pool);

    // Get wallets to sync
    let wallets = if all {
        account_repo
            .list_accounts()
            .await?
            .into_iter()
            .filter(|a| {
                matches!(
                    a.account_type,
                    AccountType::SoftwareWallet | AccountType::HardwareWallet
                )
            })
            .collect::<Vec<_>>()
    } else if let Some(wallet_name) = name {
        let account = account_repo
            .get_account(&wallet_name)
            .await?
            .ok_or_else(|| CryptofolioError::AccountNotFound(wallet_name))?;
        vec![account]
    } else {
        return Err(CryptofolioError::Other(
            "Specify wallet name or use --all".to_string(),
        ));
    };

    if wallets.is_empty() {
        info("No wallets to sync");
        return Ok(());
    }

    // Sync each wallet
    for wallet in wallets {
        let addresses = account_repo.list_addresses(&wallet.id).await?;

        if addresses.is_empty() {
            continue;
        }

        for addr in addresses {
            let is_testnet = addr.network.as_deref() == Some("testnet");

            if !opts.quiet {
                println!("\nSyncing {} ({})...", wallet.name, addr.blockchain);
                if is_testnet {
                    info("  Using testnet API");
                }
            }

            // Handle different blockchains
            if addr.blockchain.eq_ignore_ascii_case("bitcoin") {
                sync_bitcoin_wallet(&wallet.name, &addr, is_testnet, import_history, opts).await?;
            } else if addr.blockchain.eq_ignore_ascii_case("ethereum") {
                sync_ethereum_wallet(&wallet.name, &addr, is_testnet, import_history, opts).await?;
            } else if addr.blockchain.eq_ignore_ascii_case("cardano") {
                sync_cardano_wallet(&wallet.name, &addr, is_testnet, import_history, opts).await?;
            } else {
                if !opts.quiet {
                    println!(
                        "  ⚠️  Blockchain {} not yet supported for sync",
                        addr.blockchain
                    );
                }
            }
        }
    }

    Ok(())
}

async fn sync_bitcoin_wallet(
    wallet_name: &str,
    addr: &WalletAddress,
    is_testnet: bool,
    import_history: bool,
    opts: &GlobalOptions,
) -> Result<()> {
    // Create Blockstream client
    let client = blockchain::bitcoin::BlockstreamClient::new(is_testnet);

    // Fetch address info
    match client.get_address_info(&addr.address).await {
        Ok(addr_info) => {
            if opts.json {
                println!(
                    r#"{{"wallet":"{}","address":"{}","balance":"{}","tx_count":{}}}"#,
                    wallet_name, addr.address, addr_info.balance, addr_info.tx_count
                );
            } else {
                success(&format!(
                    "✓ Synced {} balance: {:.8}",
                    addr.blockchain.to_uppercase(),
                    addr_info.balance
                ));
                info(&format!("  Transactions: {}", addr_info.tx_count));
                info(&format!(
                    "  Total received: {:.8} BTC",
                    addr_info.total_received
                ));
                info(&format!("  Total sent: {:.8} BTC", addr_info.total_sent));
            }

            // Import transaction history if requested
            if import_history {
                match client.get_transactions(&addr.address).await {
                    Ok(txs) => {
                        if !opts.quiet {
                            success(&format!("✓ Imported {} transactions", txs.len()));
                        }
                        // TODO: Save transactions to database
                    }
                    Err(e) => {
                        if !opts.quiet {
                            println!("  ⚠️  Failed to fetch transactions: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            if !opts.quiet {
                println!("  ❌ Failed to sync {}: {}", addr.address, e);
            }
        }
    }

    Ok(())
}

async fn sync_ethereum_wallet(
    wallet_name: &str,
    addr: &WalletAddress,
    is_testnet: bool,
    import_history: bool,
    opts: &GlobalOptions,
) -> Result<()> {
    // Create Etherscan client (no API key for now)
    let client = blockchain::ethereum::EtherscanClient::new(is_testnet, None);

    // Fetch address info (balance + tokens)
    match client.get_address_info(&addr.address).await {
        Ok(addr_info) => {
            if opts.json {
                println!(
                    r#"{{"wallet":"{}","address":"{}","balance":"{}","tokens":{}}}"#,
                    wallet_name,
                    addr.address,
                    addr_info.balance,
                    addr_info.tokens.len()
                );
            } else {
                success(&format!("✓ Synced ETH balance: {:.4}", addr_info.balance));

                // Show token balances
                if addr_info.tokens.is_empty() {
                    info("  No tokens found");
                } else {
                    success(&format!("✓ Found {} tokens", addr_info.tokens.len()));
                    for token in &addr_info.tokens {
                        info(&format!("  {}: {:.2}", token.symbol, token.balance));
                    }
                }
            }

            // Import transaction history if requested
            if import_history {
                match client.get_transactions(&addr.address).await {
                    Ok(txs) => {
                        if !opts.quiet {
                            success(&format!("✓ Imported {} transactions", txs.len()));
                        }
                        // TODO: Save transactions to database
                    }
                    Err(e) => {
                        if !opts.quiet {
                            println!("  ⚠️  Failed to fetch transactions: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            if !opts.quiet {
                println!("  ❌ Failed to sync {}: {}", addr.address, e);
            }
        }
    }

    Ok(())
}

async fn sync_cardano_wallet(
    wallet_name: &str,
    addr: &WalletAddress,
    is_testnet: bool,
    import_history: bool,
    opts: &GlobalOptions,
) -> Result<()> {
    // Load config and get Blockfrost API key
    let config = crate::config::AppConfig::load()?;
    let api_key = config.get_blockfrost_api_key(is_testnet, addr.network.as_deref());

    // Check if API key is available
    if api_key.is_none() {
        return Err(crate::error::CryptofolioError::Config(
            "Blockfrost API key not configured. Set it with:\n  cryptofolio config set blockfrost.preprod_api_key <your-key>\n  or set environment variable: BLOCKFROST_API_KEY".to_string()
        ));
    }

    // Create Blockfrost client with API key
    let client = blockchain::cardano::BlockfrostClient::new(is_testnet, api_key);

    // Fetch address info (balance + native tokens + stake info)
    match client.get_address_info(&addr.address).await {
        Ok(addr_info) => {
            if opts.json {
                println!(
                    r#"{{"wallet":"{}","address":"{}","balance":"{}","tokens":{},"delegated":{}}}"#,
                    wallet_name,
                    addr.address,
                    addr_info.balance,
                    addr_info.tokens.len(),
                    addr_info.stake_pool.is_some()
                );
            } else {
                success(&format!("✓ Synced ADA balance: {:.1}", addr_info.balance));

                // Show native token balances
                if addr_info.tokens.is_empty() {
                    info("  No tokens found");
                } else {
                    success(&format!("✓ Found {} tokens", addr_info.tokens.len()));
                    for token in &addr_info.tokens {
                        info(&format!("  {}: {:.2}", token.display_name, token.balance));
                    }
                }

                // Show stake delegation if any
                if let Some(pool) = &addr_info.stake_pool {
                    info(&format!("  Delegated to: {}", pool.ticker));
                }
            }

            // Import transaction history if requested
            if import_history {
                match client.get_transactions(&addr.address).await {
                    Ok(txs) => {
                        if !opts.quiet {
                            success(&format!("✓ Imported {} transactions", txs.len()));
                        }
                        // TODO: Save transactions to database
                    }
                    Err(e) => {
                        if !opts.quiet {
                            println!("  ⚠️  Failed to fetch transactions: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            if !opts.quiet {
                println!("  ❌ Failed to sync {}: {}", addr.address, e);
            }
        }
    }

    Ok(())
}

async fn handle_wallet_remove(
    name: String,
    yes: bool,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let account_repo = AccountRepository::new(pool);

    // Check if wallet exists
    let account = account_repo.get_account(&name).await?;
    if account.is_none() {
        return Err(CryptofolioError::AccountNotFound(name));
    }

    // Confirm deletion
    if !yes && !opts.quiet {
        println!("Are you sure you want to remove wallet '{}'? (y/N): ", name);
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            info("Cancelled");
            return Ok(());
        }
    }

    // Delete the account (cascades to addresses)
    account_repo.delete_account(&name).await?;

    if opts.json {
        println!(r#"{{"success": true, "removed": "{}"}}"#, name);
    } else {
        success(&format!("✓ Removed wallet '{}'", name));
    }

    Ok(())
}

/// Validate that an address matches the specified blockchain
fn validate_address_for_blockchain(address: &str, blockchain: &str) -> Result<()> {
    match blockchain.to_lowercase().as_str() {
        "bitcoin" => {
            blockchain::validate_bitcoin_address(address)?;
            Ok(())
        }
        "ethereum" => {
            blockchain::validate_ethereum_address(address)?;
            Ok(())
        }
        "solana" => {
            // TODO: Implement Solana address validation
            validate_solana_address(address)
        }
        "cardano" => {
            // TODO: Implement Cardano address validation
            validate_cardano_address(address)
        }
        _ => Err(CryptofolioError::Other(format!(
            "Unsupported blockchain: {}. Supported: bitcoin, ethereum, solana, cardano",
            blockchain
        ))),
    }
}

/// Basic Solana address validation (placeholder)
fn validate_solana_address(address: &str) -> Result<()> {
    // Solana addresses are base58 encoded, 32-44 characters
    if address.is_empty() {
        return Err(CryptofolioError::Other("Empty Solana address".to_string()));
    }

    if address.len() < 32 || address.len() > 44 {
        return Err(CryptofolioError::Other(
            "Invalid Solana address: must be 32-44 characters".to_string(),
        ));
    }

    // Base58 validation (no 0, O, I, l)
    if !address
        .chars()
        .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
    {
        return Err(CryptofolioError::Other(
            "Invalid Solana address: invalid base58 characters".to_string(),
        ));
    }

    Ok(())
}

/// Basic Cardano address validation (placeholder)
fn validate_cardano_address(address: &str) -> Result<()> {
    // Cardano addresses start with 'addr1' (mainnet) or 'addr_test1' (testnet)
    if address.is_empty() {
        return Err(CryptofolioError::Other("Empty Cardano address".to_string()));
    }

    if !address.starts_with("addr1") && !address.starts_with("addr_test1") {
        return Err(CryptofolioError::Other(
            "Invalid Cardano address: must start with 'addr1' or 'addr_test1'".to_string(),
        ));
    }

    // Cardano addresses are typically 100+ characters
    if address.len() < 50 {
        return Err(CryptofolioError::Other(
            "Invalid Cardano address: too short".to_string(),
        ));
    }

    Ok(())
}
