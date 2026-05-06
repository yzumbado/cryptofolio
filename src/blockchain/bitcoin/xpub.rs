/// BIP32 xpub address derivation
///
/// Derives receiving addresses from xpub / ypub / zpub (and their testnet equivalents).
/// The xpub is assumed to be at account level (e.g. m/84'/0'/0'), so derivation
/// continues with the external chain: m/0/0, m/0/1, … m/0/(count-1).
use crate::error::{CryptofolioError, Result};
use bitcoin::bip32::{ChildNumber, Xpub};
use bitcoin::key::CompressedPublicKey;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, Network, PublicKey};
use std::str::FromStr;

/// Address type inferred from the xpub version-byte prefix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XpubAddressType {
    Legacy,        // xpub / tpub  → P2PKH  (1… / m…)
    WrappedSegwit, // ypub / upub  → P2SH-P2WPKH  (3…)
    NativeSegwit,  // zpub / vpub  → P2WPKH bech32 (bc1q…)
    Taproot,       // BIP86 xpub  → P2TR bech32m (bc1p…)
}

/// Derive `count` external-chain receiving addresses from an xpub/ypub/zpub.
///
/// The address type is inferred from the key prefix (xpub → Legacy, ypub →
/// WrappedSegwit, zpub → NativeSegwit). Pass `force_type` to override the
/// inferred type — use `Some(XpubAddressType::Taproot)` for BIP-86 xpubs that
/// share the same version bytes as Legacy keys.
///
/// Returns a `Vec<(address_string, child_index)>`.
pub fn derive_addresses(
    xpub_str: &str,
    is_testnet: bool,
    count: u32,
) -> Result<Vec<(String, u32)>> {
    derive_addresses_with_type(xpub_str, is_testnet, count, None)
}

/// Like [`derive_addresses`] but allows explicitly overriding the address type.
pub fn derive_addresses_with_type(
    xpub_str: &str,
    is_testnet: bool,
    count: u32,
    force_type: Option<XpubAddressType>,
) -> Result<Vec<(String, u32)>> {
    let network = if is_testnet {
        Network::Testnet
    } else {
        Network::Bitcoin
    };

    // Normalise ypub/zpub to the canonical xpub version bytes so the bitcoin
    // crate can parse the key, and remember which address type to produce.
    let (xpub_canonical, inferred_type) = normalize_to_xpub(xpub_str, is_testnet)?;
    let addr_type = force_type.unwrap_or(inferred_type);

    let secp = Secp256k1::verification_only();
    let xpub = Xpub::from_str(&xpub_canonical)
        .map_err(|e| CryptofolioError::Other(format!("Invalid xpub: {}", e)))?;

    // Derive external chain: m/0
    let external_idx = ChildNumber::from_normal_idx(0)
        .map_err(|e| CryptofolioError::Other(format!("Key derivation error: {}", e)))?;
    let external = xpub
        .ckd_pub(&secp, external_idx)
        .map_err(|e| CryptofolioError::Other(format!("Key derivation error: {}", e)))?;

    let mut addrs = Vec::with_capacity(count as usize);
    for i in 0..count {
        let child_idx = ChildNumber::from_normal_idx(i)
            .map_err(|e| CryptofolioError::Other(format!("Invalid child index {}: {}", i, e)))?;
        let child = external.ckd_pub(&secp, child_idx).map_err(|e| {
            CryptofolioError::Other(format!("Key derivation error at index {}: {}", i, e))
        })?;

        let address = match addr_type {
            XpubAddressType::Legacy => Address::p2pkh(PublicKey::new(child.public_key), network),
            XpubAddressType::WrappedSegwit => {
                let compressed = CompressedPublicKey(child.public_key);
                Address::p2shwpkh(&compressed, network)
            }
            XpubAddressType::NativeSegwit => {
                let compressed = CompressedPublicKey(child.public_key);
                Address::p2wpkh(&compressed, network)
            }
            XpubAddressType::Taproot => {
                // BIP-86: x-only pubkey → P2TR (bc1p… on mainnet)
                let x_only = child.to_x_only_pub();
                Address::p2tr(&secp, x_only, None, network)
            }
        };

        addrs.push((address.to_string(), i));
    }

    Ok(addrs)
}

