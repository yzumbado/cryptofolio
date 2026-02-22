use colored::Colorize;
use serde::Serialize;
use sqlx::SqlitePool;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::cli::{ConfigCommands, GlobalOptions};
use crate::cli::output::{print_kv, success};
use crate::config::secrets::{
    ensure_secure_permissions, is_secret_key, read_secret_from_env, read_secret_from_file,
    read_secret_from_stdin, read_secret_interactive, show_security_warning,
};
use crate::config::AppConfig;
use crate::db::KeychainKeyRepository;
use crate::error::{CryptofolioError, Result};

#[cfg(target_os = "macos")]
use crate::config::keychain::{get_keychain, KeychainSecurityLevel};
#[cfg(target_os = "macos")]
use crate::config::migration;

#[derive(Serialize)]
struct ConfigOutput {
    general: GeneralConfig,
    binance: BinanceConfig,
    display: DisplayConfig,
    paths: PathsConfig,
}

#[derive(Serialize)]
struct GeneralConfig {
    default_account: Option<String>,
    use_testnet: bool,
    currency: String,
}

#[derive(Serialize)]
struct BinanceConfig {
    api_key_configured: bool,
    api_secret_configured: bool,
}

#[derive(Serialize)]
struct DisplayConfig {
    color: bool,
    decimals: u8,
    price_decimals: u8,
    thousands_separator: bool,
}

#[derive(Serialize)]
struct PathsConfig {
    config_dir: String,
    database: String,
}

