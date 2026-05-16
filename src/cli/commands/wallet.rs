use std::sync::Arc;

use crate::blockchain;
use crate::blockchain::provider::{PrivacyLevel, PrivacyMode, ProviderRegistry};
use crate::blockchain::sync::{SyncEngine, SyncOptions};
use crate::blockchain::types::Chain;
use crate::cli::output::{info, success};
use crate::cli::{GlobalOptions, WalletCommands};
use crate::core::account::{Account, AccountConfig, AccountType, WalletAddress};
use crate::core::transaction::{Transaction, TransactionType};
use crate::db::accounts::AccountRepository;
use crate::db::holdings::HoldingRepository;
use crate::db::transactions::TransactionRepository;
use crate::error::{CryptofolioError, Result};
use chrono::{TimeZone, Utc};
use colored::Colorize;
use rust_decimal::Decimal;
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

    // Security: reject private key material before touching the DB
    if let Some(ref addr) = address {
        blockchain::reject_if_private_key(addr)?;
    }
    if let Some(ref xpub_val) = xpub {
        blockchain::reject_if_private_key(xpub_val)?;
    }
    if let Some(ref lbl) = label {
        blockchain::reject_if_private_key(lbl)?;
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
        // Derive the first 20 external-chain receiving addresses (BIP44 gap limit)
        let is_testnet_net = network == Some("testnet");
        // Allow explicit --type taproot to force BIP-86 P2TR derivation from a plain xpub
        let force_type = match address_type.as_deref() {
            Some("taproot") => Some(blockchain::bitcoin::XpubAddressType::Taproot),
            Some("native_segwit") => Some(blockchain::bitcoin::XpubAddressType::NativeSegwit),
            Some("segwit") => Some(blockchain::bitcoin::XpubAddressType::WrappedSegwit),
            Some("legacy") => Some(blockchain::bitcoin::XpubAddressType::Legacy),
            _ => None,
        };
        let derived = blockchain::bitcoin::derive_addresses_with_type(
            &xpub_val,
            is_testnet_net,
            20,
            force_type,
        )
        .map_err(|e| CryptofolioError::Other(format!("xpub derivation failed: {}", e)))?;

        // Secure xpub storage: macOS Keychain when available, DB column otherwise.
        let xpub_for_db = store_xpub_secure(&account_id, &xpub_val)?;

        // Store each derived address, all sharing the same xpub reference
        for (addr_str, idx) in &derived {
            let child_path = derivation_path
                .as_ref()
                .map(|p| format!("{}/0/{}", p, idx))
                .unwrap_or_else(|| format!("m/0/{}", idx));

            account_repo
                .add_address_with_xpub(
                    &account_id,
                    &blockchain,
                    addr_str,
                    label.as_deref(),
                    xpub_for_db.as_deref(),
                    Some(&child_path),
                    address_type.as_deref(),
                    network,
                )
                .await?;
        }

        let network_label = if network == Some("testnet") {
            " [TESTNET]"
        } else {
            ""
        };
        let xpub_preview = &xpub_val[..xpub_val.len().min(20)];

        if opts.json {
            println!(
                r#"{{"success": true, "wallet": "{}", "blockchain": "{}", "xpub": "{}...", "network": "{}", "derived_addresses": {}}}"#,
                name,
                blockchain,
                xpub_preview,
                network.unwrap_or("mainnet"),
                derived.len()
            );
        } else {
            success(&format!(
                "✓ Added HD wallet '{}' ({} xpub{}, {} addresses derived)",
                name,
                blockchain,
                network_label,
                derived.len()
            ));
            info(&format!("  xpub: {}...", xpub_preview));
            if let Some(path) = &derivation_path {
                info(&format!("  Derivation path: {}", path));
            }
            for (addr_str, idx) in &derived {
                info(&format!("  [{}] {}", idx, addr_str));
            }
            if network == Some("testnet") {
                info("  ⚠️  This is a TESTNET xpub");
            }
        }
    }

    Ok(())
}

