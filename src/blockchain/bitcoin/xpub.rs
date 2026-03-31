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
}

/// Derive `count` external-chain receiving addresses from an xpub/ypub/zpub.
///
/// Returns a `Vec<(address_string, child_index)>`.
pub fn derive_addresses(
    xpub_str: &str,
    is_testnet: bool,
    count: u32,
) -> Result<Vec<(String, u32)>> {
    let network = if is_testnet {
        Network::Testnet
    } else {
        Network::Bitcoin
    };

    // Normalise ypub/zpub to the canonical xpub version bytes so the bitcoin
    // crate can parse the key, and remember which address type to produce.
    let (xpub_canonical, addr_type) = normalize_to_xpub(xpub_str, is_testnet)?;

    let secp = Secp256k1::verification_only();
    let xpub = Xpub::from_str(&xpub_canonical)
        .map_err(|e| CryptofolioError::Other(format!("Invalid xpub: {}", e)))?;

    // Derive external chain: m/0
    let external = xpub
        .ckd_pub(&secp, ChildNumber::from_normal_idx(0).unwrap())
        .map_err(|e| CryptofolioError::Other(format!("Key derivation error: {}", e)))?;

    let mut addrs = Vec::with_capacity(count as usize);
    for i in 0..count {
        let child = external
            .ckd_pub(&secp, ChildNumber::from_normal_idx(i).unwrap())
            .map_err(|e| {
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
}
