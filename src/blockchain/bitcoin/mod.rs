mod address;
mod client;

pub use address::{
    validate_address,
    validate_xpub,
    is_testnet_address,
    is_testnet_xpub,
    AddressType,
};

pub use client::{
    BlockstreamClient,
    AddressInfo,
    BitcoinTransaction,
};
