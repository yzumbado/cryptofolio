use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::error::{CryptofolioError, Result};

/// Check if a config key is a secret/credential
pub fn is_secret_key(key: &str) -> bool {
    let key_lower = key.to_lowercase();
    key_lower.contains("api_key")
        || key_lower.contains("api_secret")
        || key_lower.contains("secret_key")
        || key_lower.contains("password")
        || key_lower.contains("token")
        || key_lower.contains("secret")
}

/// Check if a config key is specifically an API credential
pub fn is_api_credential_key(key: &str) -> bool {
    let key_lower = key.to_lowercase();
    key_lower.contains("api_key") || key_lower.contains("api_secret")
}

/// Read secret from stdin (for piped input)
pub fn read_secret_from_stdin() -> Result<String> {
    let mut secret = String::new();
    io::stdin()
        .read_to_string(&mut secret)
        .map_err(|e| CryptofolioError::Io(e))?;

    let trimmed = secret.trim().to_string();
    if trimmed.is_empty() {
        return Err(CryptofolioError::Config("Empty secret provided".into()));
    }

    Ok(trimmed)
}

/// Read secret from file
pub fn read_secret_from_file(path: &Path) -> Result<String> {
    if !path.exists() {
        return Err(CryptofolioError::Config(format!(
            "Secret file not found: {}",
            path.display()
        )));
    }

    let secret = fs::read_to_string(path)
        .map_err(|e| CryptofolioError::Config(format!(
            "Failed to read secret file: {}",
            e
        )))?;

    let trimmed = secret.trim().to_string();
    if trimmed.is_empty() {
        return Err(CryptofolioError::Config("Empty secret in file".into()));
    }

    Ok(trimmed)
}

/// Read secret from environment variable
pub fn read_secret_from_env(env_var: &str) -> Result<String> {
    std::env::var(env_var)
        .map_err(|_| CryptofolioError::Config(format!(
            "Environment variable not found: {}",
            env_var
        )))
        .and_then(|val| {
            if val.trim().is_empty() {
                Err(CryptofolioError::Config(format!(
                    "Environment variable is empty: {}",
                    env_var
                )))
            } else {
                Ok(val.trim().to_string())
            }
        })
}

/// Read secret interactively with hidden input
pub fn read_secret_interactive(key: &str) -> Result<String> {
    let prompt = format!("Enter {} (hidden): ", key);

    let secret = rpassword::prompt_password(prompt)
        .map_err(|e| CryptofolioError::Config(format!(
            "Failed to read password: {}",
            e
        )))?;

    if secret.trim().is_empty() {
        return Err(CryptofolioError::Config("Empty secret provided".into()));
    }

    Ok(secret.trim().to_string())
}

/// Show security warning before setting API credentials
pub fn show_security_warning(key: &str) -> Result<()> {
    // Only show for API credentials
    if !is_api_credential_key(key) {
        return Ok(());
    }

    println!();
    println!("  ╔═══════════════════════════════════════════════════════════════╗");
    println!("  ║                     SECURITY NOTICE                           ║");
    println!("  ╚═══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Cryptofolio v0.2 stores API keys in PLAINTEXT on your filesystem:");
    println!("    ~/.config/cryptofolio/config.toml");
    println!();
    println!("  ⚠️  IMPORTANT: Use READ-ONLY API keys ONLY");
    println!();
    println!("  When creating your Binance API key:");
    println!("    ✅ Enable: 'Enable Reading'");
    println!("    ❌ DISABLE: 'Enable Spot & Margin Trading'");
    println!("    ❌ DISABLE: 'Enable Withdrawals'");
    println!("    ❌ DISABLE: 'Enable Internal Transfer'");
    println!();
    println!("  Why? If your computer is compromised, attackers could:");
    println!("    • Read your API key from config.toml");
    println!("    • Use WRITE permissions to steal funds");
    println!();
    println!("  With READ-ONLY keys, they can only:");
    println!("    • View your portfolio (no financial loss)");
    println!();
    println!("  🔮 Coming in v0.3: Encrypted keychain storage");
    println!("     (macOS Keychain, Windows Credential Manager, Linux Secret Service)");
    println!();

    // Require acknowledgment
    print!("  I understand, continue? [y/N] ");
    io::stdout().flush().map_err(|e| CryptofolioError::Io(e))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| CryptofolioError::Io(e))?;

    if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
        println!();
        println!("  Cancelled. No changes made.");
        println!();
        return Err(CryptofolioError::OperationCancelled);
    }

    println!();
    Ok(())
}

/// Ensure config file has secure permissions (Unix only)
#[cfg(unix)]
pub fn ensure_secure_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)
        .map_err(|e| CryptofolioError::Io(e))?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if file is readable by group or others (not 0600 or 0400)
    if mode & 0o077 != 0 {
        eprintln!();
        eprintln!("  ⚠️  WARNING: Config file has insecure permissions!");
        eprintln!("     File: {}", path.display());
        eprintln!("     Current: {:o}", mode & 0o777);
        eprintln!();
        eprintln!("  Fixing permissions to 0600 (owner read/write only)...");

        // Fix permissions
        let mut new_permissions = permissions;
        new_permissions.set_mode(0o600);
        fs::set_permissions(path, new_permissions)
            .map_err(|e| CryptofolioError::Io(e))?;

        eprintln!("  ✓ Permissions updated to 0600");
        eprintln!();
    }

    Ok(())
}

#[cfg(not(unix))]
pub fn ensure_secure_permissions(path: &Path) -> Result<()> {
    // Windows: Just warn the user
    eprintln!();
    eprintln!("  ⚠️  Windows detected: Ensure only your user can read:");
    eprintln!("     {}", path.display());
    eprintln!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secret_key() {
        assert!(is_secret_key("binance.api_secret"));
        assert!(is_secret_key("api_key"));
        assert!(is_secret_key("MY_PASSWORD"));
        assert!(is_secret_key("auth_token"));
        assert!(!is_secret_key("general.use_testnet"));
        assert!(!is_secret_key("display.color"));
    }

    #[test]
    fn test_is_api_credential_key() {
        assert!(is_api_credential_key("binance.api_secret"));
        assert!(is_api_credential_key("binance.api_key"));
        assert!(is_api_credential_key("AI.CLAUDE_API_KEY"));
        assert!(!is_api_credential_key("password"));
        assert!(!is_api_credential_key("general.use_testnet"));
    }

    #[test]
    fn test_read_secret_from_env() {
        std::env::set_var("TEST_SECRET_123", "my-test-secret");
        let result = read_secret_from_env("TEST_SECRET_123");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "my-test-secret");

        // Test missing env var
        let result = read_secret_from_env("NONEXISTENT_VAR_XYZ");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_secret_from_env_empty() {
        std::env::set_var("EMPTY_SECRET", "");
        let result = read_secret_from_env("EMPTY_SECRET");
        assert!(result.is_err());
    }
}
