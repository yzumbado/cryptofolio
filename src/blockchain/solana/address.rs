/// Solana address validation.
///
/// Solana addresses are base58-encoded 32-byte Ed25519 public keys.
/// They look like: `vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg`
use crate::error::{CryptofolioError, Result};

/// Validate a Solana address.
///
/// Rules:
/// - Must be valid base58
/// - Decoded bytes must be exactly 32 (Ed25519 public key size)
pub fn validate_address(address: &str) -> Result<()> {
    let decoded = bitcoin::base58::decode(address).map_err(|_| {
        CryptofolioError::InvalidAddress(format!(
            "Invalid Solana address: not valid base58 — {}",
            address
        ))
    })?;

    // base58::decode from the bitcoin crate appends a 4-byte checksum;
    // raw Solana addresses have no checksum so the decoded length is exactly 32.
    if decoded.len() != 32 {
        return Err(CryptofolioError::InvalidAddress(format!(
            "Invalid Solana address: expected 32 bytes, got {} — {}",
            decoded.len(),
            address
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known Solana System Program address — always valid.
    const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
    // A realistic mainnet address.
    const VALID_ADDR: &str = "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg";

    #[test]
    fn test_valid_system_program() {
        assert!(validate_address(SYSTEM_PROGRAM).is_ok());
    }

    #[test]
    fn test_valid_mainnet_address() {
        assert!(validate_address(VALID_ADDR).is_ok());
    }

    #[test]
    fn test_rejects_ethereum_address() {
        assert!(validate_address("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").is_err());
    }

    #[test]
    fn test_rejects_empty_string() {
        assert!(validate_address("").is_err());
    }

    #[test]
    fn test_rejects_too_short() {
        assert!(validate_address("abc123").is_err());
    }
}
