use colored::Colorize;
use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::cli::output;
use crate::cli::CurrencyCommands;
use crate::core::currency::{AssetType, Currency, ExchangeRate};
use crate::db::currencies;
use crate::error::{CryptofolioError, Result};

pub async fn handle_currency_command(pool: &SqlitePool, cmd: CurrencyCommands) -> Result<()> {
    match cmd {
        CurrencyCommands::List {
            enabled,
            type_filter,
        } => list_currencies(pool, enabled, type_filter, false).await,

        CurrencyCommands::Show { code } => show_currency(pool, &code, false).await,

        CurrencyCommands::Add {
            code,
            name,
            symbol,
            decimals,
            type_name,
        } => add_currency(pool, &code, &name, &symbol, decimals, &type_name).await,

        CurrencyCommands::Remove { code, yes } => remove_currency(pool, &code, yes).await,

        CurrencyCommands::Toggle {
            code,
            enable,
            disable,
        } => toggle_currency(pool, &code, enable, disable).await,

        CurrencyCommands::SetRate {
            from,
            to,
            rate,
            notes,
        } => {
            let rate_decimal = rate.parse::<Decimal>().map_err(|_| {
                CryptofolioError::InvalidInput(format!("Invalid rate: {}", rate))
            })?;
            set_exchange_rate(pool, &from, &to, rate_decimal, notes).await
        }

        CurrencyCommands::ShowRate { from, to, history } => {
            show_exchange_rate(pool, &from, &to, history, false).await
        }
    }
}

async fn list_currencies(
    pool: &SqlitePool,
    enabled_only: bool,
    type_filter: Option<String>,
    json: bool,
) -> Result<()> {
    let mut currencies = currencies::list_currencies(pool).await?;

    // Filter by enabled status
    if enabled_only {
        currencies.retain(|c| c.enabled);
    }

    // Filter by type
    if let Some(ref filter) = type_filter {
        let asset_type = AssetType::from_str(filter).ok_or_else(|| {
            CryptofolioError::InvalidInput(format!("Invalid asset type: {}", filter))
        })?;
        currencies.retain(|c| c.asset_type == asset_type);
    }

    if json {
        output::print_json(&currencies)?;
        return Ok(());
    }

    // Print table
    println!("\n{}", "Supported Currencies".bold());
    println!("{}", "═".repeat(80));
    println!(
        "{:<6} {:<25} {:<12} {:<8} {:<8}",
        "Code", "Name", "Type", "Symbol", "Decimals"
    );
    println!("{}", "─".repeat(80));

    for currency in currencies {
        let status = if currency.enabled { "" } else { " (disabled)" };
        println!(
            "{:<6} {:<25} {:<12} {:<8} {:<8}",
            currency.code.bright_cyan(),
            format!("{}{}", currency.name, status),
            currency.asset_type.display_name(),
            currency.symbol,
            currency.decimals
        );
    }

    println!();
    Ok(())
}

async fn show_currency(pool: &SqlitePool, code: &str, json: bool) -> Result<()> {
    let code = code.to_uppercase();
    let currency = currencies::get_currency(pool, &code)
        .await?
        .ok_or_else(|| CryptofolioError::NotFound(format!("Currency not found: {}", code)))?;

    if json {
        output::print_json(&currency)?;
        return Ok(());
    }

    println!("\n{} {}", "Currency:".bold(), currency.code.bright_cyan());
    println!("{}", "═".repeat(50));
    println!("  Name:         {}", currency.name);
    println!("  Symbol:       {}", currency.symbol);
    println!("  Decimals:     {}", currency.decimals);
    println!("  Type:         {}", currency.asset_type.display_name());
    println!(
        "  Status:       {}",
        if currency.enabled {
            "Enabled".green()
        } else {
            "Disabled".red()
        }
    );
    println!();

    Ok(())
}

async fn add_currency(
    pool: &SqlitePool,
    code: &str,
    name: &str,
    symbol: &str,
    decimals: u8,
    type_name: &str,
) -> Result<()> {
    let code = code.to_uppercase();

    // Check if already exists
    if currencies::currency_exists(pool, &code).await? {
        return Err(CryptofolioError::AlreadyExists(format!(
            "Currency already exists: {}",
            code
        )));
    }

    let asset_type = AssetType::from_str(type_name).ok_or_else(|| {
        CryptofolioError::InvalidInput(format!("Invalid asset type: {}", type_name))
    })?;

    let currency = Currency::new(code.clone(), name, symbol, decimals, asset_type);

    currencies::add_currency(pool, &currency).await?;

    println!(
        "{} Added currency {} ({})",
        "✓".green(),
        code.bright_cyan(),
        name
    );

    Ok(())
}

