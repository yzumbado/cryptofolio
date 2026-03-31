pub mod bitcoin;
pub mod ethereum;
pub mod cardano;

// Re-export commonly used items
pub use bitcoin::validate_address as validate_bitcoin_address;
pub use ethereum::validate_address as validate_ethereum_address;
pub use cardano::validate_address as validate_cardano_address;