/// Normalise ypub / zpub to xpub version bytes so `bitcoin::bip32::Xpub` can
/// parse it.  Returns the re-encoded string and the inferred address type.
fn normalize_to_xpub(xpub_str: &str, is_testnet: bool) -> Result<(String, XpubAddressType)> {
    // xpub / tpub — bitcoin crate handles natively
    if xpub_str.starts_with("xpub") || xpub_str.starts_with("tpub") {
        return Ok((xpub_str.to_string(), XpubAddressType::Legacy));
    }

    let prefix = xpub_str.get(..4).unwrap_or("");
    let (addr_type, target_version): (XpubAddressType, [u8; 4]) = match prefix {
        "ypub" | "upub" => {
            let v = if is_testnet {
                [0x04, 0x35, 0x87, 0xCF] // tpub
            } else {
                [0x04, 0x88, 0xB2, 0x1E] // xpub
            };
            (XpubAddressType::WrappedSegwit, v)
        }
        "zpub" | "vpub" => {
            let v = if is_testnet {
                [0x04, 0x35, 0x87, 0xCF] // tpub
            } else {
                [0x04, 0x88, 0xB2, 0x1E] // xpub
            };
            (XpubAddressType::NativeSegwit, v)
        }
        _ => {
            return Err(CryptofolioError::Other(format!(
                "Unsupported xpub prefix in '{}'. Expected xpub, ypub, zpub, tpub, upub, or vpub",
                &xpub_str[..xpub_str.len().min(8)]
            )));
        }
    };

    // Base58Check decode
    let mut decoded = bitcoin::base58::decode_check(xpub_str)
        .map_err(|e| CryptofolioError::Other(format!("Invalid xpub base58: {}", e)))?;

    if decoded.len() < 4 {
        return Err(CryptofolioError::Other(
            "xpub payload too short".to_string(),
        ));
    }

    // Swap version bytes
    decoded[0..4].copy_from_slice(&target_version);

    // Base58Check re-encode
    let reencoded = bitcoin::base58::encode_check(&decoded);

    Ok((reencoded, addr_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_from_xpub() {
        // BIP44 test vector xpub at m/44'/0'/0'
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        let addrs = derive_addresses(xpub, false, 3).unwrap();
        assert_eq!(addrs.len(), 3);
        // All should be valid mainnet addresses
        for (addr, _) in &addrs {
            assert!(!addr.is_empty());
            assert!(
                addr.starts_with('1') || addr.starts_with('3') || addr.starts_with("bc1"),
                "unexpected address format: {}",
                addr
            );
        }
    }

    #[test]
    fn test_address_type_detection() {
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        let (_, addr_type) = normalize_to_xpub(xpub, false).unwrap();
        assert_eq!(addr_type, XpubAddressType::Legacy);
    }

    #[test]
    fn test_invalid_prefix_returns_error() {
        let result = derive_addresses("badprefix123", false, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_taproot_from_xpub_with_force_type() {
        // BIP44 xpub — forced to Taproot (BIP86 path users pass --type taproot)
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        let addrs =
            derive_addresses_with_type(xpub, false, 3, Some(XpubAddressType::Taproot)).unwrap();
        assert_eq!(addrs.len(), 3);
        for (addr, _) in &addrs {
            assert!(
                addr.starts_with("bc1p"),
                "Taproot address should start with bc1p, got: {}",
                addr
            );
        }
    }

    #[test]
    fn test_derive_addresses_with_type_none_matches_inferred() {
        // force_type = None should produce same result as derive_addresses
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        let inferred = derive_addresses(xpub, false, 3).unwrap();
        let explicit = derive_addresses_with_type(xpub, false, 3, None).unwrap();
        assert_eq!(inferred, explicit);
    }
}
