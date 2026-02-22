//! TOML to Keychain migration tool
//!
//! Migrates secrets from plaintext TOML files to encrypted macOS Keychain.

use std::fs;
use std::io::{self, Write};

#[cfg(target_os = "macos")]
use super::keychain::{get_keychain, KeychainSecurityLevel};
use super::settings::AppConfig;
use crate::db::KeychainKeyRepository;
use crate::error::{CryptofolioError, Result};

/// Secret found in TOML config
#[derive(Debug)]
pub struct SecretToMigrate {
    pub key: String,
    pub display_name: String,
    pub has_value: bool,
}

/// Find all secrets in the current TOML config
pub fn find_secrets_in_config(config: &AppConfig) -> Vec<SecretToMigrate> {
    let mut secrets = Vec::new();

    // Binance API key
    if config.binance.api_key.is_some() {
        secrets.push(SecretToMigrate {
            key: "binance.api_key".to_string(),
            display_name: "Binance API Key".to_string(),
            has_value: true,
        });
    }

    // Binance API secret
    if config.binance.api_secret.is_some() {
        secrets.push(SecretToMigrate {
            key: "binance.api_secret".to_string(),
            display_name: "Binance API Secret".to_string(),
            has_value: true,
        });
    }

    // Claude API key
    if let Some(ref ai) = config.ai {
        if ai.claude_api_key.is_some() {
            secrets.push(SecretToMigrate {
                key: "ai.claude_api_key".to_string(),
                display_name: "Claude API Key".to_string(),
                has_value: true,
            });
        }
    }

    secrets
}

/// Prompt user to select security level
#[cfg(target_os = "macos")]
pub fn prompt_security_level(is_touchid_available: bool) -> Result<KeychainSecurityLevel> {
    println!();
    println!("  🔐 Security Level:");
    println!();

    if is_touchid_available {
        println!("    [1] Standard");
        println!("        • Protected by macOS encryption");
        println!("        • Accessible when Mac is unlocked");
        println!("        • Good for: Automated scripts, cron jobs");
        println!();
        println!("    [2] Touch ID Protected (Recommended)");
        println!("        • Requires Touch ID or password");
        println!("        • Prompts once per terminal session");
        println!("        • Good for: Daily interactive use");
        println!();
        println!("    [3] Touch ID Only (Maximum Security)");
        println!("        • ONLY biometric authentication");
        println!("        • No password fallback");
        println!("        • Good for: High-value accounts");
        println!();

        print!("  Choose level [1-3] (default: 2): ");
    } else {
        println!("    [1] Standard (Only option - Touch ID not available)");
        println!("        • Protected by macOS encryption");
        println!("        • Accessible when Mac is unlocked");
        println!();

        print!("  Choose level [1]: ");
    }

    io::stdout().flush().map_err(CryptofolioError::Io)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(CryptofolioError::Io)?;

    let choice = input.trim();

    if !is_touchid_available {
        return Ok(KeychainSecurityLevel::Standard);
    }

    match choice {
        "1" => Ok(KeychainSecurityLevel::Standard),
        "3" => Ok(KeychainSecurityLevel::TouchIdOnly),
        "" | "2" => Ok(KeychainSecurityLevel::TouchIdProtected),
        _ => {
            eprintln!("  Invalid choice, using Touch ID Protected");
            Ok(KeychainSecurityLevel::TouchIdProtected)
        }
    }
}

/// Create a backup of the current config
pub fn create_backup() -> Result<()> {
    let config_path = AppConfig::config_path()?;

    if !config_path.exists() {
        return Ok(());
    }

    let backup_path = config_path.with_extension("toml.backup");
    fs::copy(&config_path, &backup_path)?;

    println!("  ✓ Created backup: {}", backup_path.display());

    Ok(())
}

