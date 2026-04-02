# Ethereum Blockchain Client Specification

## Provider

**Etherscan.io** — free tier requires no key for basic use; API key raises rate limits.

- Mainnet: `https://api.etherscan.io/api`
- Testnet (Sepolia): `https://api-sepolia.etherscan.io/api`
- Custom: `EtherscanClient::with_base_url(url)` for tests

Set `ETHERSCAN_API_KEY` env var to authenticate.

## Address Format

EIP-55 mixed-case checksummed hex: `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0`

Validation: must start with `0x`, followed by exactly 40 hex chars.
Source: `src/blockchain/ethereum/address.rs`

## BlockchainClient Implementation

```
health_check()        → eth_blockNumber action
get_address_summary() → balance + ERC-20 token list
get_transactions()    → normal txs + token txs since startblock
```

### Balance

- ETH balance: `?module=account&action=balance`
- ERC-20 tokens: `?module=account&action=tokentx` — deduplicated by symbol, summed

### ERC-20 Detection

`AddressSummary.balances` contains one `WalletBalance` per distinct token symbol
found in the address's token transfer history. Zero-value balances are excluded.

### Transaction Direction

Determined by comparing `from` address to the queried address (case-insensitive):
- `from == address` → Outgoing
- `to == address` → Incoming
- Both → Internal (self-transfer)

### Fee Calculation

`fee = gas_used × gas_price_gwei / 1_000_000_000` (in ETH)

## Known Limitations

- ERC-20 balance is computed from transfer history, not current state.
  For tokens with complex mechanics (rebase, fee-on-transfer), balance may diverge.
- No ERC-721 / ERC-1155 (NFT) support
- Free tier rate limit: 5 req/s without API key
