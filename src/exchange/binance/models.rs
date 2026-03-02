#![allow(dead_code)]

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BinancePriceResponse {
    pub symbol: String,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct BinanceTicker24hResponse {
    pub symbol: String,
    #[serde(rename = "lastPrice", deserialize_with = "deserialize_decimal")]
    pub last_price: Decimal,
    #[serde(rename = "priceChange", deserialize_with = "deserialize_decimal")]
    pub price_change: Decimal,
    #[serde(rename = "priceChangePercent", deserialize_with = "deserialize_decimal")]
    pub price_change_percent: Decimal,
    #[serde(rename = "highPrice", deserialize_with = "deserialize_decimal")]
    pub high_price: Decimal,
    #[serde(rename = "lowPrice", deserialize_with = "deserialize_decimal")]
    pub low_price: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub volume: Decimal,
    #[serde(rename = "quoteVolume", deserialize_with = "deserialize_decimal")]
    pub quote_volume: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct BinanceAccountResponse {
    pub balances: Vec<BinanceBalance>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub free: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub locked: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct BinanceExchangeInfo {
    pub symbols: Vec<BinanceSymbolInfo>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceSymbolInfo {
    pub symbol: String,
    #[serde(rename = "baseAsset")]
    pub base_asset: String,
    #[serde(rename = "quoteAsset")]
    pub quote_asset: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceError {
    pub code: i32,
    pub msg: String,
}

// Trade history models
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceTrade {
    pub symbol: String,
    pub id: i64,
    #[serde(rename = "orderId")]
    pub order_id: i64,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub price: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub qty: Decimal,
    #[serde(rename = "quoteQty", deserialize_with = "deserialize_decimal")]
    pub quote_qty: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub commission: Decimal,
    #[serde(rename = "commissionAsset")]
    pub commission_asset: String,
    pub time: i64,
    #[serde(rename = "isBuyer")]
    pub is_buyer: bool,
    #[serde(rename = "isMaker")]
    pub is_maker: bool,
}

// Deposit history models
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceDeposit {
    pub id: String,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub amount: Decimal,
    pub coin: String,
    pub network: String,
    pub status: i32, // 0=pending, 6=credited, 1=success
    #[serde(rename = "address")]
    pub address: Option<String>,
    #[serde(rename = "txId")]
    pub tx_id: Option<String>,
    #[serde(rename = "insertTime")]
    pub insert_time: i64,
    #[serde(rename = "confirmTimes")]
    pub confirm_times: Option<String>,
}

// Withdrawal history models
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceWithdrawal {
    pub id: String,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub amount: Decimal,
    #[serde(rename = "transactionFee", deserialize_with = "deserialize_decimal")]
    pub transaction_fee: Decimal,
    pub coin: String,
    pub status: i32, // 0-6, 6=success
    pub address: String,
    #[serde(rename = "txId")]
    pub tx_id: Option<String>,
    #[serde(rename = "applyTime")]
    pub apply_time: i64,
    pub network: String,
    #[serde(rename = "withdrawOrderId")]
    pub withdraw_order_id: Option<String>,
}

// Fiat deposit/withdrawal models
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceFiatOrder {
    #[serde(rename = "orderNo")]
    pub order_no: String,
    #[serde(rename = "fiatCurrency")]
    pub fiat_currency: String,
    #[serde(rename = "indicatedAmount", deserialize_with = "deserialize_decimal")]
    pub indicated_amount: Decimal,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub amount: Decimal,
    #[serde(rename = "totalFee", deserialize_with = "deserialize_decimal")]
    pub total_fee: Decimal,
    pub method: String, // "Card", "Bank Transfer", etc.
    pub status: String, // "Processing", "Completed", "Failed"
    #[serde(rename = "cryptoCurrency")]
    pub crypto_currency: String,
    #[serde(rename = "createTime")]
    pub create_time: i64,
    #[serde(rename = "updateTime")]
    pub update_time: i64,
}

#[derive(Debug, Deserialize)]
pub struct BinanceFiatOrderResponse {
    pub code: String,
    pub message: String,
    pub data: Vec<BinanceFiatOrder>,
    pub total: i64,
    pub success: bool,
}

// Internal transfer models (Spot <-> Earn, etc.)
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceTransfer {
    #[serde(rename = "tranId")]
    pub tran_id: i64,
    #[serde(deserialize_with = "deserialize_decimal")]
    pub amount: Decimal,
    pub asset: String,
    #[serde(rename = "type")]
    pub transfer_type: String, // "MAIN_UMFUTURE", "MAIN_C2C", etc.
    pub status: String, // "CONFIRMED", "PENDING", "FAILED"
    pub timestamp: i64,
}

#[derive(Debug, Deserialize)]
pub struct BinanceTransferResponse {
    pub total: i64,
    pub rows: Vec<BinanceTransfer>,
}

// Coin info model
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceCoinInfo {
    pub coin: String,
    #[serde(rename = "depositAllEnable")]
    pub deposit_all_enable: bool,
    #[serde(rename = "withdrawAllEnable")]
    pub withdraw_all_enable: bool,
    pub name: String,
    #[serde(rename = "networkList")]
    pub network_list: Vec<BinanceNetwork>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BinanceNetwork {
    pub network: String,
    pub name: String,
    #[serde(rename = "depositEnable")]
    pub deposit_enable: bool,
    #[serde(rename = "withdrawEnable")]
    pub withdraw_enable: bool,
}

// Custom deserializer for Decimal from string
fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
