#![allow(dead_code)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptofolioError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Category not found: {0}")]
    CategoryNotFound(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    #[error("Insufficient balance: have {available}, need {required}")]
    InsufficientBalance {
        available: String,
        required: String,
    },

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Exchange API error: {0}")]
    ExchangeApi(String),

    #[error("Authentication required: {0}")]
    AuthRequired(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Shell error: {0}")]
    Shell(String),

    #[error("AI error: {0}")]
    Ai(String),

    #[error("Operation cancelled by user")]
    OperationCancelled,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CryptofolioError>;
