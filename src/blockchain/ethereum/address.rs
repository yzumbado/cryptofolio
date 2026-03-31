/// Ethereum address validation
use crate::error::{CryptofolioError, Result};

/// Validate an Ethereum address
pub fn validate_address(address: &str) -> Result<()> {
    if address.is_empty() {
        return Err(CryptofolioError::Other("Empty address".to_string()));
    }

    // Ethereum addresses must start with 0x
    if !address.starts_with("0x") && !address.starts_with("0X") {
        return Err(CryptofolioError::Other(
            "Invalid Ethereum address: must start with 0x".to_string(),
        ));
    }

    // Must be exactly 42 characters (0x + 40 hex chars)
    if address.len() != 42 {
        return Err(CryptofolioError::Other(
            "Invalid Ethereum address: must be 42 characters (0x + 40 hex digits)".to_string(),
        ));
    }

    // Validate hex characters
    let hex_part = &address[2..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CryptofolioError::Other(
            "Invalid Ethereum address: contains non-hexadecimal characters".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_address() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_validate_valid_address_lowercase() {
        let addr = "0x742d35cc6634c0532925a3b844bc9e7595f0beb0";
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_validate_valid_address_uppercase() {
        let addr = "0x742D35CC6634C0532925A3B844BC9E7595F0BEB0";
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_invalid_address_no_prefix() {
        let addr = "742d35Cc6634C0532925a3b844Bc9e7595f0bEb";
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_invalid_address_too_short() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e759";
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_invalid_address_too_long() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb123";
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_invalid_address_non_hex() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEg"; // 'g' is not hex
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_invalid_address_empty() {
        assert!(validate_address("").is_err());
    }
}