pub async fn handle_config_command(
    command: ConfigCommands,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    match command {
        ConfigCommands::Show => {
            let config = AppConfig::load()?;

            if opts.json {
                let output = ConfigOutput {
                    general: GeneralConfig {
                        default_account: config.general.default_account.clone(),
                        use_testnet: config.general.use_testnet,
                        currency: config.general.currency.clone(),
                    },
                    binance: BinanceConfig {
                        api_key_configured: config.binance.api_key.is_some(),
                        api_secret_configured: config.binance.api_secret.is_some(),
                    },
                    display: DisplayConfig {
                        color: config.display.color,
                        decimals: config.display.decimals,
                        price_decimals: config.display.price_decimals,
                        thousands_separator: config.display.thousands_separator,
                    },
                    paths: PathsConfig {
                        config_dir: AppConfig::config_dir()?.display().to_string(),
                        database: AppConfig::database_path()?.display().to_string(),
                    },
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
            } else {
                println!();
                println!("{}", "Configuration".bold());
                println!();

                println!("{}", "[general]".dimmed());
                print_kv(
                    "default_account",
                    config
                        .general
                        .default_account
                        .as_deref()
                        .unwrap_or("-"),
                );
                print_kv(
                    "use_testnet",
                    if config.general.use_testnet {
                        "true"
                    } else {
                        "false"
                    },
                );
                print_kv("currency", &config.general.currency);
                println!();

                println!("{}", "[binance]".dimmed());
                print_kv(
                    "api_key",
                    if config.binance.api_key.is_some() {
                        "***configured***"
                    } else {
                        "-"
                    },
                );
                print_kv(
                    "api_secret",
                    if config.binance.api_secret.is_some() {
                        "***configured***"
                    } else {
                        "-"
                    },
                );
                println!();

                println!("{}", "[display]".dimmed());
                print_kv("color", if config.display.color { "true" } else { "false" });
                print_kv("decimals", &config.display.decimals.to_string());
                print_kv("price_decimals", &config.display.price_decimals.to_string());
                print_kv("thousands_separator", if config.display.thousands_separator { "true" } else { "false" });
                println!();

                // Show paths
                println!("{}", "Paths".bold());
                println!();
                print_kv(
                    "config_dir",
                    &AppConfig::config_dir()?.display().to_string(),
                );
                print_kv(
                    "database",
                    &AppConfig::database_path()?.display().to_string(),
                );
                println!();
            }
        }

        ConfigCommands::Set { key, value } => {
            // Warn if user is trying to set a secret insecurely
            if is_secret_key(&key) {
                eprintln!();
                eprintln!(
                    "{}",
                    "⚠️  WARNING: Setting secrets via command line arguments is insecure!"
                        .yellow()
                        .bold()
                );
                eprintln!("{}", "⚠️  Your secret will be visible in shell history.".yellow());
                eprintln!();
                eprintln!("   Use this instead:");
                eprintln!("   {}", format!("cryptofolio config set-secret {}", key).cyan());
                eprintln!();
                print!("Continue anyway? [y/N] ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!();
                    println!("Cancelled. No changes made.");
                    println!();
                    return Ok(());
                }
                println!();
            }

            let mut config = AppConfig::load()?;
            config.set(&key, &value)?;
            config.save()?;

            // Ensure secure permissions
            let config_path = AppConfig::config_path()?;
            ensure_secure_permissions(&config_path)?;

            // Mask sensitive values
            let display_value = if is_secret_key(&key) {
                "***".to_string()
            } else {
                value
            };

            success(&format!("Set {} = {}", key, display_value));
        }

        ConfigCommands::SetSecret {
            key,
            secret_file,
            from_env,
            security_level,
        } => {
            handle_set_secret_command(key, secret_file, from_env, security_level, pool).await?;
        }

        ConfigCommands::UseTestnet => {
            let mut config = AppConfig::load()?;
            config.general.use_testnet = true;
            config.save()?;

            success("Testnet mode enabled");
            println!("  All exchange operations will use testnet endpoints.");
        }

        ConfigCommands::UseMainnet => {
            let mut config = AppConfig::load()?;
            config.general.use_testnet = false;
            config.save()?;

            success("Mainnet mode enabled");
            println!(
                "  {}",
                "Warning: Real funds will be used for trading operations!".yellow()
            );
        }

        ConfigCommands::MigrateToKeychain => {
            #[cfg(target_os = "macos")]
            {
                let keychain_repo = KeychainKeyRepository::new(pool.clone());
                migration::run_migration(&keychain_repo).await?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                return Err(CryptofolioError::KeychainNotAvailable);
            }
        }

        ConfigCommands::KeychainStatus => {
            #[cfg(target_os = "macos")]
            {
                handle_keychain_status_command(pool, opts).await?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                return Err(CryptofolioError::KeychainNotAvailable);
            }
        }

        ConfigCommands::UpgradeSecurity { key, to } => {
            #[cfg(target_os = "macos")]
            {
                handle_upgrade_security_command(key, to, pool).await?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                return Err(CryptofolioError::KeychainNotAvailable);
            }
        }

        ConfigCommands::DowngradeSecurity { key, to } => {
            #[cfg(target_os = "macos")]
            {
                handle_downgrade_security_command(key, to, pool).await?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                return Err(CryptofolioError::KeychainNotAvailable);
            }
        }
    }

    Ok(())
}

async fn handle_set_secret_command(
    key: String,
    secret_file: Option<PathBuf>,
    from_env: Option<String>,
    security_level: Option<String>,
    pool: &SqlitePool,
) -> Result<()> {
    // Read secret from appropriate source
    let secret = if let Some(env_var) = from_env {
        // Read from environment variable
        read_secret_from_env(&env_var)?
    } else if let Some(file_path) = secret_file {
        // Read from file
        read_secret_from_file(&file_path)?
    } else if is_terminal::is_terminal(std::io::stdin()) {
        // TTY available - interactive prompt
        read_secret_interactive(&key)?
    } else {
        // Stdin piped - read from pipe
        read_secret_from_stdin()?
    };

    // Validate secret is not empty
    if secret.trim().is_empty() {
        return Err(CryptofolioError::Config("Empty secret provided".into()));
    }

    // Try to store in keychain on macOS
    #[cfg(target_os = "macos")]
    {
        let keychain = get_keychain();

        // Determine security level
        let level = if let Some(ref level_str) = security_level {
            KeychainSecurityLevel::from_str(level_str).ok_or_else(|| {
                CryptofolioError::Config(format!(
                    "Invalid security level: {}. Use: standard, touchid, or touchid-only",
                    level_str
                ))
            })?
        } else {
            // Default: Touch ID Protected (if available), otherwise Standard
            if keychain.is_touchid_available() {
                KeychainSecurityLevel::TouchIdProtected
            } else {
                KeychainSecurityLevel::Standard
            }
        };

        // Store in keychain
        match keychain.store_with_security(&key, &secret, level) {
            Ok(()) => {
                // Record in database
                let keychain_repo = KeychainKeyRepository::new(pool.clone());
                keychain_repo
                    .upsert(
                        &key,
                        crate::db::keychain::StorageType::Keychain,
                        Some(level),
                    )
                    .await?;

                println!();
                println!(
                    "  ✓ Secret stored in macOS Keychain ({})",
                    level.as_display_str()
                );
                println!();
                println!("  {}", "⚠️  Remember: Use READ-ONLY API keys only!".yellow());
                println!();

                return Ok(());
            }
            Err(e) => {
                eprintln!("Warning: Failed to store in keychain: {}", e);
                eprintln!("Falling back to TOML storage...");
                eprintln!();
                // Fall through to TOML storage
            }
        }
    }

    // Fall back to TOML storage (or only option on non-macOS)
    #[cfg(not(target_os = "macos"))]
    {
        if security_level.is_some() {
            eprintln!("Warning: --security-level is only supported on macOS");
            eprintln!();
        }
    }

    // Show security warning for TOML storage
    show_security_warning(&key)?;

    // Save to config
    let mut config = AppConfig::load()?;
    config.set(&key, &secret)?;
    config.save()?;

    // Ensure secure file permissions
    let config_path = AppConfig::config_path()?;
    ensure_secure_permissions(&config_path)?;

    // Success message
    println!("✓ Secret saved to ~/.config/cryptofolio/config.toml");
    println!();
    println!("  {}", "⚠️  Remember: Use READ-ONLY API keys only!".yellow());
    println!();

    Ok(())
}

#[cfg(target_os = "macos")]
async fn handle_keychain_status_command(pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    use crate::db::keychain::StorageType;

    let keychain_repo = KeychainKeyRepository::new(pool.clone());
    let keys = keychain_repo.list().await?;

    if opts.json {
        // JSON output
        let json_keys: Vec<serde_json::Value> = keys
            .iter()
            .map(|k| {
                serde_json::json!({
                    "key_name": k.key_name,
                    "storage_type": k.storage_type.as_str(),
                    "security_level": k.security_level.as_ref().map(|l| l.as_db_str()),
                    "last_accessed": k.last_accessed,
                    "migrated_at": k.migrated_at,
                })
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&json_keys)?);
        return Ok(());
    }

    // Table output
    println!();
    println!("{}", "Keychain Status".bold());
    println!();

    if keys.is_empty() {
        println!("  No secrets tracked in keychain");
        println!();
        println!("  Run {} to migrate secrets from config.toml", "cryptofolio config migrate-to-keychain".cyan());
        println!();
        return Ok(());
    }

    // Print table header
    println!("{}", "┌────────────────────────┬──────────────────┬────────────┐".dimmed());
    println!(
        "{}",
        "│ Key                    │ Security Level   │ Status     │".dimmed()
    );
    println!("{}", "├────────────────────────┼──────────────────┼────────────┤".dimmed());

    for key in &keys {
        let security_level = if key.storage_type == StorageType::Keychain {
            key.security_level
                .as_ref()
                .map(|l| l.as_display_str())
                .unwrap_or("Unknown")
        } else {
            "-"
        };

        let status = match key.storage_type {
            StorageType::Keychain => "✓ Active".green().to_string(),
            StorageType::Toml => "TOML".yellow().to_string(),
            StorageType::Env => "ENV".yellow().to_string(),
        };

        println!(
            "│ {:<22} │ {:<16} │ {:<10} │",
            truncate_string(&key.key_name, 22),
            security_level,
            status
        );
    }

    println!("{}", "└────────────────────────┴──────────────────┴────────────┘".dimmed());
    println!();

    Ok(())
}

#[cfg(target_os = "macos")]
async fn handle_upgrade_security_command(
    key: String,
    to: String,
    pool: &SqlitePool,
) -> Result<()> {
    let keychain = get_keychain();

    // Check if key exists in keychain
    if !keychain.exists(&key) {
        return Err(CryptofolioError::Keychain(format!(
            "Secret '{}' not found in keychain. Use 'config set-secret {}' first.",
            key, key
        )));
    }

    // Parse target security level
    let target_level = KeychainSecurityLevel::from_str(&to).ok_or_else(|| {
        CryptofolioError::Config(format!("Invalid security level: {}", to))
    })?;

    // Validate upgrade path
    if !matches!(
        target_level,
        KeychainSecurityLevel::TouchIdProtected | KeychainSecurityLevel::TouchIdOnly
    ) {
        return Err(CryptofolioError::Config(
            "Can only upgrade to 'touchid' or 'touchid-only'".into(),
        ));
    }

    println!();
    println!("  Upgrading security for: {}", key);
    println!("  Target level: {}", target_level.as_display_str());
    println!();

    // Update security level (this will trigger Touch ID prompt to retrieve current secret)
    keychain.update_security_level(&key, target_level)?;

    // Update database
    let keychain_repo = KeychainKeyRepository::new(pool.clone());
    keychain_repo.update_security_level(&key, target_level).await?;

    println!();
    success(&format!("Upgraded '{}' to {}", key, target_level.as_display_str()));
    println!();

    Ok(())
}

#[cfg(target_os = "macos")]
async fn handle_downgrade_security_command(
    key: String,
    to: String,
    pool: &SqlitePool,
) -> Result<()> {
    let keychain = get_keychain();

    // Check if key exists in keychain
    if !keychain.exists(&key) {
        return Err(CryptofolioError::Keychain(format!(
            "Secret '{}' not found in keychain",
            key
        )));
    }

    // Parse target security level
    let target_level = KeychainSecurityLevel::from_str(&to).ok_or_else(|| {
        CryptofolioError::Config(format!("Invalid security level: {}", to))
    })?;

    println!();
    println!("  Downgrading security for: {}", key);
    println!("  Target level: {}", target_level.as_display_str());
    println!();

    // Confirmation for downgrading to standard
    if target_level == KeychainSecurityLevel::Standard {
        println!("  {}", "⚠️  WARNING: Downgrading to Standard".yellow());
        println!("     Standard level doesn't require Touch ID for access");
        println!();

        print!("  Continue? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!();
            println!("  Cancelled. No changes made.");
            println!();
            return Ok(());
        }
    }

    // Update security level (this will trigger Touch ID prompt to retrieve current secret)
    keychain.update_security_level(&key, target_level)?;

    // Update database
    let keychain_repo = KeychainKeyRepository::new(pool.clone());
    keychain_repo.update_security_level(&key, target_level).await?;

    println!();
    success(&format!("Downgraded '{}' to {}", key, target_level.as_display_str()));
    println!();

    Ok(())
}

/// Truncate string to max length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
