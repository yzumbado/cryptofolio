/// Test fixture data for BDD scenarios

pub struct TestFixtures;

impl TestFixtures {
    /// Bitcoin test addresses
    pub fn bitcoin_address() -> &'static str {
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    }

    pub fn bitcoin_xpub() -> &'static str {
        "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvphXR5ePCqYAN5qRbNnCLanT9qDKnNT4yKYr8j6L51HvvPahBJPJJZpNAQTwD"
    }

    /// Ethereum test addresses
    pub fn ethereum_address() -> &'static str {
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"
    }

    /// Solana test address
    pub fn solana_address() -> &'static str {
        "DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK"
    }

    /// Cardano test address
    pub fn cardano_address() -> &'static str {
        "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh9lxptqcskpqldlml"
    }

    /// ERC-20 token addresses
    pub fn usdt_contract() -> &'static str {
        "0xdac17f958d2ee523a2206206994597c13d831ec7"
    }

    pub fn reth_contract() -> &'static str {
        "0xae78736cd615f374d3085123a210448e74fc6393"
    }

    pub fn rpl_contract() -> &'static str {
        "0xd33526068d116ce69f19a9ee46f0bd304f21a51f"
    }
}
