use crate::support::world::CryptofolioWorld;
use cucumber::{given, then, when};

/// Common step definitions shared across features

#[given("I have a clean test database")]
async fn setup_database(world: &mut CryptofolioWorld) {
    world
        .setup_db()
        .await
        .expect("Failed to setup test database");
}

#[given(expr = "I have an account {string}")]
async fn create_account(world: &mut CryptofolioWorld, account_name: String) {
    use chrono::Utc;
    use cryptofolio::core::account::{Account, AccountConfig, AccountType};
    use cryptofolio::db::accounts::AccountRepository;

    let pool = world.pool();
    let repo = AccountRepository::new(pool);

    let account = Account {
        id: account_name.to_lowercase().replace(" ", "-"),
        name: account_name.clone(),
        category_id: "trading".to_string(),
        account_type: AccountType::SoftwareWallet,
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };

    repo.create_account(&account)
        .await
        .expect("Failed to create account");

    world.last_account_id = Some(account.id);
}

#[when(expr = "I run {string}")]
async fn run_command(world: &mut CryptofolioWorld, command: String) {
    use std::io::Write;

    world.reset_command_state();

    // Parse command (strip "cryptofolio " prefix if present)
    let cmd = command.strip_prefix("cryptofolio ").unwrap_or(&command);

    // Use shell_words to properly parse quoted arguments
    let parts = match shell_words::split(cmd) {
        Ok(p) => p,
        Err(e) => {
            world.last_output = format!("Failed to parse command: {}", e);
            world.last_exit_code = 1;
            return;
        }
    };

    if parts.is_empty() {
        world.last_exit_code = 1;
        world.last_output = "Empty command".to_string();
        return;
    }

    // Capture output
    let mut output = Vec::new();

    let result = match parts.get(0).map(|s| s.as_str()) {
        Some("wallet") => {
            let args: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();
            handle_wallet_command_test(world, &args, &mut output).await
        }
        Some("sync-history") => {
            let args: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();
            handle_sync_history_command_test(world, &args, &mut output).await
        }
        Some(cmd) => {
            world.last_output = format!("Unknown command: {}", cmd);
            world.last_exit_code = 1;
            return;
        }
        None => {
            world.last_output = "Empty command".to_string();
            world.last_exit_code = 1;
            return;
        }
    };

    world.last_output = String::from_utf8_lossy(&output).to_string();
    world.last_exit_code = if result.is_ok() { 0 } else { 1 };

    if let Err(e) = result {
        writeln!(&mut output, "[ERROR] {}", e).ok();
        world.last_output = String::from_utf8_lossy(&output).to_string();
    }
}

