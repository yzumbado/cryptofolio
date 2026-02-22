//! macOS Keychain implementation with Touch ID support
//!
//! This module provides secure credential storage using macOS Keychain Services
//! with optional Touch ID/biometric authentication.
//!
//! Features:
//! - Standard keychain storage (unlocked with Mac login)
//! - Touch ID protected storage (requires biometric or password)
//! - Touch ID only storage (biometric authentication only)
//! - Session caching (15-minute timeout to avoid repeated prompts)
//! - Automatic fallback when Touch ID unavailable

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::keychain::{KeychainSecurityLevel, KeychainStorage};
use super::keychain_ffi::{
    create_access_control, keychain_add_password, keychain_delete_password, keychain_get_password,
    K_SEC_ACCESS_CONTROL_BIOMETRY_ANY, K_SEC_ACCESS_CONTROL_USER_PRESENCE,
};
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

/// macOS Keychain implementation
pub struct MacOSKeychain {
    /// Session cache to avoid repeated Touch ID prompts
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl MacOSKeychain {
    /// Create a new macOS Keychain instance
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a cached value is still valid
    fn get_cached(&self, key: &str) -> Option<String> {
        let cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.cached_at.elapsed() < CACHE_TIMEOUT {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Store a value in the session cache
    fn set_cached(&self, key: &str, value: String) {
        let mut cache = self.cache.lock().unwrap();
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
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key);
    }

    /// Clear expired cache entries
    fn cleanup_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, entry| entry.cached_at.elapsed() < CACHE_TIMEOUT);
    }
}

impl Default for MacOSKeychain {
    fn default() -> Self {
        Self::new()
    }
}

impl KeychainStorage for MacOSKeychain {
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
        let _ = keychain_delete_password(SERVICE_NAME, key);

        // Create access control based on security level
        let (access_control, prompt) = match level {
            KeychainSecurityLevel::Standard => {
                // Standard keychain access (no Touch ID)
                (None, None)
            }
            KeychainSecurityLevel::TouchIdProtected => {
                // Touch ID OR password fallback
                if !self.is_touchid_available() {
                    eprintln!("Warning: Touch ID not available. Falling back to standard keychain.");
                    (None, None)
                } else {
                    let ac = create_access_control(K_SEC_ACCESS_CONTROL_USER_PRESENCE)
                        .map_err(|e| {
                            CryptofolioError::Keychain(format!(
                                "Failed to create Touch ID access control: {}",
                                e
                            ))
                        })?;
                    (
                        Some(ac),
                        Some("Cryptofolio needs access to this secret"),
                    )
                }
            }
            KeychainSecurityLevel::TouchIdOnly => {
                // Touch ID ONLY (no password fallback)
                if !self.is_touchid_available() {
                    return Err(CryptofolioError::Keychain(
                        "Touch ID not available. Cannot store with TouchIdOnly security level."
                            .to_string(),
                    ));
                }
                let ac =
                    create_access_control(K_SEC_ACCESS_CONTROL_BIOMETRY_ANY).map_err(|e| {
                        CryptofolioError::Keychain(format!(
                            "Failed to create Touch ID access control: {}",
                            e
                        ))
                    })?;
                (
                    Some(ac),
                    Some("Cryptofolio needs access to this secret (Touch ID required)"),
                )
            }
        };

        // Store the password with access control
        keychain_add_password(SERVICE_NAME, key, secret, access_control, prompt).map_err(
            |e| CryptofolioError::Keychain(format!("Failed to store secret '{}': {}", key, e)),
        )?;

        // Clear cache when updating
        self.clear_cached(key);

        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<String> {
        // Check cache first
        if let Some(cached) = self.get_cached(key) {
            return Ok(cached);
        }

        // Retrieve from keychain (may trigger Touch ID prompt)
        let prompt = format!("Cryptofolio needs access to \"{}\"", key);
        let secret = keychain_get_password(SERVICE_NAME, key, Some(&prompt)).map_err(|e| {
            // Convert FFI errors to CryptofolioError
            let err_str = e.to_string();
            if err_str.contains("not found") {
                CryptofolioError::Keychain(format!(
                    "Secret '{}' not found in keychain. Use 'config set-secret {}' to configure it.",
                    key, key
                ))
            } else if err_str.contains("canceled") {
                CryptofolioError::KeychainAuthCancelled(
                    "Touch ID authentication was canceled by user".to_string(),
                )
            } else if err_str.contains("failed") {
                CryptofolioError::Keychain(format!("Touch ID authentication failed for '{}'", key))
            } else {
                CryptofolioError::Keychain(format!(
                    "Failed to retrieve '{}' from keychain: {}",
                    key, err_str
                ))
            }
        })?;

        // Cache for session
        self.set_cached(key, secret.clone());

        Ok(secret)
    }

    fn delete(&self, key: &str) -> Result<()> {
        keychain_delete_password(SERVICE_NAME, key).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("not found") {
                CryptofolioError::Keychain(format!("Secret '{}' not found in keychain", key))
            } else {
                CryptofolioError::Keychain(format!("Failed to delete secret '{}': {}", key, e))
            }
        })?;

        // Clear from cache
        self.clear_cached(key);

        Ok(())
    }

    fn get_security_level(&self, key: &str) -> Result<KeychainSecurityLevel> {
        // The security level is tracked in the database, not in the keychain itself
        // This method would need to query the database, but we keep it simple here
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

        // Check if Touch ID hardware is available
        // We do this by trying to create an access control - if it succeeds, Touch ID is available
        create_access_control(K_SEC_ACCESS_CONTROL_USER_PRESENCE).is_ok()
    }

    fn exists(&self, key: &str) -> bool {
        keychain_get_password(SERVICE_NAME, key, None).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_keychain_new() {
        let keychain = MacOSKeychain::new();
        assert!(keychain.cache.lock().unwrap().is_empty());
    }

    #[test]
    fn test_cache_operations() {
        let keychain = MacOSKeychain::new();

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
        let keychain = MacOSKeychain::new();

        // Set SSH env var
        std::env::set_var("SSH_CONNECTION", "1");

        // Touch ID should not be available in SSH
        assert!(!keychain.is_touchid_available());

        // Clean up
        std::env::remove_var("SSH_CONNECTION");
    }

    // Note: Integration tests with actual keychain access
    // should be run manually as they require user interaction
}
