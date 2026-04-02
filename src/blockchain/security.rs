/// Security guards for the wallet add boundary.
///
/// Detects private key material before it can be written to disk.
/// Returns an error immediately — nothing is persisted.
///
/// Patterns detected:
/// - Bitcoin WIF private keys (uncompressed: 51 chars starting 5,
///   compressed: 52 chars starting K or L)
/// - Ethereum raw private keys (64 hex chars, with or without 0x)
/// - BIP39 seed phrases (12, 18 or 24 space-separated lowercase words)

use crate::error::{CryptofolioError, Result};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Variant of private key material detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateKeyKind {
    BitcoinWif,
    EthereumRaw,
    Bip39SeedPhrase,
}

impl PrivateKeyKind {
    fn label(&self) -> &'static str {
        match self {
            Self::BitcoinWif => "Bitcoin WIF private key",
            Self::EthereumRaw => "Ethereum raw private key",
            Self::Bip39SeedPhrase => "BIP39 seed phrase",
        }
    }
}

/// Detect whether `input` looks like private key material.
///
/// Returns `Some(kind)` on a match, `None` if the input appears safe.
pub fn detect_private_key(input: &str) -> Option<PrivateKeyKind> {
    let trimmed = input.trim();

    if is_bitcoin_wif(trimmed) {
        return Some(PrivateKeyKind::BitcoinWif);
    }
    if is_ethereum_raw_key(trimmed) {
        return Some(PrivateKeyKind::EthereumRaw);
    }
    if is_bip39_seed_phrase(trimmed) {
        return Some(PrivateKeyKind::Bip39SeedPhrase);
    }
    None
}