/// Migrate a single secret to keychain
#[cfg(target_os = "macos")]
pub async fn migrate_secret_to_keychain(
    config: &AppConfig,
    secret: &SecretToMigrate,
    security_level: KeychainSecurityLevel,
    keychain_repo: &KeychainKeyRepository,
) -> Result<()> {
    // Get the secret value from config
    let value = match secret.key.as_str() {
        "binance.api_key" => config.binance.api_key.clone(),
        "binance.api_secret" => config.binance.api_secret.clone(),
        "ai.claude_api_key" => config.ai.as_ref().and_then(|ai| ai.claude_api_key.clone()),
        _ => None,
    };

    let value = value.ok_or_else(|| {
        CryptofolioError::Config(format!("Secret '{}' has no value", secret.key))
    })?;

    // Store in keychain
    let keychain = get_keychain();
    keychain.store_with_security(&secret.key, &value, security_level)?;

    // Record in database
    keychain_repo
        .upsert(
            &secret.key,
            crate::db::keychain::StorageType::Keychain,
            Some(security_level),
        )
        .await?;

    keychain_repo.mark_migrated(&secret.key).await?;

    println!(
        "  ✓ Migrated {} to keychain ({})",
        secret.display_name,
        security_level.as_display_str()
    );

    Ok(())
}

/// Clear secrets from TOML config
pub fn clear_secrets_from_config(config: &mut AppConfig, secrets: &[SecretToMigrate]) -> Result<()> {
    for secret in secrets {
        match secret.key.as_str() {
            "binance.api_key" => {
                config.binance.api_key = None;
            }
            "binance.api_secret" => {
                config.binance.api_secret = None;
            }
            "ai.claude_api_key" => {
                if let Some(ref mut ai) = config.ai {
                    ai.claude_api_key = None;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Prompt for confirmation
pub fn confirm_migration(secret_count: usize) -> Result<bool> {
    println!();
    print!(
        "  Migrate {} secret{} to macOS Keychain? [y/N] ",
        secret_count,
        if secret_count == 1 { "" } else { "s" }
    );
    io::stdout().flush().map_err(CryptofolioError::Io)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(CryptofolioError::Io)?;

    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

/// Full migration workflow
#[cfg(target_os = "macos")]
pub async fn run_migration(keychain_repo: &KeychainKeyRepository) -> Result<()> {
    println!();
    println!("  ╔═══════════════════════════════════════════════════════════════╗");
    println!("  ║           MIGRATE SECRETS TO macOS KEYCHAIN                  ║");
    println!("  ╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Load current config
    let mut config = AppConfig::load()?;

    // Find secrets
    let secrets = find_secrets_in_config(&config);

    if secrets.is_empty() {
        println!("  No secrets found in config.toml");
        println!();
        println!("  All secrets are already in the keychain, or none are configured.");
        println!();
        return Ok(());
    }

    println!("  Found {} secret{} in config.toml:", secrets.len(), if secrets.len() == 1 { "" } else { "s" });
    for secret in &secrets {
        println!("    • {}", secret.display_name);
    }
    println!();

    // Check Touch ID availability
    let keychain = get_keychain();
    let is_touchid_available = keychain.is_touchid_available();

    if !is_touchid_available {
        println!("  ⚠️  Touch ID not available (SSH session or no hardware)");
        println!("      Using Standard security level");
        println!();
    }

    // Prompt for security level
    let security_level = prompt_security_level(is_touchid_available)?;

    // Confirm migration
    if !confirm_migration(secrets.len())? {
        println!();
        println!("  Cancelled. No changes made.");
        println!();
        return Err(CryptofolioError::OperationCancelled);
    }

    println!();

    // Create backup
    create_backup()?;

    // Migrate each secret
    for secret in &secrets {
        migrate_secret_to_keychain(&config, secret, security_level, keychain_repo).await?;
    }

    // Clear secrets from config
    clear_secrets_from_config(&mut config, &secrets)?;

    // Save updated config
    config.save()?;
    println!("  ✓ Cleared secrets from config.toml");

    println!();
    println!("  ✅ Migration complete!");
    println!();
    println!("  Your secrets are now protected by macOS Keychain.");
    if security_level.requires_touchid() {
        println!("  You'll be prompted for Touch ID when accessing them.");
    }
    println!();

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub async fn run_migration(_keychain_repo: &KeychainKeyRepository) -> Result<()> {
    Err(CryptofolioError::KeychainNotAvailable)
}