async fn remove_currency(pool: &SqlitePool, code: &str, yes: bool) -> Result<()> {
    let code = code.to_uppercase();

    // Check if exists
    let currency = currencies::get_currency(pool, &code)
        .await?
        .ok_or_else(|| CryptofolioError::NotFound(format!("Currency not found: {}", code)))?;

    if !yes {
        println!(
            "{} This will delete currency '{}' ({}). Continue? [y/N]",
            "⚠".yellow(),
            code.bright_cyan(),
            currency.name
        );

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    currencies::remove_currency(pool, &code).await?;

    println!("{} Removed currency {}", "✓".green(), code.bright_cyan());

    Ok(())
}

async fn toggle_currency(
    pool: &SqlitePool,
    code: &str,
    enable: bool,
    disable: bool,
) -> Result<()> {
    let code = code.to_uppercase();

    let mut currency = currencies::get_currency(pool, &code)
        .await?
        .ok_or_else(|| CryptofolioError::NotFound(format!("Currency not found: {}", code)))?;

    if enable {
        currency.enabled = true;
        currencies::update_currency(pool, &currency).await?;
        println!("{} Enabled currency {}", "✓".green(), code.bright_cyan());
    } else if disable {
        currency.enabled = false;
        currencies::update_currency(pool, &currency).await?;
        println!("{} Disabled currency {}", "✓".green(), code.bright_cyan());
    } else {
        return Err(CryptofolioError::InvalidInput(
            "Must specify --enable or --disable".to_string(),
        ));
    }

    Ok(())
}

async fn set_exchange_rate(
    pool: &SqlitePool,
    from: &str,
    to: &str,
    rate: Decimal,
    notes: Option<String>,
) -> Result<()> {
    let from = from.to_uppercase();
    let to = to.to_uppercase();

    // Validate currencies exist
    if !currencies::currency_exists(pool, &from).await? {
        return Err(CryptofolioError::NotFound(format!(
            "Currency not found: {}",
            from
        )));
    }
    if !currencies::currency_exists(pool, &to).await? {
        return Err(CryptofolioError::NotFound(format!(
            "Currency not found: {}",
            to
        )));
    }

    let mut exchange_rate = ExchangeRate::new_manual(&from, &to, rate, chrono::Utc::now());
    exchange_rate.notes = notes;

    currencies::add_exchange_rate(pool, &exchange_rate).await?;

    println!(
        "{} Set exchange rate: {} {} = 1 {}",
        "✓".green(),
        rate,
        from.bright_cyan(),
        to.bright_cyan()
    );

    Ok(())
}

async fn show_exchange_rate(
    pool: &SqlitePool,
    from: &str,
    to: &str,
    history: bool,
    json: bool,
) -> Result<()> {
    let from = from.to_uppercase();
    let to = to.to_uppercase();

    if history {
        let rates = currencies::list_exchange_rates(pool, &from, &to).await?;

        if rates.is_empty() {
            println!("No exchange rates found for {}/{}", from, to);
            return Ok(());
        }

        if json {
            output::print_json(&rates)?;
            return Ok(());
        }

        println!(
            "\n{} Exchange Rate History: {}/{}",
            "📊".bold(),
            from.bright_cyan(),
            to.bright_cyan()
        );
        println!("{}", "═".repeat(80));
        println!("{:<20} {:<15} {:<10} {:<30}", "Date", "Rate", "Source", "Notes");
        println!("{}", "─".repeat(80));

        for rate in rates {
            println!(
                "{:<20} {:<15} {:<10} {:<30}",
                rate.timestamp.format("%Y-%m-%d %H:%M"),
                rate.rate,
                rate.source,
                rate.notes.as_deref().unwrap_or("")
            );
        }
        println!();
    } else {
        let rate = currencies::get_latest_exchange_rate(pool, &from, &to)
            .await?
            .ok_or_else(|| {
                CryptofolioError::NotFound(format!("No exchange rate found for {}/{}", from, to))
            })?;

        if json {
            output::print_json(&rate)?;
            return Ok(());
        }

        println!(
            "\n{} Exchange Rate: {}/{}",
            "💱".bold(),
            from.bright_cyan(),
            to.bright_cyan()
        );
        println!("{}", "═".repeat(50));
        println!("  Rate:     {} {} = 1 {}", rate.rate, from, to);
        println!("  Date:     {}", rate.timestamp.format("%Y-%m-%d %H:%M"));
        println!("  Source:   {}", rate.source);
        if let Some(notes) = rate.notes {
            println!("  Notes:    {}", notes);
        }
        println!();
    }

    Ok(())
}
