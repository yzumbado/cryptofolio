/// Integration tests for blockchain API clients via the BlockchainClient trait.
///
/// These tests hit **real public APIs** with known, stable mainnet addresses.
/// No mocks. No wiremock. Real network calls.
///
/// # Running locally
///
///   CRYPTOFOLIO_INTEGRATION_TESTS=1 cargo test --test blockchain_clients
///
/// # Cardano (requires Blockfrost API key)
///
///   CRYPTOFOLIO_INTEGRATION_TESTS=1 BLOCKFROST_API_KEY=mainnetXXX cargo test --test blockchain_clients
///
/// # CI
///
/// Run nightly via the `integration` job in ci.yml.
/// BLOCKFROST_API_KEY and ETHERSCAN_API_KEY are injected from GitHub Secrets.

use cryptofolio::blockchain::BlockchainClient;

// ---------------------------------------------------------------------------
// Known stable mainnet addresses
// ---------------------------------------------------------------------------
//
// Bitcoin  — Satoshi's genesis coinbase. Never spent. People still send to it.
const BTC_ADDRESS: &str = "1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf";

// Ethereum — Vitalik Buterin's publicly known address (linked to his ENS).
const ETH_ADDRESS: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";

// Cardano  — A well-known mainnet address used across Cardano documentation.
const ADA_ADDRESS: &str =
    "addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x";

// Solana   — Marinade Finance staking program: a well-known, long-lived address
//            with SOL balance and a stable transaction history.
const SOL_ADDRESS: &str = "8szGkuLTAux9XMgZ2vtY39jVSowEovBGME4mDXQkNGNR";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn integration_enabled() -> bool {
    std::env::var("CRYPTOFOLIO_INTEGRATION_TESTS").is_ok()
}

fn blockfrost_api_key() -> Option<String> {
    std::env::var("BLOCKFROST_API_KEY").ok()
}

fn etherscan_api_key() -> Option<String> {
    std::env::var("ETHERSCAN_API_KEY").ok()
}

fn solana_rpc_url() -> Option<String> {
    std::env::var("SOLANA_RPC_URL").ok()
}

// ---------------------------------------------------------------------------
// Bitcoin — BlockstreamClient via BlockchainClient trait
// ---------------------------------------------------------------------------

#[cfg(test)]
mod bitcoin {
    use super::*;
    use cryptofolio::blockchain::bitcoin::BlockstreamClient;
    use cryptofolio::blockchain::types::Chain;

    #[tokio::test]
    async fn test_health_check_returns_reachable() {
        if !integration_enabled() {
            return;
        }
        let client = BlockstreamClient::new(false);
        let status = client.health_check().await.expect("health_check should succeed");

        assert!(status.reachable, "Blockstream should be reachable");
        assert!(status.block_height.unwrap_or(0) > 0, "block_height should be > 0");
        assert_eq!(status.provider, "Blockstream");
    }

    #[tokio::test]
    async fn test_get_address_summary_returns_btc_balance() {
        if !integration_enabled() {
            return;
        }
        let client = BlockstreamClient::new(false);
        let summary = client
            .get_address_summary(BTC_ADDRESS)
            .await
            .expect("get_address_summary should succeed");

        assert_eq!(summary.address, BTC_ADDRESS);
        assert_eq!(summary.chain, Chain::Bitcoin);
        assert!(!summary.balances.is_empty(), "should have at least one balance entry");

        let btc = &summary.balances[0];
        assert_eq!(btc.asset, "BTC");
        assert!(btc.quantity > rust_decimal::Decimal::ZERO, "genesis address should have BTC");
        assert!(summary.transaction_count > 0, "genesis address should have transactions");
    }

    #[tokio::test]
    async fn test_get_transactions_returns_nonempty_list() {
        if !integration_enabled() {
            return;
        }
        let client = BlockstreamClient::new(false);
        let txs = client
            .get_transactions(BTC_ADDRESS, None)
            .await
            .expect("get_transactions should succeed");

        assert!(!txs.is_empty(), "genesis address should have transactions");
        for tx in &txs {
            assert!(!tx.external_id.is_empty(), "external_id should not be empty");
            assert!(tx.amount >= rust_decimal::Decimal::ZERO, "amount should be non-negative");
        }
    }

    #[tokio::test]
    async fn test_get_transactions_since_block_filters_results() {
        if !integration_enabled() {
            return;
        }
        let client = BlockstreamClient::new(false);

        let all_txs = client
            .get_transactions(BTC_ADDRESS, None)
            .await
            .expect("get_transactions should succeed");

        // A very recent block height should return fewer (or equal) results
        let recent_txs = client
            .get_transactions(BTC_ADDRESS, Some(800_000))
            .await
            .expect("get_transactions with since_block should succeed");

        assert!(
            recent_txs.len() <= all_txs.len(),
            "since_block filter should not return more results than no filter"
        );
    }
}

