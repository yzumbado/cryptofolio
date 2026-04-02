# Bitcoin Blockchain Client Specification

## Provider

**Blockstream.info** — public, no API key required.

- Mainnet: `https://blockstream.info/api`
- Testnet: `https://blockstream.info/testnet/api`
- Custom: `BlockstreamClient::with_base_url(url)` for tests

## Address Types Supported

| Type | Prefix | Example |
|------|--------|---------|
| P2PKH (Legacy) | `1` | `1A1zP1eP5QGe…` |
| P2SH (SegWit wrapped) | `3` | `3J98t1WpEZ73…` |
| Bech32 (Native SegWit) | `bc1q` | `bc1qxy2kgdy…` |
| Bech32m (Taproot) | `bc1p` | `bc1p5cyxnux…` |
| Testnet | `m`, `n`, `2`, `tb1` | various |

Validation is in `src/blockchain/bitcoin/address.rs`.

## HD Wallet (xpub/zpub)

- `xpub` → BIP44 P2PKH derivation
- `zpub` → BIP84 Native SegWit (normalized to xpub internally)
- Derives first 20 external-chain addresses (BIP44 gap limit) at `wallet add` time
- Derivation path: `m/0/{index}` for external chain
- Source: `src/blockchain/bitcoin/xpub.rs`

## BlockchainClient Implementation

```
health_check()        → GET /blocks/tip/height
get_address_summary() → GET /address/{addr}  (balance + tx count)
get_transactions()    → GET /address/{addr}/txs  (with since_block filter)
```

Transaction direction is computed from UTXO analysis:
- `value_in` = sum of outputs TO this address
- `value_out` = sum of inputs FROM this address (via prevout)
- `net_value > 0` → Incoming, `< 0` → Outgoing

## Known Limitations

- No native RBF (replace-by-fee) detection
- Unconfirmed transactions included in balance but block_height = None
- Large address histories (>1000 txs) may require pagination in future
