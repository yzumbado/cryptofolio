//! Keychain storage abstraction for secure credential management
//!
//! This module provides a platform-independent interface for storing and retrieving
//! secrets using the operating system's native keychain/credential manager.
//!
//! Supported platforms:
//! - macOS: Keychain with Touch ID support
//! - Linux/Windows: TOML fallback (keychain support coming in future versions)

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Security level for keychain-stored secrets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeychainSecurityLevel {
    /// Standard keychain access (unlocked with Mac login)
    /// - Good for: Automated scripts, cron jobs
    /// - Security: Protected by macOS encryption, accessible when Mac is unlocked
    Standard,

    /// Require Touch ID OR device password for access
    /// - Good for: Interactive use, balance of convenience and security
    /// - Security: Requires biometric or password authentication
    /// - Recommended for most users
    #[serde(rename = "touchid")]
    TouchIdProtected,

    /// ONLY Touch ID authentication (no password fallback)
    /// - Good for: Maximum security, high-value accounts
    /// - Security: Only biometric authentication accepted
    /// - Warning: May not work in SSH sessions or without Touch ID hardware
    #[serde(rename = "touchid-only")]
    TouchIdOnly,
}

impl KeychainSecurityLevel {
    /// Convert from string (for CLI parsing)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "standard" => Some(Self::Standard),
            "touchid" | "touchid-protected" => Some(Self::TouchIdProtected),
            "touchid-only" => Some(Self::TouchIdOnly),
            _ => None,
        }
    }

    /// Convert to user-friendly display string
    pub fn as_display_str(&self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::TouchIdProtected => "Touch ID Protected",
            Self::TouchIdOnly => "Touch ID Only",
        }
    }

    /// Convert to database string
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::TouchIdProtected => "touchid",
            Self::TouchIdOnly => "touchid-only",
        }
    }

    /// Check if this level requires Touch ID
    pub fn requires_touchid(&self) -> bool {
        matches!(self, Self::TouchIdProtected | Self::TouchIdOnly)
    }
}

/// Keychain storage backend trait
///
/// Implementations should handle:
/// - Secure storage in OS-native credential manager
/// - Touch ID/biometric authentication (where supported)
/// - Session caching to avoid repeated prompts
/// - Graceful fallback when keychain is unavailable
#[cfg(target_os = "macos")]
pub trait KeychainStorage {
    /// Store a secret in the keychain with standard security
    fn store(&self, key: &str, secret: &str) -> Result<()>;

    /// Store a secret with specific security level (Touch ID support)
    fn store_with_security(
        &self,
        key: &str,
        secret: &str,
        level: KeychainSecurityLevel,
    ) -> Result<()>;

    /// Retrieve a secret from the keychain
    /// May trigger Touch ID prompt if configured
    fn retrieve(&self, key: &str) -> Result<String>;

    /// Delete a secret from the keychain
    fn delete(&self, key: &str) -> Result<()>;

    /// Get the security level of a stored secret
    fn get_security_level(&self, key: &str) -> Result<KeychainSecurityLevel>;

    /// Update the security level of an existing secret
    fn update_security_level(&self, key: &str, level: KeychainSecurityLevel) -> Result<()>;

    /// Check if Touch ID is available on this system
    fn is_touchid_available(&self) -> bool;

    /// Check if a key exists in the keychain
    fn exists(&self, key: &str) -> bool;
}

/// Get the default keychain implementation for this platform
#[cfg(target_os = "macos")]
pub fn get_keychain() -> Box<dyn KeychainStorage> {
    Box::new(super::keychain_macos::MacOSKeychain::new())
}

/// Check if keychain is available on this platform
pub fn is_keychain_available() -> bool {
    cfg!(target_os = "macos")
}

/// Get platform name for keychain
pub fn platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS Keychain"
    } else if cfg!(target_os = "linux") {
        "Linux Secret Service (not yet implemented)"
    } else if cfg!(target_os = "windows") {
        "Windows Credential Manager (not yet implemented)"
    } else {
        "Unknown platform"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_level_from_str() {
        assert_eq!(
            KeychainSecurityLevel::from_str("standard"),
            Some(KeychainSecurityLevel::Standard)
        );
        assert_eq!(
            KeychainSecurityLevel::from_str("touchid"),
            Some(KeychainSecurityLevel::TouchIdProtected)
        );
        assert_eq!(
            KeychainSecurityLevel::from_str("touchid-protected"),
            Some(KeychainSecurityLevel::TouchIdProtected)
        );
        assert_eq!(
            KeychainSecurityLevel::from_str("touchid-only"),
            Some(KeychainSecurityLevel::TouchIdOnly)
        );
        assert_eq!(KeychainSecurityLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_security_level_display() {
        assert_eq!(KeychainSecurityLevel::Standard.as_display_str(), "Standard");
        assert_eq!(
            KeychainSecurityLevel::TouchIdProtected.as_display_str(),
            "Touch ID Protected"
        );
        assert_eq!(
            KeychainSecurityLevel::TouchIdOnly.as_display_str(),
            "Touch ID Only"
        );
    }

    #[test]
    fn test_requires_touchid() {
        assert!(!KeychainSecurityLevel::Standard.requires_touchid());
        assert!(KeychainSecurityLevel::TouchIdProtected.requires_touchid());
        assert!(KeychainSecurityLevel::TouchIdOnly.requires_touchid());
    }

    #[test]
    fn test_platform_detection() {
        #[cfg(target_os = "macos")]
        assert!(is_keychain_available());

        #[cfg(not(target_os = "macos"))]
        assert!(!is_keychain_available());
    }
}
