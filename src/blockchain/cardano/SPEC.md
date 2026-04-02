# Cardano Blockchain Client Specification

## Provider

**Blockfrost.io** — free tier: 50,000 requests/day. API key required for production use.

- Mainnet: `https://cardano-mainnet.blockfrost.io/api/v0`
- Custom: `BlockfrostClient::with_base_url(url)` for tests

Set `BLOCKFROST_API_KEY` env var. Without a key, requests will be rate-limited
to the point of being unusable for sync.

## Address Format

CIP-0019 bech32 encoding.

| Prefix | Type |
|--------|------|
| `addr1` | Enterprise address (mainnet) |
| `addr_test1` | Enterprise address (testnet) |

Validation: bech32 decode + first byte network/type check.
Source: `src/blockchain/cardano/address.rs`

**Test address:** `addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x`
(Only valid address in the BDD test suite — use this in all mock setups)

## BlockchainClient Implementation

```
health_check()        → GET /health + GET /blocks/latest (sequential)
get_address_summary() → GET /addresses/{addr} + GET /addresses/{addr}/utxos
get_transactions()    → GET /addresses/{addr}/transactions (paginated)
get_chain_extras()    → stake delegation info
```

### ADA Balance

Computed from UTXO set: sum of `lovelace` quantities across all UTXOs.
`1 ADA = 1_000_000 lovelace`

### Native Tokens

Also extracted from UTXOs. Each distinct policy_id+asset_name combination
becomes a `WalletBalance` entry. Display name from Blockfrost's `onchain_metadata`.

### Stake Delegation (ChainExtras::Cardano)

`get_chain_extras()` queries the stake address associated with the payment address
and returns:
- `pool_id`, `pool_ticker`, `pool_name` — delegation target
- `active_stake` — lovelace staked
- `margin_cost` — pool fee

### Transaction Direction

Currently returns `Internal` for all transactions. UTXO-level direction resolution
(tracking inputs vs outputs per address) requires extra per-transaction API calls
and is deferred to a future release.

## Known Limitations

- UTXO model means "balance" is always computed, never stored on-chain
- Direction is `Internal` until UTXO resolution is implemented
- Free tier exhausted quickly for addresses with high UTXO churn
