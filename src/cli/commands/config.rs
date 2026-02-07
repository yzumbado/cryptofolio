use colored::Colorize;
use sqlx::SqlitePool;
use std::io::{self, BufRead, Write};

use crate::cli::{ConfigCommands, GlobalOptions};
use crate::cli::output::{info, print_kv, success};
use crate::config::AppConfig;
use crate::error::Result;

pub async fn handle_config_command(command: ConfigCommands, _pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let _ = opts;
    match command {
        ConfigCommands::Show => {
            let config = AppConfig::load()?;

            println!();
            println!("{}", "Configuration".bold());
            println!();

            println!("{}", "[general]".dimmed());
            print_kv("default_account", config.general.default_account.as_deref().unwrap_or("-"));
            print_kv("use_testnet", if config.general.use_testnet { "true" } else { "false" });
            print_kv("currency", &config.general.currency);
            println!();

            println!("{}", "[binance]".dimmed());
            print_kv("api_key", if config.binance.api_key.is_some() { "***configured***" } else { "-" });
            print_kv("api_secret", if config.binance.api_secret.is_some() { "***configured***" } else { "-" });
            println!();

            println!("{}", "[display]".dimmed());
            print_kv("color", if config.display.color { "true" } else { "false" });
            print_kv("decimals", &config.display.decimals.to_string());
            println!();

            // Show paths
            println!("{}", "Paths".bold());
            println!();
            print_kv("config_dir", &AppConfig::config_dir()?.display().to_string());
            print_kv("database", &AppConfig::database_path()?.display().to_string());
            println!();
        }

        ConfigCommands::Set { key, value } => {
            let mut config = AppConfig::load()?;

            // If no value provided and it's a secret, read from stdin
            let final_value = if let Some(v) = value {
                v
            } else if key.contains("secret") || key.contains("key") {
                // Read secret from stdin
                info("Reading secret from stdin (paste and press Enter):");
                io::stdout().flush()?;

                let stdin = io::stdin();
                let mut line = String::new();
                stdin.lock().read_line(&mut line)?;
                line.trim().to_string()
            } else {
                return Err(crate::error::CryptofolioError::Config(
                    "Value is required for non-secret configuration keys".into()
                ));
            };

            config.set(&key, &final_value)?;
            config.save()?;

            // Mask sensitive values
            let display_value = if key.contains("secret") || key.contains("key") {
                "***".to_string()
            } else {
                final_value
            };

            success(&format!("Set {} = {}", key, display_value));
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
            println!("  {}", "Warning: Real funds will be used for trading operations!".yellow());
        }
    }

    Ok(())
}