// ---------------------------------------------------------------------------
// Ethereum — EtherscanClient via BlockchainClient trait
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ethereum {
    use super::*;
    use cryptofolio::blockchain::ethereum::EtherscanClient;
    use cryptofolio::blockchain::types::Chain;

    #[tokio::test]
    async fn test_health_check_returns_reachable() {
        if !integration_enabled() {
            return;
        }
        let client = EtherscanClient::new(false, etherscan_api_key());
        let status = client.health_check().await.expect("health_check should succeed");

        assert!(status.reachable, "Etherscan should be reachable");
        assert!(status.block_height.unwrap_or(0) > 0, "block_height should be > 0");
        assert_eq!(status.provider, "Etherscan");
    }

    #[tokio::test]
    async fn test_get_address_summary_returns_eth_balance() {
        if !integration_enabled() {
            return;
        }
        let client = EtherscanClient::new(false, etherscan_api_key());
        let summary = client
            .get_address_summary(ETH_ADDRESS)
            .await
            .expect("get_address_summary should succeed");

        assert_eq!(summary.address, ETH_ADDRESS);
        assert_eq!(summary.chain, Chain::Ethereum);
        assert!(!summary.balances.is_empty());

        let eth = &summary.balances[0];
        assert_eq!(eth.asset, "ETH");
        assert!(eth.quantity > rust_decimal::Decimal::ZERO, "Vitalik's address should have ETH");
    }

    #[tokio::test]
    async fn test_get_address_summary_includes_erc20_tokens() {
        if !integration_enabled() {
            return;
        }
        let client = EtherscanClient::new(false, etherscan_api_key());
        let summary = client
            .get_address_summary(ETH_ADDRESS)
            .await
            .expect("get_address_summary should succeed");

        // Vitalik's address holds many ERC-20 tokens
        assert!(summary.balances.len() > 1, "should have ETH + ERC-20 tokens");
        for balance in &summary.balances[1..] {
            assert!(!balance.asset.is_empty(), "token symbol should not be empty");
        }
    }

    #[tokio::test]
    async fn test_get_transactions_returns_nonempty_list() {
        if !integration_enabled() {
            return;
        }
        let client = EtherscanClient::new(false, etherscan_api_key());
        let txs = client
            .get_transactions(ETH_ADDRESS, None)
            .await
            .expect("get_transactions should succeed");

        assert!(!txs.is_empty(), "Vitalik's address should have transactions");
        for tx in &txs {
            assert!(!tx.external_id.is_empty());
            assert!(tx.block_height.unwrap_or(0) > 0);
        }
    }

    #[tokio::test]
    async fn test_get_transactions_since_block_filters_results() {
        if !integration_enabled() {
            return;
        }
        let client = EtherscanClient::new(false, etherscan_api_key());

        let all_txs = client
            .get_transactions(ETH_ADDRESS, None)
            .await
            .expect("get_transactions should succeed");

        let recent_txs = client
            .get_transactions(ETH_ADDRESS, Some(18_000_000))
            .await
            .expect("get_transactions with since_block should succeed");

        assert!(
            recent_txs.len() <= all_txs.len(),
            "since_block filter should not return more results than no filter"
        );
    }
}

