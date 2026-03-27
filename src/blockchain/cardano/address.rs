/// Cardano address validation
use crate::error::Result;

/// Validate a Cardano address
///
/// Cardano addresses use Bech32 encoding with specific prefixes:
/// - Mainnet: addr1... (payment), stake1... (stake)
/// - Testnet: addr_test1... (payment), stake_test1... (stake)
pub fn validate_address(address: &str) -> Result<()> {
    // Check if address starts with valid prefix
    let valid_prefixes = [
        "addr1",       // Mainnet payment address
        "addr_test1",  // Testnet payment address
        "stake1",      // Mainnet stake address
        "stake_test1", // Testnet stake address
    ];

    let has_valid_prefix = valid_prefixes.iter().any(|prefix| address.starts_with(prefix));

    if !has_valid_prefix {
        return Err(crate::error::CryptofolioError::InvalidAddress(
            "Invalid Cardano address: must start with addr1, addr_test1, stake1, or stake_test1".to_string()
        ));
    }

    // Basic length check (Cardano addresses are typically 103-108 characters)
    if address.len() < 50 || address.len() > 150 {
        return Err(crate::error::CryptofolioError::InvalidAddress(
            "Invalid Cardano address: incorrect length".to_string()
        ));
    }

    // Check for valid Bech32 characters (lowercase alphanumeric, underscore for testnet prefix, no uppercase)
    if !address.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(crate::error::CryptofolioError::InvalidAddress(
            "Invalid Cardano address: contains invalid characters".to_string()
        ));
    }

    // TODO: Full Bech32 checksum validation
    // The bech32 crate validation requires careful handling of Cardano's specific Bech32 format
    // For now, we rely on prefix, length, and character validation
    // Future enhancement: Implement proper Bech32 checksum verification

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_mainnet_address() {
        // Note: Using a test address - in production, only real addresses with valid checksums should be used
        let addr = "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa";
        // This will pass basic validation (prefix, length, characters)
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_valid_testnet_address() {
        // Note: Using a test address - in production, only real addresses with valid checksums should be used
        let addr = "addr_test1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qgp3r9e";
        // This will pass basic validation (prefix, length, characters)
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_invalid_prefix() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_too_short() {
        let addr = "addr1qxy2";
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_uppercase_characters() {
        let addr = "ADDR1QXY2KGDYGJRSQTZQ2N0YRF2493P83KKFJHX0WLH0TCP5DC2UKMUQJJW0APG6K8XFN63T8Y9P2L3W8W5Z2X7JN8SQF3QVWA";
        assert!(validate_address(addr).is_err());
    }
}