async fn handle_wallet_command_test(
    world: &mut CryptofolioWorld,
    args: &[&str],
    output: &mut Vec<u8>,
) -> anyhow::Result<()> {
    use cryptofolio::cli::commands::wallet::*;
    use cryptofolio::cli::WalletCommands;
    use std::io::Write;

    let pool = world.pool();
    let opts = cryptofolio::cli::GlobalOptions {
        no_color: false,
        testnet: false,
        json: false,
        quiet: false,
        verbose: false,
    };

    if args.is_empty() {
        writeln!(output, "[ERROR] Missing wallet subcommand")?;
        return Err(anyhow::anyhow!("Missing wallet subcommand"));
    }

    match args[0] {
        "add" => {
            // Parse: wallet add 'Name' --blockchain bitcoin --address bc1...
            let name = args
                .get(1)
                .map(|s| s.trim_matches('\''))
                .ok_or_else(|| anyhow::anyhow!("Missing wallet name"))?;

            let mut blockchain = None;
            let mut address = None;
            let mut xpub = None;
            let mut network = None;

            let mut i = 2;
            while i < args.len() {
                match args[i] {
                    "--blockchain" => {
                        blockchain = args.get(i + 1).copied();
                        i += 2;
                    }
                    "--address" => {
                        address = args.get(i + 1).copied();
                        i += 2;
                    }
                    "--xpub" => {
                        xpub = args.get(i + 1).copied();
                        i += 2;
                    }
                    "--network" => {
                        network = args.get(i + 1).copied();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            let cmd = WalletCommands::Add {
                name: name.to_string(),
                blockchain: blockchain
                    .ok_or_else(|| anyhow::anyhow!("Missing --blockchain"))?
                    .to_string(),
                address: address.map(String::from),
                xpub: xpub.map(String::from),
                derivation_path: None,
                address_type: None,
                label: None,
            };

            handle_wallet_command(cmd, pool, &opts).await?;

            let testnet_tag = if network == Some("testnet") {
                " [TESTNET]"
            } else {
                ""
            };
            if xpub.is_some() {
                writeln!(output, "[OK] ✓ Added HD wallet '{}'{}", name, testnet_tag)?;
            } else {
                writeln!(output, "[OK] ✓ Added wallet '{}'{}", name, testnet_tag)?;
            }
        }
        "list" => {
            let cmd = WalletCommands::List { blockchain: None };
            handle_wallet_command(cmd, pool, &opts).await?;

            // Query wallets and write to output
            use cryptofolio::db::accounts::AccountRepository;
            let repo = AccountRepository::new(pool);
            let accounts = repo.list_accounts().await?;

            for account in accounts {
                writeln!(output, "{}", account.name)?;
                let addresses = repo.list_addresses(&account.id).await?;
                for addr in addresses {
                    writeln!(output, "  {} {}", addr.blockchain, addr.address)?;
                }
            }
        }
        "sync" => {
            // Parse: wallet sync 'Name' [--import-history]
            let name = args
                .get(1)
                .map(|s| s.trim_matches('\''))
                .ok_or_else(|| anyhow::anyhow!("Missing wallet name"))?;

            let import_history = args.contains(&"--import-history");

            // Get the wallet
            use cryptofolio::db::accounts::AccountRepository;
            let repo = AccountRepository::new(pool);
            let account = match repo.get_account(&name).await {
                Ok(Some(acc)) => acc,
                Ok(None) => {
                    writeln!(output, "[ERROR] Wallet not found: {}", name)?;
                    return Err(anyhow::anyhow!("Wallet not found: {}", name));
                }
                Err(e) => {
                    writeln!(output, "[ERROR] Database error: {}", e)?;
                    return Err(e.into());
                }
            };

            let addresses = repo.list_addresses(&account.id).await?;
            if addresses.is_empty() {
                writeln!(output, "[ERROR] No addresses found for wallet")?;
                return Err(anyhow::anyhow!("No addresses"));
            }

            // Get the mock server URLs if available
            let btc_mock_url = world.blockchain_mock_url();
            let eth_mock_url = world
                .ethereum_mock
                .as_ref()
                .map(|m| format!("{}/api", m.url()));
            let ada_mock_url = world.cardano_mock.as_ref().map(|m| m.url());

            // Set up accumulated ERC-20 tokens if any
            if !world.accumulated_tokens.is_empty() {
                if let Some(mock) = &world.ethereum_mock {
                    let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
                    // Mock ETH balance
                    mock.mock_balance(address, 0.5).await;
                    // Mock all accumulated tokens
                    let tokens_ref: Vec<(&str, &str, f64, u8)> = world
                        .accumulated_tokens
                        .iter()
                        .map(|(s, n, b, d)| (s.as_str(), n.as_str(), *b, *d))
                        .collect();
                    mock.mock_tokens(address, &tokens_ref).await;
                    // Clear accumulated tokens after setting up mock
                    world.accumulated_tokens.clear();
                }
            }

            // Set up accumulated Cardano native tokens if any
            if !world.accumulated_cardano_tokens.is_empty() {
                if let Some(mock) = &world.cardano_mock {
                    let address = "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa";
                    // Mock ADA balance
                    mock.mock_balance(address, 100.0).await;
                    // Mock all accumulated tokens
                    let tokens_ref: Vec<(&str, &str, u8)> = world
                        .accumulated_cardano_tokens
                        .iter()
                        .map(|(s, q, d)| (s.as_str(), q.as_str(), *d))
                        .collect();
                    mock.mock_tokens(address, &tokens_ref).await;
                    // Clear accumulated tokens after setting up mock
                    world.accumulated_cardano_tokens.clear();
                }
            }

            for addr in addresses {
                let is_testnet = addr.network.as_deref() == Some("testnet");

                if addr.blockchain.eq_ignore_ascii_case("bitcoin") {
                    // Create Bitcoin client (use mock URL if available)
                    use cryptofolio::blockchain::bitcoin::BlockstreamClient;
                    let client = if let Some(url) = &btc_mock_url {
                        BlockstreamClient::with_base_url(url.clone())
                    } else {
                        BlockstreamClient::new(is_testnet)
                    };

                    // Fetch address info
                    match client.get_address_info(&addr.address).await {
                        Ok(addr_info) => {
                            writeln!(
                                output,
                                "[OK] ✓ Synced BTC balance: {:.4}",
                                addr_info.balance
                            )?;
                            writeln!(output, "[INFO]   Transactions: {}", addr_info.tx_count)?;
                            writeln!(
                                output,
                                "[INFO]   Total received: {:.8} BTC",
                                addr_info.total_received
                            )?;
                            writeln!(
                                output,
                                "[INFO]   Total sent: {:.8} BTC",
                                addr_info.total_sent
                            )?;

                            if import_history {
                                match client.get_transactions(&addr.address).await {
                                    Ok(txs) => {
                                        writeln!(
                                            output,
                                            "[OK] ✓ Imported {} transactions",
                                            txs.len()
                                        )?;
                                    }
                                    Err(e) => {
                                        writeln!(
                                            output,
                                            "[ERROR] Failed to fetch transactions: {}",
                                            e
                                        )?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            writeln!(output, "[ERROR] Failed to sync {}: {}", addr.address, e)?;
                            return Err(anyhow::anyhow!("Sync failed: {}", e));
                        }
                    }
                } else if addr.blockchain.eq_ignore_ascii_case("ethereum") {
                    // Create Ethereum client (use mock URL if available)
                    use cryptofolio::blockchain::ethereum::EtherscanClient;
                    let client = if let Some(url) = &eth_mock_url {
                        EtherscanClient::with_base_url(url.clone())
                    } else {
                        EtherscanClient::new(is_testnet, None)
                    };

                    // Fetch address info (balance + tokens)
                    match client.get_address_info(&addr.address).await {
                        Ok(addr_info) => {
                            writeln!(
                                output,
                                "[OK] ✓ Synced ETH balance: {:.4}",
                                addr_info.balance
                            )?;

                            // Show token balances
                            if addr_info.tokens.is_empty() {
                                writeln!(output, "[INFO]   No tokens found")?;
                            } else {
                                writeln!(output, "[OK] ✓ Found {} tokens", addr_info.tokens.len())?;
                                for token in &addr_info.tokens {
                                    writeln!(
                                        output,
                                        "[INFO]   {}: {:.2}",
                                        token.symbol, token.balance
                                    )?;
                                }
                            }

                            if import_history {
                                match client.get_transactions(&addr.address).await {
                                    Ok(txs) => {
                                        writeln!(
                                            output,
                                            "[OK] ✓ Imported {} transactions",
                                            txs.len()
                                        )?;
                                    }
                                    Err(e) => {
                                        writeln!(
                                            output,
                                            "[ERROR] Failed to fetch transactions: {}",
                                            e
                                        )?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            writeln!(output, "[ERROR] Failed to sync {}: {}", addr.address, e)?;
                            return Err(anyhow::anyhow!("Sync failed: {}", e));
                        }
                    }
                } else if addr.blockchain.eq_ignore_ascii_case("cardano") {
                    // Create Cardano client (use mock URL if available)
                    use cryptofolio::blockchain::cardano::BlockfrostClient;
                    let client = if let Some(url) = &ada_mock_url {
                        BlockfrostClient::with_base_url(url.clone())
                    } else {
                        BlockfrostClient::new(is_testnet, None)
                    };

                    // Fetch address info (balance + native tokens)
                    match client.get_address_info(&addr.address).await {
                        Ok(addr_info) => {
                            writeln!(output, "[OK] ✓ Synced ADA balance: {}", addr_info.balance)?;

                            // Show native token balances
                            if addr_info.tokens.is_empty() {
                                writeln!(output, "[INFO]   No tokens found")?;
                            } else {
                                writeln!(output, "[OK] ✓ Found {} tokens", addr_info.tokens.len())?;
                                for token in &addr_info.tokens {
                                    writeln!(
                                        output,
                                        "[INFO]   {}: {:.2}",
                                        token.display_name, token.balance
                                    )?;
                                }
                            }

                            // Show stake delegation if any
                            if let Some(pool) = &addr_info.stake_pool {
                                writeln!(output, "[INFO]   Delegated to: {}", pool.ticker)?;
                            }

                            if import_history {
                                match client.get_transactions(&addr.address).await {
                                    Ok(txs) => {
                                        writeln!(
                                            output,
                                            "[OK] ✓ Imported {} transactions",
                                            txs.len()
                                        )?;
                                    }
                                    Err(e) => {
                                        writeln!(
                                            output,
                                            "[ERROR] Failed to fetch transactions: {}",
                                            e
                                        )?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            writeln!(output, "[ERROR] Failed to sync {}: {}", addr.address, e)?;
                            return Err(anyhow::anyhow!("Sync failed: {}", e));
                        }
                    }
                } else {
                    writeln!(
                        output,
                        "[WARN] Blockchain {} not yet supported for sync",
                        addr.blockchain
                    )?;
                }
            }
        }
        "remove" => {
            let name = args
                .get(1)
                .map(|s| s.trim_matches('\''))
                .ok_or_else(|| anyhow::anyhow!("Missing wallet name"))?;

            let cmd = WalletCommands::Remove {
                name: name.to_string(),
                yes: true,
            };

            handle_wallet_command(cmd, pool, &opts).await?;
            writeln!(output, "[OK] ✓ Removed wallet '{}'", name)?;
        }
        _ => {
            writeln!(output, "[ERROR] Unknown wallet subcommand: {}", args[0])?;
            return Err(anyhow::anyhow!("Unknown subcommand"));
        }
    }

    Ok(())
}

#[then("the command should succeed")]
fn command_succeeds(world: &mut CryptofolioWorld) {
    assert_eq!(
        world.last_exit_code, 0,
        "Command failed with exit code {}. Output: {}",
        world.last_exit_code, world.last_output
    );
}

#[then("the command should fail")]
fn command_fails(world: &mut CryptofolioWorld) {
    assert_ne!(
        world.last_exit_code, 0,
        "Command succeeded but was expected to fail"
    );
}

#[then(expr = "I should see {string}")]
fn output_contains(world: &mut CryptofolioWorld, expected: String) {
    assert!(
        world.last_output.contains(&expected),
        "Expected output to contain '{}', but got: {}",
        expected,
        world.last_output
    );
}

#[then(expr = "I should not see {string}")]
fn output_not_contains(world: &mut CryptofolioWorld, unexpected: String) {
    assert!(
        !world.last_output.contains(&unexpected),
        "Expected output NOT to contain '{}', but it did: {}",
        unexpected,
        world.last_output
    );
}

async fn handle_sync_history_command_test(
    world: &mut CryptofolioWorld,
    args: &[&str],
    output: &mut Vec<u8>,
) -> anyhow::Result<()> {
    use cryptofolio::cli::commands::sync::handle_sync_history_command;
    use std::io::Write;

    let pool = world.pool();
    let opts = cryptofolio::cli::GlobalOptions {
        no_color: false,
        testnet: false,
        json: false,
        quiet: false,
        verbose: false,
    };

    // Parse flags from args
    let mut account = String::new();
    let mut symbols = String::new();
    let mut full_history = false;
    let mut from: Option<String> = None;
    let mut no_trades = false;
    let mut no_deposits = false;
    let mut no_withdrawals = false;
    let mut no_fiat = false;
    let mut no_transfers = false;
    let mut dry_run = false;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--account" => {
                i += 1;
                if i < args.len() {
                    account = args[i].to_string();
                }
            }
            "--symbols" => {
                i += 1;
                if i < args.len() {
                    symbols = args[i].to_string();
                }
            }
            "--from" => {
                i += 1;
                if i < args.len() {
                    from = Some(args[i].to_string());
                }
            }
            "--full-history" => full_history = true,
            "--no-trades" => no_trades = true,
            "--no-deposits" => no_deposits = true,
            "--no-withdrawals" => no_withdrawals = true,
            "--no-fiat" => no_fiat = true,
            "--no-transfers" => no_transfers = true,
            "--dry-run" => dry_run = true,
            _ => {}
        }
        i += 1;
    }

    let result = handle_sync_history_command(
        account,
        symbols,
        full_history,
        from,
        no_trades,
        no_deposits,
        no_withdrawals,
        no_fiat,
        no_transfers,
        dry_run,
        &pool,
        &opts,
    )
    .await;

    if let Err(ref e) = result {
        writeln!(output, "{}", e).ok();
    }

    result.map_err(|e| anyhow::anyhow!("{}", e))
}
