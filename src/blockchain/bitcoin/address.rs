/// Bitcoin address validation
///
/// Validates Bitcoin addresses for different formats:
/// - Legacy (P2PKH): starts with '1'
/// - P2SH: starts with '3'
/// - Bech32 (SegWit): starts with 'bc1'
/// - Testnet: starts with 'm', 'n', '2', or 'tb1'

use crate::error::{CryptofolioError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    P2PKH,      // Legacy (1...)
    P2SH,       // Script hash (3...)
    Bech32,     // Native SegWit (bc1...)
    Testnet,    // Testnet addresses
}

/// Validate a Bitcoin address (mainnet or testnet)
pub fn validate_address(address: &str) -> Result<AddressType> {
    if address.is_empty() {
        return Err(CryptofolioError::Other("Empty address".to_string()));
    }

    // Check length constraints
    if address.len() < 26 || address.len() > 90 {
        return Err(CryptofolioError::Other(
            "Invalid Bitcoin address: incorrect length".to_string(),
        ));
    }

    // Determine address type by prefix
    let addr_type = if address.starts_with("bc1") || address.starts_with("BC1") {
        AddressType::Bech32
    } else if address.starts_with("tb1") || address.starts_with("TB1") {
        AddressType::Testnet
    } else if address.starts_with('1') {
        AddressType::P2PKH
    } else if address.starts_with('3') {
        AddressType::P2SH
    } else if address.starts_with('m') || address.starts_with('n') || address.starts_with('2') {
        AddressType::Testnet
    } else {
        return Err(CryptofolioError::Other(
            "Invalid Bitcoin address: unknown format".to_string(),
        ));
    };

    // Validate character set
    match addr_type {
        AddressType::Bech32 | AddressType::Testnet if address.starts_with("bc1") || address.starts_with("BC1") || address.starts_with("tb1") || address.starts_with("TB1") => {
            // Bech32 validation - alphanumeric excluding 'b', 'i', 'o' (except in prefix)
            // Bech32 charset: qpzry9x8gf2tvdw0s3jn54khce6mua7l
            let lower = address.to_lowercase();
            let valid_bech32_chars = "qpzry9x8gf2tvdw0s3jn54khce6mua7lbc1t";
            if !lower.chars().all(|c| valid_bech32_chars.contains(c)) {
                return Err(CryptofolioError::Other(
                    "Invalid Bitcoin address: invalid bech32 characters".to_string(),
                ));
            }
        }
        _ => {
            // Base58 validation - alphanumeric excluding '0', 'O', 'I', 'l'
            if !address.chars().all(|c| {
                c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
            }) {
                return Err(CryptofolioError::Other(
                    "Invalid Bitcoin address: invalid base58 characters".to_string(),
                ));
            }
        }
    }

    Ok(addr_type)
}

/// Check if address is a testnet address
pub fn is_testnet_address(address: &str) -> bool {
    address.starts_with("tb1")
        || address.starts_with("TB1")
        || address.starts_with('m')
        || address.starts_with('n')
        || address.starts_with('2')
}

/// Validate an extended public key (xpub, ypub, zpub)
pub fn validate_xpub(xpub: &str) -> Result<()> {
    if xpub.is_empty() {
        return Err(CryptofolioError::Other("Empty xpub".to_string()));
    }

    // Check prefix
    let valid_prefixes = ["xpub", "ypub", "zpub", "tpub", "upub", "vpub"];
    if !valid_prefixes.iter().any(|&prefix| xpub.starts_with(prefix)) {
        return Err(CryptofolioError::Other(
            "Invalid xpub: must start with xpub, ypub, zpub, tpub, upub, or vpub".to_string(),
        ));
    }

    // Check length (should be 111 characters for most xpubs)
    if xpub.len() < 100 || xpub.len() > 120 {
        return Err(CryptofolioError::Other(
            "Invalid xpub: incorrect length".to_string(),
        ));
    }

    // Validate base58 characters
    if !xpub.chars().all(|c| {
        c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
    }) {
        return Err(CryptofolioError::Other(
            "Invalid xpub: invalid base58 characters".to_string(),
        ));
    }

    Ok(())
}

/// Check if xpub is for testnet
pub fn is_testnet_xpub(xpub: &str) -> bool {
    xpub.starts_with("tpub") || xpub.starts_with("upub") || xpub.starts_with("vpub")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_legacy_address() {
        let addr = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"; // Genesis block address
        assert!(validate_address(addr).is_ok());
        assert_eq!(validate_address(addr).unwrap(), AddressType::P2PKH);
    }

    #[test]
    fn test_validate_p2sh_address() {
        let addr = "3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy";
        assert!(validate_address(addr).is_ok());
        assert_eq!(validate_address(addr).unwrap(), AddressType::P2SH);
    }

    #[test]
    fn test_validate_bech32_address() {
        let addr = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
        assert!(validate_address(addr).is_ok());
        assert_eq!(validate_address(addr).unwrap(), AddressType::Bech32);
    }

    #[test]
    fn test_invalid_address_empty() {
        assert!(validate_address("").is_err());
    }

    #[test]
    fn test_invalid_address_too_short() {
        assert!(validate_address("1A1zP1eP").is_err());
    }

    #[test]
    fn test_invalid_address_bad_chars() {
        assert!(validate_address("invalid_address").is_err());
    }

    #[test]
    fn test_invalid_address_base58_forbidden_chars() {
        // Contains '0' which is not allowed in base58
        assert!(validate_address("1A1zP1eP5QGefi2DMPTfTL5SLmv70ivfNa").is_err());
    }

    #[test]
    fn test_validate_xpub() {
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        assert!(validate_xpub(xpub).is_ok());
    }

    #[test]
    fn test_validate_zpub() {
        let zpub = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvphXR5ePCqYAN5qRbNnCLanT9qDKnNT4yKYr8j6L51HvvPahBJPJJZpNAQTwD";
        assert!(validate_xpub(zpub).is_ok());
    }

    #[test]
    fn test_invalid_xpub_wrong_prefix() {
        assert!(validate_xpub("wrongprefix123").is_err());
    }

    #[test]
    fn test_invalid_xpub_too_short() {
        assert!(validate_xpub("xpub123").is_err());
    }

    #[test]
    fn test_testnet_bech32_address() {
        let addr = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";
        assert!(validate_address(addr).is_ok());
        assert_eq!(validate_address(addr).unwrap(), AddressType::Testnet);
        assert!(is_testnet_address(addr));
    }

    #[test]
    fn test_testnet_legacy_address() {
        let addr = "mjSk1Ny9spzU2fouzYgLqGUD8U41iR35QN";
        assert!(validate_address(addr).is_ok());
        assert_eq!(validate_address(addr).unwrap(), AddressType::Testnet);
        assert!(is_testnet_address(addr));
    }

    #[test]
    fn test_mainnet_not_testnet() {
        let addr = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
        assert!(!is_testnet_address(addr));
    }

    #[test]
    fn test_testnet_xpub() {
        let tpub = "tpubD6NzVbkrYhZ4XgiXtGrdW5XDAPFCL9h7we1vwNCpn8tGbBcgfVYjXyhWo4E1xkh56hjod1RhGjxbaTLV3X4FyWuejifB9jusQ46QzG87VKp";
        assert!(validate_xpub(tpub).is_ok());
        assert!(is_testnet_xpub(tpub));
    }

    #[test]
    fn test_mainnet_xpub_not_testnet() {
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        assert!(!is_testnet_xpub(xpub));
    }
}
