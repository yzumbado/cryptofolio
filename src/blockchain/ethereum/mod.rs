mod address;
mod client;

pub use address::validate_address;
pub use client::{AddressInfo, ERC20Token, EthereumTransaction, EtherscanClient};