/// Reject `input` with an error if it contains private key material.
///
/// Call this at every point where user-supplied strings enter the system
/// (address, xpub, label, etc.) before writing anything to the DB.
pub fn reject_if_private_key(input: &str) -> Result<()> {
    if let Some(kind) = detect_private_key(input) {
        return Err(CryptofolioError::Other(format!(
            "Rejected: input looks like a {} — cryptofolio is watch-only and \
             never stores private keys. Check your clipboard.",
            kind.label()
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pattern matchers
// ---------------------------------------------------------------------------

/// Bitcoin WIF: Base58Check-encoded private key.
///
/// Uncompressed (51 chars): starts with '5'
/// Compressed   (52 chars): starts with 'K' or 'L'
///
/// We use a conservative length window (50–52 chars) to allow for minor
/// encoding quirks while avoiding false positives on xpubs (which are 111
/// chars long).
fn is_bitcoin_wif(s: &str) -> bool {
    if s.len() < 50 || s.len() > 52 {
        return false;
    }
    let first = s.chars().next().unwrap_or('\0');
    if !matches!(first, '5' | 'K' | 'L') {
        return false;
    }
    // All characters must be Base58 (no 0, O, I, l)
    s.chars()
        .all(|c| matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'))
}

/// Ethereum raw private key: 32 bytes = 64 hex characters.
///
/// Matches `[0-9a-fA-F]{64}` with optional `0x` prefix.
/// A valid xpub or address is never 64 hex chars long.
fn is_ethereum_raw_key(s: &str) -> bool {
    let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// BIP39 seed phrase: 12, 18, or 24 lowercase alphabetic words.
///
/// We check:
/// 1. Word count is exactly 12, 18, or 24.
/// 2. Every word contains only lowercase ASCII letters.
/// 3. Every word is between 3 and 8 characters (BIP39 wordlist range).
///
/// This is a heuristic — we do NOT load the full 2048-word BIP39 wordlist
/// to avoid a binary size dependency. The checks above have near-zero false-
/// positive rate for real address strings or xpubs.
fn is_bip39_seed_phrase(s: &str) -> bool {
    let words: Vec<&str> = s.split_whitespace().collect();
    if !matches!(words.len(), 12 | 18 | 24) {
        return false;
    }
    words.iter().all(|w| {
        let n = w.len();
        n >= 3 && n <= 8 && w.chars().all(|c| c.is_ascii_lowercase())
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Bitcoin WIF ---

    #[test]
    fn detects_wif_uncompressed() {
        // All-zeros private key, uncompressed WIF (starts with 5, 51 chars)
        let wif = "5HpHagT65TZzG1PH3CSu63k8DbpvD8s5ip4nEB3kEsreAnchuDf";
        assert_eq!(detect_private_key(wif), Some(PrivateKeyKind::BitcoinWif));
    }

    #[test]
    fn detects_wif_compressed_k() {
        // All-zeros private key, compressed WIF (starts with K, 52 chars)
        let wif = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFYuSQrcATrz";
        assert_eq!(detect_private_key(wif), Some(PrivateKeyKind::BitcoinWif));
    }

    #[test]
    fn does_not_flag_bitcoin_address() {
        let addr = "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf"; // 34 chars, starts with 1
        assert_eq!(detect_private_key(addr), None);
    }

    #[test]
    fn does_not_flag_xpub() {
        // xpub is 111 chars — well outside the WIF window
        let xpub = "xpub661MyMwAqRbcGJ8fS7x4hJaGAAZBMBZFVUwBq3fxMK7MnE6j3yz4YqdWSXJXRbNimizxDuq6KYNdMC2SZjWJg4VBhbQCT2Xj6o4bWvQF5g";
        assert_eq!(detect_private_key(xpub), None);
    }

    // --- Ethereum raw private key ---

    #[test]
    fn detects_eth_raw_key_no_prefix() {
        let key = "4c0883a69102937d6231471b5dbb6e538eba2ef62a0b8fe5b30f24a6f2b5f7a8";
        assert_eq!(detect_private_key(key), Some(PrivateKeyKind::EthereumRaw));
    }

    #[test]
    fn detects_eth_raw_key_with_0x() {
        let key = "0x4c0883a69102937d6231471b5dbb6e538eba2ef62a0b8fe5b30f24a6f2b5f7a8";
        assert_eq!(detect_private_key(key), Some(PrivateKeyKind::EthereumRaw));
    }

    #[test]
    fn does_not_flag_eth_address() {
        let addr = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"; // 42 chars with 0x
        assert_eq!(detect_private_key(addr), None);
    }

    // --- BIP39 seed phrase ---

    #[test]
    fn detects_12_word_seed_phrase() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        assert_eq!(
            detect_private_key(phrase),
            Some(PrivateKeyKind::Bip39SeedPhrase)
        );
    }

    #[test]
    fn detects_24_word_seed_phrase() {
        let phrase = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
        assert_eq!(
            detect_private_key(phrase),
            Some(PrivateKeyKind::Bip39SeedPhrase)
        );
    }

    #[test]
    fn does_not_flag_11_words() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        assert_eq!(detect_private_key(phrase), None);
    }

    #[test]
    fn does_not_flag_sentence_with_uppercase() {
        // Uppercase → not a seed phrase
        let phrase = "This is not a seed phrase at all because it has mixed Case words here";
        assert_eq!(detect_private_key(phrase), None);
    }

    // --- reject_if_private_key ---

    #[test]
    fn reject_returns_err_for_private_key() {
        let key = "4c0883a69102937d6231471b5dbb6e538eba2ef62a0b8fe5b30f24a6f2b5f7a8";
        assert!(reject_if_private_key(key).is_err());
    }

    #[test]
    fn reject_returns_ok_for_safe_input() {
        let addr = "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf";
        assert!(reject_if_private_key(addr).is_ok());
    }

    #[test]
    fn reject_error_message_is_descriptive() {
        let key = "0x4c0883a69102937d6231471b5dbb6e538eba2ef62a0b8fe5b30f24a6f2b5f7a8";
        let err = reject_if_private_key(key).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("watch-only"), "error should mention watch-only");
        assert!(msg.contains("private"), "error should mention private key");
    }
}
