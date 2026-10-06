//! `aave health` — read-only Aave V3 position health (collateral, debt, health factor).

use colored::Colorize;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::blockchain::ethereum::EtherscanClient;
use crate::cli::output::{format_quantity, format_usd, print_kv, print_section};
use crate::cli::{AaveCommands, GlobalOptions};
use crate::config::AppConfig;
use crate::core::aave::{self, AaveUserAccountData};
use crate::error::{CryptofolioError, Result};

#[derive(Serialize)]
struct AaveHealthOutput {
    address: String,
    collateral_usd: String,
    debt_usd: String,
    available_borrows_usd: String,
    liquidation_threshold_pct: String,
    ltv_pct: String,
    /// `None` means no debt (Aave's "infinite" health factor).
    #[serde(skip_serializing_if = "Option::is_none")]
    health_factor: Option<String>,
}

impl From<(&str, &AaveUserAccountData)> for AaveHealthOutput {
    fn from((address, d): (&str, &AaveUserAccountData)) -> Self {
        Self {
            address: address.to_string(),
            collateral_usd: d.total_collateral_usd.to_string(),
            debt_usd: d.total_debt_usd.to_string(),
            available_borrows_usd: d.available_borrows_usd.to_string(),
            liquidation_threshold_pct: d.liquidation_threshold_pct.to_string(),
            ltv_pct: d.ltv_pct.to_string(),
            health_factor: d.health_factor.map(|v| v.to_string()),
        }
    }
}

pub async fn handle_aave_command(
    command: AaveCommands,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    match command {
        AaveCommands::Health { wallet, address } => aave_health(wallet, address, pool, opts).await,
    }
}

async fn aave_health(
    wallet: Option<String>,
    address: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let addresses = match (wallet, address) {
        (Some(_), Some(_)) => {
            return Err(CryptofolioError::InvalidInput(
                "specify either --wallet or --address, not both".into(),
            ))
        }
        (Some(name), None) => vec![resolve_ethereum_address(pool, &name).await?],
        (None, Some(addr)) => vec![addr],
        (None, None) => all_ethereum_addresses(pool).await?,
    };

    let config = AppConfig::load()?;
    let use_testnet = opts.testnet || config.general.use_testnet;
    let etherscan_key = std::env::var("ETHERSCAN_API_KEY")
        .ok()
        .or_else(|| config.get_etherscan_api_key());

    if use_testnet {
        // Aave V3 Sepolia exists, but the tracked positions are mainnet; the pool
        // constant is mainnet, so warn rather than silently querying mainnet in
        // testnet mode.
        crate::cli::output::warning("Testnet mode — Aave Pool constant is mainnet");
    }

    let client = EtherscanClient::new(false, etherscan_key);

    let mut results: Vec<AaveHealthOutput> = Vec::new();
    for addr in &addresses {
        let calldata = aave::build_user_account_data_call(addr);
        let hex = client.eth_call(aave::AAVE_V3_POOL, &calldata).await?;
        let data = aave::decode_user_account_data(&hex)?;
        // Skip addresses with no Aave position (no collateral and no debt).
        if data.total_collateral_usd.is_zero() && data.total_debt_usd.is_zero() {
            continue;
        }
        results.push(AaveHealthOutput::from((addr.as_str(), &data)));
    }

    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        return Ok(());
    }

    if results.is_empty() {
        println!("No Aave position found on the tracked Ethereum wallets.");
        return Ok(());
    }

    for (i, r) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("{}", "Aave V3 Health".bold());
        print_kv("Address", &r.address);
        println!();
        print_section("Position");
        print_kv("Collateral", &format_usd_str(&r.collateral_usd));
        print_kv("Debt", &format_usd_str(&r.debt_usd));
        print_kv(
            "Available to borrow",
            &format_usd_str(&r.available_borrows_usd),
        );
        print_kv(
            "Loan-to-value (LTV)",
            &format!("{}%", format_quantity(parse_decimal(&r.ltv_pct)?)),
        );
        print_kv(
            "Liquidation threshold",
            &format!(
                "{}%",
                format_quantity(parse_decimal(&r.liquidation_threshold_pct)?)
            ),
        );
        println!();
        print_section("Health Factor");
        print_kv(
            "Health factor",
            &health_factor_label(r.health_factor.as_deref()),
        );
    }

    Ok(())
}

pub(crate) fn health_factor_label(hf: Option<&str>) -> String {
    let Some(hf) = hf else {
        return "∞ (no debt)".to_string();
    };
    let hf = hf.parse::<f64>().unwrap_or(0.0);
    let (label, color) = if hf >= 1.5 {
        ("Healthy", "green")
    } else if hf >= 1.0 {
        ("Caution", "yellow")
    } else {
        ("Liquidation risk", "red")
    };
    format!("{}  ({})", hf, label.color(color).bold())
}

fn parse_decimal(s: &str) -> Result<rust_decimal::Decimal> {
    use std::str::FromStr;
    rust_decimal::Decimal::from_str(s)
        .map_err(|e| CryptofolioError::Other(format!("invalid decimal '{}': {}", s, e)))
}

// format_usd takes a Decimal; re-parse the string for the human path (the JSON
// path serializes the string directly).
fn format_usd_str(s: &str) -> String {
    match parse_decimal(s) {
        Ok(d) => format_usd(d),
        Err(_) => s.to_string(),
    }
}

async fn resolve_ethereum_address(pool: &SqlitePool, name: &str) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT wa.address FROM wallet_addresses wa \
         JOIN accounts a ON a.id = wa.account_id \
         WHERE a.name = ? AND wa.blockchain = 'ethereum' \
         ORDER BY wa.last_synced_at DESC LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    row.map(|(a,)| a).ok_or_else(|| {
        CryptofolioError::InvalidInput(format!("no Ethereum wallet named '{}'", name))
    })
}

async fn all_ethereum_addresses(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT wa.address FROM wallet_addresses wa \
         WHERE wa.blockchain = 'ethereum' AND wa.network = 'mainnet'",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(a,)| a).collect())
}

/// Fetch the Aave health for the first tracked Ethereum wallet with an active
/// debt position. Returns `None` when no such wallet exists. Used by the
/// portfolio headline to surface the health factor without a dedicated call.
pub(crate) async fn fetch_aave_health(
    pool: &SqlitePool,
    config: &AppConfig,
) -> Result<Option<AaveUserAccountData>> {
    let addresses = all_ethereum_addresses(pool).await?;
    if addresses.is_empty() {
        return Ok(None);
    }
    let etherscan_key = std::env::var("ETHERSCAN_API_KEY")
        .ok()
        .or_else(|| config.get_etherscan_api_key());
    let client = EtherscanClient::new(false, etherscan_key);

    for addr in &addresses {
        let calldata = aave::build_user_account_data_call(addr);
        let hex = client.eth_call(aave::AAVE_V3_POOL, &calldata).await?;
        let data = aave::decode_user_account_data(&hex)?;
        if !data.total_debt_usd.is_zero() {
            return Ok(Some(data));
        }
    }
    Ok(None)
}
