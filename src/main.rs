mod cli;
mod config;
mod core;
mod db;
mod error;
mod exchange;
mod shell;

use clap::Parser;

use cli::commands::{
    handle_account_command, handle_category_command, handle_config_command,
    handle_holdings_command, handle_import_command, handle_market_command,
    handle_portfolio_command, handle_price_command, handle_sync_command,
    handle_tx_command,
};
use cli::output::init_color;
use cli::{Cli, Commands, GlobalOptions};
use error::Result;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        cli::output::error(&e.to_string());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    // Initialize global options
    let opts = GlobalOptions::from_cli(&cli);

    // Initialize color settings
    init_color(opts.no_color);

    // Initialize database
    let pool = db::init_pool().await?;

    match cli.command {
        Commands::Price { symbols } => {
            handle_price_command(symbols, &pool, &opts).await?;
        }

        Commands::Market { symbol, show_24h } => {
            handle_market_command(symbol, show_24h, &pool, &opts).await?;
        }

        Commands::Account { command } => {
            handle_account_command(command, &pool, &opts).await?;
        }

        Commands::Category { command } => {
            handle_category_command(command, &pool, &opts).await?;
        }

        Commands::Holdings { command } => {
            handle_holdings_command(command, &pool, &opts).await?;
        }

        Commands::Portfolio {
            by_account,
            by_category,
            account,
            category,
        } => {
            handle_portfolio_command(by_account, by_category, account, category, &pool, &opts).await?;
        }

        Commands::Tx { command } => {
            handle_tx_command(command, &pool, &opts).await?;
        }

        Commands::Sync { account } => {
            handle_sync_command(account, &pool, &opts).await?;
        }

        Commands::Import {
            file,
            account,
            format,
        } => {
            handle_import_command(file, account, format, &pool, &opts).await?;
        }

        Commands::Config { command } => {
            handle_config_command(command, &pool, &opts).await?;
        }

        Commands::Shell => {
            let mut shell = shell::Shell::new(pool, opts).await?;
            shell.run().await?;
        }
    }

    Ok(())
}
