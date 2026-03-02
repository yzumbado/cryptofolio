#![allow(dead_code)]

pub const MAINNET_BASE_URL: &str = "https://api.binance.com";
pub const TESTNET_BASE_URL: &str = "https://testnet.binance.vision";

// Public endpoints
pub const TICKER_PRICE: &str = "/api/v3/ticker/price";
pub const TICKER_24H: &str = "/api/v3/ticker/24hr";
pub const EXCHANGE_INFO: &str = "/api/v3/exchangeInfo";

// Private endpoints (require authentication)
pub const ACCOUNT: &str = "/api/v3/account";
pub const MY_TRADES: &str = "/api/v3/myTrades";

// History endpoints (SAPI - requires authentication)
pub const DEPOSIT_HISTORY: &str = "/sapi/v1/capital/deposit/hisrec";
pub const WITHDRAWAL_HISTORY: &str = "/sapi/v1/capital/withdraw/history";
pub const FIAT_DEPOSIT_HISTORY: &str = "/sapi/v1/fiat/orders";
pub const UNIVERSAL_TRANSFER_HISTORY: &str = "/sapi/v1/asset/transfer";
pub const ALL_COINS_INFO: &str = "/sapi/v1/capital/config/getall";
