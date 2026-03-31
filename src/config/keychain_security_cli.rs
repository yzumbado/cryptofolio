//! macOS Keychain implementation using `security` command-line tool
//!
//! This implementation uses the `security` command instead of Security Framework FFI,
//! which allows it to work without code signing entitlements.
//!
//! Features:
//! - Standard keychain storage (unlocked with Mac login)
//! - No code signing required (uses system `security` command)
//! - Session caching (15-minute timeout to avoid repeated prompts)
//! - Automatic user approval on first access
//!
//! Limitations:
//! - Touch ID support limited (requires interactive password entry)
//! - User must approve keychain access via system dialog

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::keychain::{KeychainSecurityLevel, KeychainStorage};
use crate::error::{CryptofolioError, Result};

/// Service name for keychain entries
const SERVICE_NAME: &str = "com.cryptofolio.api-keys";

/// Session cache timeout (15 minutes)
const CACHE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Session cache entry
#[derive(Clone)]
struct CacheEntry {
    value: String,
    cached_at: Instant,
}

/// macOS Keychain implementation using `security` CLI
pub struct SecurityCliKeychain {
    /// Session cache to avoid repeated prompts
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl SecurityCliKeychain {
    /// Create a new SecurityCliKeychain instance
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a cached value is still valid
    fn get_cached(&self, key: &str) -> Option<String> {
        let cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                return None;
            }
        };

        if let Some(entry) = cache.get(key) {
            if entry.cached_at.elapsed() < CACHE_TIMEOUT {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Store a value in the session cache
    fn set_cached(&self, key: &str, value: String) {
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                guard
            }
        };

        cache.insert(
            key.to_string(),
            CacheEntry {
                value,
                cached_at: Instant::now(),
            },
        );
    }

    /// Clear a value from the session cache
    fn clear_cached(&self, key: &str) {
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                return;
            }
        };

        cache.remove(key);
    }

    /// Clear expired cache entries
    fn cleanup_cache(&self) {
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                return;
            }
        };

        cache.retain(|_, entry| entry.cached_at.elapsed() < CACHE_TIMEOUT);
    }
}

impl Default for SecurityCliKeychain {
    fn default() -> Self {
        Self::new()
    }
}

impl KeychainStorage for SecurityCliKeychain {
    fn store(&self, key: &str, secret: &str) -> Result<()> {
        self.store_with_security(key, secret, KeychainSecurityLevel::Standard)
    }

    fn store_with_security(
        &self,
        key: &str,
        secret: &str,
        level: KeychainSecurityLevel,
    ) -> Result<()> {
        // Cleanup old cache entries
        self.cleanup_cache();

        // Delete existing entry first (keychain requires this for updates)
        let _ = self.delete(key);

        // Build security command
        // Note: Touch ID support via CLI is limited - we use standard keychain for all levels
        if level.requires_touchid() {
            eprintln!(
                "Note: Touch ID protection via `security` command is limited. \
                 Using standard keychain security. For full Touch ID support, \
                 sign the binary with Apple Developer ID."
            );
        }

        let output = Command::new("security")
            .args(&[
                "add-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                key,
                "-w",
                secret,
                "-U", // Update if exists (though we deleted above)
            ])
            .output()
            .map_err(|e| {
                CryptofolioError::Keychain(format!(
                    "Failed to run security command: {}. Is macOS security command available?",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CryptofolioError::Keychain(format!(
                "Failed to store secret '{}': {}",
                key, stderr
            )));
        }

        // Clear cache when updating
        self.clear_cached(key);

        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<String> {
        // Check cache first
        if let Some(cached) = self.get_cached(key) {
            return Ok(cached);
        }

        // Retrieve from keychain using security command
        let output = Command::new("security")
            .args(&[
                "find-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                key,
                "-w", // Print password only
            ])
            .output()
            .map_err(|e| {
                CryptofolioError::Keychain(format!(
                    "Failed to run security command: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("could not be found") {
                return Err(CryptofolioError::Keychain(format!(
                    "Secret '{}' not found in keychain. Use 'config set-secret {}' to configure it.",
                    key, key
                )));
            } else {
                return Err(CryptofolioError::Keychain(format!(
                    "Failed to retrieve '{}' from keychain: {}",
                    key, stderr
                )));
            }
        }

        let secret = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        if secret.is_empty() {
            return Err(CryptofolioError::Keychain(format!(
                "Secret '{}' is empty in keychain",
                key
            )));
        }

        // Cache for session
        self.set_cached(key, secret.clone());

        Ok(secret)
    }

    fn delete(&self, key: &str) -> Result<()> {
        let output = Command::new("security")
            .args(&[
                "delete-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                key,
            ])
            .output()
            .map_err(|e| {
                CryptofolioError::Keychain(format!(
                    "Failed to run security command: {}",
                    e
                ))
            })?;

        // Don't error if not found - deletion is idempotent
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("could not be found") {
                return Err(CryptofolioError::Keychain(format!(
                    "Failed to delete secret '{}': {}",
                    key, stderr
                )));
            }
        }

        // Clear from cache
        self.clear_cached(key);

        Ok(())
    }

    fn get_security_level(&self, key: &str) -> Result<KeychainSecurityLevel> {
        // Security CLI doesn't provide security level information
        // Default to Standard for existing items
        if self.exists(key) {
            Ok(KeychainSecurityLevel::Standard)
        } else {
            Err(CryptofolioError::Keychain(format!(
                "Secret '{}' not found in keychain",
                key
            )))
        }
    }

    fn update_security_level(&self, key: &str, level: KeychainSecurityLevel) -> Result<()> {
        // Retrieve the current secret
        let secret = self.retrieve(key)?;

        // Re-store with new security level
        self.store_with_security(key, &secret, level)
    }

    fn is_touchid_available(&self) -> bool {
        // Check if we're in an SSH session (Touch ID won't work)
        if std::env::var("SSH_CONNECTION").is_ok() || std::env::var("SSH_CLIENT").is_ok() {
            return false;
        }

        // Security CLI has limited Touch ID support
        // Return false to indicate full Touch ID features not available
        false
    }

    fn exists(&self, key: &str) -> bool {
        let output = Command::new("security")
            .args(&[
                "find-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                key,
                "-w",
            ])
            .output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_cli_keychain_new() {
        let keychain = SecurityCliKeychain::new();
        assert!(keychain.cache.lock().unwrap().is_empty());
    }

    #[test]
    fn test_cache_operations() {
        let keychain = SecurityCliKeychain::new();

        // Initially no cached value
        assert!(keychain.get_cached("test_key").is_none());

        // Set cached value
        keychain.set_cached("test_key", "test_value".to_string());

        // Should be cached now
        assert_eq!(
            keychain.get_cached("test_key"),
            Some("test_value".to_string())
        );

        // Clear cache
        keychain.clear_cached("test_key");

        // Should be gone
        assert!(keychain.get_cached("test_key").is_none());
    }

    #[test]
    fn test_ssh_detection() {
        let keychain = SecurityCliKeychain::new();

        // Set SSH env var
        std::env::set_var("SSH_CONNECTION", "1");

        // Touch ID should not be available in SSH
        assert!(!keychain.is_touchid_available());

        // Clean up
        std::env::remove_var("SSH_CONNECTION");
    }
}
