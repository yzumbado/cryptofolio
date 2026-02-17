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
use crate::error::Result;

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
}

#[derive(Serialize)]
struct PathsConfig {
    config_dir: String,
    database: String,
}

pub async fn handle_config_command(
    command: ConfigCommands,
    _pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let _ = opts;
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
        } => {
            handle_set_secret_command(key, secret_file, from_env).await?;
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
    }

    Ok(())
}

async fn handle_set_secret_command(
    key: String,
    secret_file: Option<PathBuf>,
    from_env: Option<String>,
) -> Result<()> {
    // Show security warning first
    show_security_warning(&key)?;

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

    // Validate secret is not empty (already checked in read functions, but double-check)
    if secret.trim().is_empty() {
        return Err(crate::error::CryptofolioError::Config(
            "Empty secret provided".into(),
        ));
    }

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