// ---------------------------------------------------------------------------
// Cardano — BlockfrostClient via BlockchainClient trait
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cardano {
    use super::*;
    use cryptofolio::blockchain::cardano::BlockfrostClient;
    use cryptofolio::blockchain::types::{Chain, ChainExtras};

    #[tokio::test]
    async fn test_health_check_returns_reachable() {
        if !integration_enabled() {
            return;
        }
        let api_key = match blockfrost_api_key() {
            Some(k) => k,
            None => {
                eprintln!("BLOCKFROST_API_KEY not set — skipping");
                return;
            }
        };
        let client = BlockfrostClient::new(false, Some(api_key));
        let status = client.health_check().await.expect("health_check should succeed");

        assert!(status.reachable, "Blockfrost should be reachable");
        assert_eq!(status.provider, "Blockfrost");
    }

    #[tokio::test]
    async fn test_get_address_summary_returns_ada_balance() {
        if !integration_enabled() {
            return;
        }
        let api_key = match blockfrost_api_key() {
            Some(k) => k,
            None => { eprintln!("BLOCKFROST_API_KEY not set — skipping"); return; }
        };
        let client = BlockfrostClient::new(false, Some(api_key));
        let summary = client
            .get_address_summary(ADA_ADDRESS)
            .await
            .expect("get_address_summary should succeed");

        assert_eq!(summary.address, ADA_ADDRESS);
        assert_eq!(summary.chain, Chain::Cardano);
        assert!(!summary.balances.is_empty());

        let ada = &summary.balances[0];
        assert_eq!(ada.asset, "ADA");
        assert!(ada.quantity >= rust_decimal::Decimal::ZERO);
        assert!(summary.transaction_count > 0);
    }

    #[tokio::test]
    async fn test_get_address_summary_populates_cardano_extras() {
        if !integration_enabled() {
            return;
        }
        let api_key = match blockfrost_api_key() {
            Some(k) => k,
            None => { eprintln!("BLOCKFROST_API_KEY not set — skipping"); return; }
        };
        let client = BlockfrostClient::new(false, Some(api_key));
        let summary = client
            .get_address_summary(ADA_ADDRESS)
            .await
            .expect("get_address_summary should succeed");

        // extras should be ChainExtras::Cardano
        match summary.extras {
            Some(ChainExtras::Cardano { .. }) => {}
            other => panic!("expected ChainExtras::Cardano, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_transactions_returns_nonempty_list() {
        if !integration_enabled() {
            return;
        }
        let api_key = match blockfrost_api_key() {
            Some(k) => k,
            None => { eprintln!("BLOCKFROST_API_KEY not set — skipping"); return; }
        };
        let client = BlockfrostClient::new(false, Some(api_key));
        let txs = client
            .get_transactions(ADA_ADDRESS, None)
            .await
            .expect("get_transactions should succeed");

        assert!(!txs.is_empty(), "address should have transactions");
        for tx in &txs {
            assert!(!tx.external_id.is_empty());
            assert!(tx.block_height.unwrap_or(0) > 0);
            assert!(tx.fee.unwrap_or(rust_decimal::Decimal::ZERO) >= rust_decimal::Decimal::ZERO);
        }
    }
}

// ---------------------------------------------------------------------------
// Solana — SolanaRpcClient via BlockchainClient trait
// ---------------------------------------------------------------------------
//
// Requires SOLANA_RPC_URL (e.g. Helius free tier).
// Skipped gracefully when not set.

#[cfg(test)]
mod solana {
    use super::*;
    use cryptofolio::blockchain::solana::SolanaRpcClient;
    use cryptofolio::blockchain::types::Chain;

    #[tokio::test]
    async fn test_health_check_returns_reachable() {
        if !integration_enabled() {
            return;
        }
        let rpc_url = match solana_rpc_url() {
            Some(u) => u,
            None => { eprintln!("SOLANA_RPC_URL not set — skipping"); return; }
        };
        let client = SolanaRpcClient::new(rpc_url);
        let status = client.health_check().await.expect("health_check should succeed");

        assert!(status.reachable, "Solana RPC should be reachable");
        assert!(status.block_height.unwrap_or(0) > 0, "slot should be > 0");
        assert_eq!(status.provider, "Solana RPC");
    }

    #[tokio::test]
    async fn test_get_address_summary_returns_sol_balance() {
        if !integration_enabled() {
            return;
        }
        let rpc_url = match solana_rpc_url() {
            Some(u) => u,
            None => { eprintln!("SOLANA_RPC_URL not set — skipping"); return; }
        };
        let client = SolanaRpcClient::new(rpc_url);
        let summary = client
            .get_address_summary(SOL_ADDRESS)
            .await
            .expect("get_address_summary should succeed");

        assert_eq!(summary.address, SOL_ADDRESS);
        assert_eq!(summary.chain, Chain::Solana);
        assert!(!summary.balances.is_empty());

        let sol = &summary.balances[0];
        assert_eq!(sol.asset, "SOL");
        assert!(sol.quantity >= rust_decimal::Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_get_transactions_returns_list() {
        if !integration_enabled() {
            return;
        }
        let rpc_url = match solana_rpc_url() {
            Some(u) => u,
            None => { eprintln!("SOLANA_RPC_URL not set — skipping"); return; }
        };
        let client = SolanaRpcClient::new(rpc_url);
        let txs = client
            .get_transactions(SOL_ADDRESS, None)
            .await
            .expect("get_transactions should succeed");

        assert!(!txs.is_empty(), "address should have transactions");
        for tx in &txs {
            assert!(!tx.external_id.is_empty(), "signature should not be empty");
            assert!(tx.fee.unwrap_or(rust_decimal::Decimal::ZERO) >= rust_decimal::Decimal::ZERO);
        }
    }
}