/// Store an xpub securely.
///
/// - **macOS**: writes to macOS Keychain under `cryptofolio:xpub:{account_id}`.
///   Returns `None` so the DB column stays empty (no plaintext on disk).
/// - **Other platforms**: returns `Some(xpub)` — stored in the DB column as
///   before (Keychain support for Linux/Windows is planned for v1.0).
fn store_xpub_secure(account_id: &str, xpub: &str) -> Result<Option<String>> {
    #[cfg(target_os = "macos")]
    {
        use crate::config::keychain::get_keychain;
        let keychain = get_keychain();
        let key = format!("cryptofolio:xpub:{}", account_id);
        keychain.store(&key, xpub).map_err(|e| {
            CryptofolioError::Other(format!("Failed to store xpub in macOS Keychain: {}", e))
        })?;
        Ok(None) // do not write xpub to DB
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = account_id;
        Ok(Some(xpub.to_string()))
    }
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
                    let has_extra = addr.label.is_some() || addr.xpub.is_some();
                    println!("      {{");
                    println!(r#"        "blockchain": "{}","#, addr.blockchain);
                    if has_extra {
                        println!(r#"        "address": "{}","#, addr.address);
                    } else {
                        println!(r#"        "address": "{}""#, addr.address);
                    }
                    if let Some(ref label) = addr.label {
                        let has_xpub = addr.xpub.is_some();
                        if has_xpub {
                            println!(r#"        "label": "{}","#, label);
                        } else {
                            println!(r#"        "label": "{}""#, label);
                        }
                    }
                    if let Some(ref xpub) = addr.xpub {
                        println!(r#"        "xpub": "{}""#, xpub);
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

async fn handle_wallet_show(name: String, pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let account_repo = AccountRepository::new(pool);
    let holding_repo = HoldingRepository::new(pool);

    let account = account_repo
        .get_account(&name)
        .await?
        .ok_or_else(|| CryptofolioError::AccountNotFound(name.clone()))?;

    let addresses = account_repo.list_addresses(&account.id).await?;
    let holdings = holding_repo.list_by_account(&account.id).await?;

    if opts.json {
        println!("{{");
        println!(r#"  "name": "{}","#, account.name);
        println!(r#"  "id": "{}","#, account.id);
        println!(r#"  "type": "{}","#, account.account_type.as_str());
        println!(
            r#"  "created_at": "{}","#,
            account.created_at.format("%Y-%m-%d")
        );
        println!(r#"  "addresses": ["#);
        for (i, addr) in addresses.iter().enumerate() {
            println!("    {{");
            println!(r#"      "blockchain": "{}","#, addr.blockchain);
            println!(r#"      "address": "{}","#, addr.address);
            println!(
                r#"      "network": "{}""#,
                addr.network.as_deref().unwrap_or("mainnet")
            );
            println!("    }}{}", if i < addresses.len() - 1 { "," } else { "" });
        }
        println!("  ],");
        println!(r#"  "holdings": ["#);
        for (i, h) in holdings.iter().enumerate() {
            println!("    {{");
            println!(r#"      "asset": "{}","#, h.asset);
            println!(r#"      "quantity": "{}""#, h.quantity);
            println!("    }}{}", if i < holdings.len() - 1 { "," } else { "" });
        }
        println!("  ]");
        println!("}}");
    } else {
        println!("{}", "━".repeat(80).bright_black());
        println!(
            "{} ({})",
            account.name.bold(),
            account.account_type.as_str()
        );
        println!("{}", "━".repeat(80).bright_black());
        info(&format!("  ID:      {}", account.id));
        info(&format!(
            "  Created: {}",
            account.created_at.format("%Y-%m-%d")
        ));

        println!("\n{}", "Addresses".bold());
        if addresses.is_empty() {
            info("  No addresses");
        } else {
            for addr in &addresses {
                let icon = match addr.blockchain.as_str() {
                    "bitcoin" => "₿",
                    "ethereum" => "Ξ",
                    "solana" => "◎",
                    "cardano" => "₳",
                    _ => "•",
                };
                let network_tag = if addr.network.as_deref() == Some("testnet") {
                    format!(" {}", "[TESTNET]".yellow())
                } else {
                    String::new()
                };
                let label_tag = addr
                    .label
                    .as_ref()
                    .map(|l| format!(" ({})", l.dimmed()))
                    .unwrap_or_default();
                println!(
                    "  {} {} {}{}{}",
                    icon, addr.blockchain, addr.address, network_tag, label_tag
                );
            }
        }

        println!("\n{}", "Holdings".bold());
        if holdings.is_empty() {
            info("  No holdings — run: cryptofolio wallet sync");
        } else {
            for h in &holdings {
                let cost = h
                    .avg_cost_basis
                    .map(|c| format!("  (avg cost ${:.2})", c))
                    .unwrap_or_default();
                println!("  {:8}  {}{}", h.asset, h.quantity, cost);
            }
        }
        println!();
    }

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

    // Build ProviderRegistry with public providers (Balanced mode by default)
    let mut registry = ProviderRegistry::new(PrivacyMode::Balanced);

    // Bitcoin — Blockstream (public)
    {
        use crate::blockchain::bitcoin::BlockstreamClient;
        registry.register(
            &Chain::Bitcoin,
            Arc::new(BlockstreamClient::new(false)),
            PrivacyLevel::Public,
        );
    }

    // Ethereum — Etherscan (public, optional API key from env)
    {
        use crate::blockchain::ethereum::EtherscanClient;
        let api_key = std::env::var("ETHERSCAN_API_KEY").ok();
        registry.register(
            &Chain::Ethereum,
            Arc::new(EtherscanClient::new(false, api_key)),
            PrivacyLevel::Public,
        );
    }

    // Cardano — Blockfrost (public, optional API key from env)
    {
        use crate::blockchain::cardano::BlockfrostClient;
        let api_key = std::env::var("BLOCKFROST_API_KEY").ok();
        registry.register(
            &Chain::Cardano,
            Arc::new(BlockfrostClient::new(false, api_key)),
            PrivacyLevel::Public,
        );
    }

    // Solana — user-configured RPC (skipped if not set)
    if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL") {
        use crate::blockchain::solana::SolanaRpcClient;
        registry.register(
            &Chain::Solana,
            Arc::new(SolanaRpcClient::new(rpc_url)),
            PrivacyLevel::Custom,
        );
    }

    let engine = SyncEngine::new(Arc::new(registry), pool.clone());

    // Collect all (address, chain) pairs across all wallets
    for wallet in &wallets {
        let addresses = account_repo.list_addresses(&wallet.id).await?;

        if addresses.is_empty() {
            continue;
        }

        if !opts.quiet {
            println!("\nSyncing wallet: {}", wallet.name);
        }

        // Parse each address into (String, Chain)
        let mut addr_chain_pairs: Vec<(String, Chain)> = Vec::new();
        for addr in &addresses {
            let chain: Chain = match addr.blockchain.to_lowercase().as_str() {
                "bitcoin" => Chain::Bitcoin,
                "ethereum" => Chain::Ethereum,
                "cardano" => Chain::Cardano,
                "solana" => Chain::Solana,
                other => {
                    if !opts.quiet {
                        println!("  ⚠️  Blockchain {} not yet supported for sync", other);
                    }
                    continue;
                }
            };
            addr_chain_pairs.push((addr.address.clone(), chain));
        }

        if addr_chain_pairs.is_empty() {
            continue;
        }

        let sync_opts = SyncOptions {
            full_history: import_history,
            ..Default::default()
        };

        let report = engine
            .sync_addresses(&wallet.id, addr_chain_pairs, sync_opts)
            .await;

        if !opts.quiet {
            println!(
                "  ✓ Synced {} addresses — {} balance updates, {} new transactions ({} ms)",
                report.addresses_synced,
                report.balances_updated,
                report.transactions_new,
                report.duration_ms,
            );
            for err in &report.errors {
                println!("  ⚠️  {}: {}", err.address, err.message);
            }
        }
    }

    Ok(())
}

async fn sync_bitcoin_wallet(
    account_id: &str,
    wallet_name: &str,
    addr: &WalletAddress,
    is_testnet: bool,
    import_history: bool,
    opts: &GlobalOptions,
    pool: &SqlitePool,
) -> Result<()> {
    // Create Blockstream client
    let client = blockchain::bitcoin::BlockstreamClient::new(is_testnet);

    // Fetch address info
    match client.get_address_info(&addr.address).await {
        Ok(addr_info) => {
            // Persist BTC holding
            let holding_repo = HoldingRepository::new(pool);
            holding_repo
                .set_quantity(account_id, "BTC", addr_info.balance, None)
                .await?;

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
                info("  ✓ Balance saved to portfolio");
            }

            // Import transaction history if requested
            if import_history {
                match client.fetch_transactions(&addr.address).await {
                    Ok(txs) => {
                        let tx_repo = TransactionRepository::new(pool);
                        let mut saved = 0usize;
                        for tx in &txs {
                            let external_id = format!("btc-tx-{}", tx.txid);
                            let exists: bool = sqlx::query_scalar(
                                "SELECT EXISTS(SELECT 1 FROM transactions WHERE external_id = ?)",
                            )
                            .bind(&external_id)
                            .fetch_one(pool)
                            .await?;

                            if exists {
                                continue;
                            }

                            // Determine direction: positive value = received
                            let (tx_type, from_id, to_id, quantity) = if tx.value >= Decimal::ZERO {
                                (
                                    TransactionType::TransferIn,
                                    None,
                                    Some(account_id.to_string()),
                                    tx.value,
                                )
                            } else {
                                (
                                    TransactionType::TransferOut,
                                    Some(account_id.to_string()),
                                    None,
                                    tx.value.abs(),
                                )
                            };

                            let timestamp = tx
                                .timestamp
                                .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
                                .unwrap_or_else(Utc::now);

                            let record = Transaction {
                                id: 0,
                                tx_type,
                                from_account_id: from_id,
                                from_asset: if tx_type == TransactionType::TransferOut {
                                    Some("BTC".to_string())
                                } else {
                                    None
                                },
                                from_quantity: if tx_type == TransactionType::TransferOut {
                                    Some(quantity)
                                } else {
                                    None
                                },
                                to_account_id: to_id,
                                to_asset: if tx_type == TransactionType::TransferIn {
                                    Some("BTC".to_string())
                                } else {
                                    None
                                },
                                to_quantity: if tx_type == TransactionType::TransferIn {
                                    Some(quantity)
                                } else {
                                    None
                                },
                                price_usd: None,
                                price_currency: None,
                                price_amount: None,
                                exchange_rate: None,
                                exchange_rate_pair: None,
                                fee: tx.fee,
                                fee_asset: tx.fee.map(|_| "BTC".to_string()),
                                external_id: Some(external_id),
                                notes: None,
                                timestamp,
                                created_at: Utc::now(),
                            };

                            tx_repo.insert(&record).await?;
                            saved += 1;
                        }
                        if !opts.quiet {
                            success(&format!(
                                "✓ Imported {} transactions ({} new)",
                                txs.len(),
                                saved
                            ));
                        }
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
    account_id: &str,
    wallet_name: &str,
    addr: &WalletAddress,
    is_testnet: bool,
    import_history: bool,
    opts: &GlobalOptions,
    pool: &SqlitePool,
) -> Result<()> {
    // Read Etherscan API key: env var → keychain → config file
    let api_key = crate::config::AppConfig::load()
        .ok()
        .and_then(|c| c.get_etherscan_api_key());

    let client = blockchain::ethereum::EtherscanClient::new(is_testnet, api_key);

    // Fetch address info (balance + tokens)
    match client.get_address_info(&addr.address).await {
        Ok(addr_info) => {
            // Persist ETH holding
            let holding_repo = HoldingRepository::new(pool);
            holding_repo
                .set_quantity(account_id, "ETH", addr_info.balance, None)
                .await?;

            // Persist ERC-20 token holdings
            for token in &addr_info.tokens {
                holding_repo
                    .set_quantity(account_id, &token.symbol, token.balance, None)
                    .await?;
            }

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
                info("  ✓ Balance saved to portfolio");
            }

            // Import transaction history if requested
            if import_history {
                match client.fetch_transactions(&addr.address).await {
                    Ok(txs) => {
                        let tx_repo = TransactionRepository::new(pool);
                        let mut saved = 0usize;
                        for tx in &txs {
                            // Skip failed transactions
                            if tx.is_error {
                                continue;
                            }

                            let external_id = format!("eth-tx-{}", tx.hash);
                            let exists: bool = sqlx::query_scalar(
                                "SELECT EXISTS(SELECT 1 FROM transactions WHERE external_id = ?)",
                            )
                            .bind(&external_id)
                            .fetch_one(pool)
                            .await?;

                            if exists {
                                continue;
                            }

                            // Fee in ETH = gas_used * gas_price_gwei / 1e9
                            let fee_eth = Decimal::from(tx.gas_used) * tx.gas_price
                                / Decimal::from(1_000_000_000u64);

                            // Determine direction
                            let is_incoming = tx.to.eq_ignore_ascii_case(&addr.address);
                            let (tx_type, from_id, to_id) = if is_incoming {
                                (
                                    TransactionType::TransferIn,
                                    None,
                                    Some(account_id.to_string()),
                                )
                            } else {
                                (
                                    TransactionType::TransferOut,
                                    Some(account_id.to_string()),
                                    None,
                                )
                            };

                            let timestamp = Utc
                                .timestamp_opt(tx.timestamp, 0)
                                .single()
                                .unwrap_or_else(Utc::now);

                            let record = Transaction {
                                id: 0,
                                tx_type,
                                from_account_id: from_id,
                                from_asset: if !is_incoming {
                                    Some("ETH".to_string())
                                } else {
                                    None
                                },
                                from_quantity: if !is_incoming { Some(tx.value) } else { None },
                                to_account_id: to_id,
                                to_asset: if is_incoming {
                                    Some("ETH".to_string())
                                } else {
                                    None
                                },
                                to_quantity: if is_incoming { Some(tx.value) } else { None },
                                price_usd: None,
                                price_currency: None,
                                price_amount: None,
                                exchange_rate: None,
                                exchange_rate_pair: None,
                                fee: Some(fee_eth),
                                fee_asset: Some("ETH".to_string()),
                                external_id: Some(external_id),
                                notes: None,
                                timestamp,
                                created_at: Utc::now(),
                            };

                            tx_repo.insert(&record).await?;
                            saved += 1;
                        }
                        if !opts.quiet {
                            success(&format!(
                                "✓ Imported {} transactions ({} new)",
                                txs.len(),
                                saved
                            ));
                        }
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
    account_id: &str,
    wallet_name: &str,
    addr: &WalletAddress,
    is_testnet: bool,
    import_history: bool,
    opts: &GlobalOptions,
    pool: &SqlitePool,
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
            // Persist ADA holding
            let holding_repo = HoldingRepository::new(pool);
            holding_repo
                .set_quantity(account_id, "ADA", addr_info.balance, None)
                .await?;

            // Persist native token holdings
            for token in &addr_info.tokens {
                holding_repo
                    .set_quantity(account_id, &token.display_name, token.balance, None)
                    .await?;
            }

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
                info("  ✓ Balance saved to portfolio");
            }

            // Import transaction history if requested
            if import_history {
                match client.fetch_transactions(&addr.address).await {
                    Ok(txs) => {
                        if !opts.quiet {
                            success(&format!(
                                "✓ Fetched {} transactions (history saved)",
                                txs.len()
                            ));
                        }
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

    // Delete sync_audit_log rows explicitly (FK has no CASCADE)
    let account = account_repo
        .get_account(&name)
        .await?
        .ok_or_else(|| CryptofolioError::AccountNotFound(name.clone()))?;
    sqlx::query("DELETE FROM sync_audit_log WHERE account_id = ?")
        .bind(&account.id)
        .execute(pool)
        .await?;

    // Delete the account (cascades to wallet_addresses, holdings, blockchain_sync_state, etc.)
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
        "solana" => validate_solana_address(address),
        "cardano" => blockchain::cardano::validate_address(address),
        _ => Err(CryptofolioError::Other(format!(
            "Unsupported blockchain: {}. Supported: bitcoin, ethereum, solana, cardano",
            blockchain
        ))),
    }
}

/// Validate a Solana address.
///
/// Solana addresses are base58-encoded Ed25519 public keys (32 bytes).
/// Uses the bitcoin crate's plain base58 decoder to verify the decoded length.
fn validate_solana_address(address: &str) -> Result<()> {
    if address.is_empty() {
        return Err(CryptofolioError::Other("Empty Solana address".to_string()));
    }

    // Base58 character length range for 32 bytes: 32–44 chars
    if address.len() < 32 || address.len() > 44 {
        return Err(CryptofolioError::Other(
            "Invalid Solana address: must be 32-44 characters".to_string(),
        ));
    }

    // Decode plain base58 (no checksum) and verify it yields exactly 32 bytes
    let decoded = bitcoin::base58::decode(address).map_err(|_| {
        CryptofolioError::Other("Invalid Solana address: invalid base58".to_string())
    })?;

    if decoded.len() != 32 {
        return Err(CryptofolioError::Other(format!(
            "Invalid Solana address: expected 32-byte key, got {} bytes",
            decoded.len()
        )));
    }

    Ok(())
}
