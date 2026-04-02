# Solana Blockchain Client Specification

## Provider

**User-supplied RPC endpoint** — no default. The public `api.mainnet-beta.solana.com`
aggressively rate-limits unauthenticated requests and is not suitable for production.

Recommended: [Helius](https://helius.dev) free tier (100k credits/day).

Set `SOLANA_RPC_URL` env var:
```
SOLANA_RPC_URL=https://mainnet.helius-rpc.com/?api-key=<key>
```

If `SOLANA_RPC_URL` is not set, Solana is silently skipped during `wallet sync`.

## Address Format

Base58-encoded Ed25519 public key — 32 bytes → 44 Base58 characters.

Validation: Base58 decode + length check == 32 bytes.
Source: `src/blockchain/solana/address.rs`

## BlockchainClient Implementation

```
health_check()        → getSlot (JSON-RPC)
get_address_summary() → SOL balance + SPL tokens + stake accounts
get_transactions()    → getSignaturesForAddress + getTransaction per sig
```

### SOL Balance

`getBalance` → lamports → divide by 1,000,000,000 → SOL

### SPL Token Balances

`getTokenAccountsByOwner` with TokenkegQ… program ID.

Each token account provides:
- `mint` — token mint address
- `tokenAmount.uiAmount` — pre-scaled human-readable balance (uses on-chain decimals)
- `tokenAmount.decimals` — for reference

Token metadata (symbol, name) resolved from **Jupiter strict token list**
(`https://token.jup.ag/strict`). Fetched once per client instance, cached in
`RwLock<HashMap<mint, TokenInfo>>`. Unknown mints display first 8 chars of mint.

### Stake Accounts

`getProgramAccounts` on the Stake program with `memcmp` at offset 44 matching
the staker's pubkey. Returns `SolanaStakeAccount` entries in `ChainExtras::Solana`.

### Transaction History

`getSignaturesForAddress` (limit 1000) → list of signatures
`getTransaction` per signature → pre/post balance delta for direction.

`since_block` maps to `until` parameter (slot, not block height).

## Known Limitations

- Public RPC unusable without rate-limit bypass (API key required)
- Transaction direction uses SOL balance delta only — SPL token transfers
  show as direction=Internal
- Jupiter token list covers mainstream tokens; low-cap or new tokens show
  truncated mint address as symbol
- No support for Token-2022 program accounts (future)
