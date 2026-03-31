mod address;
mod client;

pub use address::validate_address;
pub use client::{
    EtherscanClient,
    AddressInfo,
    ERC20Token,
    EthereumTransaction,
};
