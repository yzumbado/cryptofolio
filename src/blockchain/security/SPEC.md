# Security Guard Specification

Location: `src/blockchain/security.rs`

## Purpose

Cryptofolio is a **watch-only** portfolio tracker. It never signs transactions
and must never store private key material. The security guard is the single
enforcement point — called at every boundary where user-supplied strings enter
the system before any DB write occurs.

## Patterns Detected

| Pattern | Example | Detection rule |
|---------|---------|----------------|
| Bitcoin WIF (uncompressed) | `5HpHagT65T…AnchuDf` | Length 50–52, starts with `5`, all Base58 chars |
| Bitcoin WIF (compressed) | `KwDiBf89Qg…ATrz` | Length 50–52, starts with `K` or `L`, all Base58 chars |
| Ethereum raw private key | `4c0883a6…` or `0x4c0883a6…` | Exactly 64 hex chars (with or without `0x`) |
| BIP39 seed phrase | `abandon abandon … about` | 12, 18, or 24 lowercase alphabetic words of 3–8 chars each |

## What Is Not Detected

- Extended public keys (xpub/zpub/ypub) — these are safe to store (watch-only)
- Testnet WIF keys starting with `c` — out of scope (testnet use not supported)
- Passphrase-protected seeds — the passphrase itself would look like a normal
  string and cannot be detected without wordlist membership checks

## API

```rust
// Returns Some(kind) if private key material detected, None if safe
pub fn detect_private_key(input: &str) -> Option<PrivateKeyKind>

// Returns Err if private key material detected, Ok(()) if safe
// Call this at every user-input boundary before any DB write
pub fn reject_if_private_key(input: &str) -> Result<()>
```

## Error Message Contract

The error message always contains the word `watch-only` so BDD assertions can
use a single stable string regardless of which pattern was detected.

## Call Sites

- `src/cli/commands/wallet.rs::handle_wallet_add` — address field, xpub field, label field
- Future: any CLI command that accepts user-supplied wallet data
