mod completer;
mod context;
mod shortcuts;

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Config, Editor};
use sqlx::SqlitePool;

use crate::cli::output::{colors_enabled, format_usd, init_color};
use crate::cli::GlobalOptions;
use crate::config::AppConfig;
use crate::db::{AccountRepository, HoldingRepository};
use crate::error::Result;
use crate::exchange::{BinanceClient, Exchange};

use completer::CryptofolioCompleter;
use context::ShellContext;
use shortcuts::expand_shortcuts;

/// Interactive shell for cryptofolio
pub struct Shell {
    pool: SqlitePool,
    opts: GlobalOptions,
    editor: Editor<CryptofolioCompleter, DefaultHistory>,
    context: ShellContext,
}

impl Shell {
    /// Create a new interactive shell
    pub async fn new(pool: SqlitePool, opts: GlobalOptions) -> Result<Self> {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut editor: Editor<CryptofolioCompleter, DefaultHistory> =
            Editor::with_config(config).map_err(|e| crate::error::CryptofolioError::Shell(e.to_string()))?;

        // Set up completer with available commands and accounts
        let completer = CryptofolioCompleter::new(&pool).await?;
        editor.set_helper(Some(completer));

        // Load history
        let history_path = AppConfig::config_dir()?.join("history.txt");
        let _ = editor.load_history(&history_path);

        Ok(Self {
            pool,
            opts,
            editor,
            context: ShellContext::new(),
        })
    }

    /// Run the interactive shell
    pub async fn run(&mut self) -> Result<()> {
        // Initialize colors
        init_color(self.opts.no_color);

        // Print welcome message
        self.print_welcome().await?;

        loop {
            // Build prompt with status
            let prompt = self.build_prompt().await;

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    // Add to history
                    let _ = self.editor.add_history_entry(line);

                    // Handle exit commands
                    if matches!(line.to_lowercase().as_str(), "exit" | "quit" | "q") {
                        println!("Goodbye!");
                        break;
                    }

                    // Handle help
                    if line == "help" || line == "?" {
                        self.print_help();
                        continue;
                    }

                    // Handle clear
                    if line == "clear" || line == "cls" {
                        print!("\x1B[2J\x1B[1;1H");
                        continue;
                    }

                    // Execute the command
                    if let Err(e) = self.execute_command(line).await {
                        crate::cli::output::error(&e.to_string());
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("exit");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        // Save history
        let history_path = AppConfig::config_dir()?.join("history.txt");
        let _ = self.editor.save_history(&history_path);

        Ok(())
    }

    /// Print welcome message with portfolio summary
    async fn print_welcome(&self) -> Result<()> {
        println!();
        if colors_enabled() {
            println!("  \x1b[1;36mCryptofolio\x1b[0m v{}", env!("CARGO_PKG_VERSION"));
        } else {
            println!("  Cryptofolio v{}", env!("CARGO_PKG_VERSION"));
        }
        println!("  AI-Powered Portfolio Assistant");
        println!();

        // Show portfolio summary
        if let Ok(summary) = self.get_portfolio_summary().await {
            println!("  Portfolio: {} ({})", summary.total_value, summary.pnl);
        }

        let config = AppConfig::load()?;
        if config.general.use_testnet || self.opts.testnet {
            println!("  Mode: \x1b[33mTestnet\x1b[0m");
        }

        println!();
        println!("  Type 'help' for commands, or describe what you want to do.");
        println!("  Press Ctrl+C to cancel, 'exit' to quit.");
        println!();

        Ok(())
    }

    /// Build the prompt string
    async fn build_prompt(&self) -> String {
        if colors_enabled() {
            "\x1b[1;32myou>\x1b[0m ".to_string()
        } else {
            "you> ".to_string()
        }
    }

    /// Execute a command
    async fn execute_command(&mut self, input: &str) -> Result<()> {
        // First, expand any shortcuts/aliases
        let expanded = expand_shortcuts(input);

        // Parse the input into arguments
        let args = match shell_words::split(&expanded) {
            Ok(args) => args,
            Err(e) => {
                return Err(crate::error::CryptofolioError::Shell(format!(
                    "Failed to parse command: {}",
                    e
                )));
            }
        };

        if args.is_empty() {
            return Ok(());
        }

        // Prepend "cryptofolio" to make it a valid CLI command
        let mut full_args = vec!["cryptofolio".to_string()];
        full_args.extend(args);

        // Apply context defaults (e.g., last used account)
        full_args = self.context.apply_defaults(&full_args);

        // Add global options
        if self.opts.json {
            full_args.push("--json".to_string());
        }
        if self.opts.quiet {
            full_args.push("--quiet".to_string());
        }
        if self.opts.testnet {
            full_args.push("--testnet".to_string());
        }

        // Parse and execute using clap
        match crate::cli::Cli::try_parse_from(&full_args) {
            Ok(cli) => {
                // Update context from this command
                self.context.update_from_command(&full_args);
                self.run_cli_command(cli).await?;
            }
            Err(e) => {
                // Check if it's a help request (which clap handles by "failing")
                let kind = e.kind();
                if kind == clap::error::ErrorKind::DisplayHelp
                    || kind == clap::error::ErrorKind::DisplayVersion
                {
                    print!("{}", e);
                } else {
                    // Try fuzzy matching to suggest corrections
                    if let Some(suggestion) = shortcuts::suggest_correction(input) {
                        if colors_enabled() {
                            println!("\x1b[33mUnknown command.\x1b[0m Did you mean '\x1b[36m{}\x1b[0m'?", suggestion);
                        } else {
                            println!("Unknown command. Did you mean '{}'?", suggestion);
                        }
                        println!("Type 'help' for available commands.");
                    } else {
                        // Show original error
                        println!("{}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Run a parsed CLI command
    async fn run_cli_command(&self, cli: crate::cli::Cli) -> Result<()> {
        use crate::cli::commands::*;
        use crate::cli::Commands;

        let opts = GlobalOptions::from_cli(&cli);

        match cli.command {
            Commands::Price { symbols } => {
                handle_price_command(symbols, &self.pool, &opts).await?;
            }
            Commands::Market { symbol, show_24h } => {
                handle_market_command(symbol, show_24h, &self.pool, &opts).await?;
            }
            Commands::Account { command } => {
                handle_account_command(command, &self.pool, &opts).await?;
                // Refresh completer with new accounts
                if let Some(helper) = self.editor.helper() {
                    // Note: Can't mutate here, would need RefCell or similar
                }
            }
            Commands::Category { command } => {
                handle_category_command(command, &self.pool, &opts).await?;
            }
            Commands::Holdings { command } => {
                handle_holdings_command(command, &self.pool, &opts).await?;
            }
            Commands::Portfolio {
                by_account,
                by_category,
                account,
                category,
            } => {
                handle_portfolio_command(by_account, by_category, account, category, &self.pool, &opts).await?;
            }
            Commands::Tx { command } => {
                handle_tx_command(command, &self.pool, &opts).await?;
            }
            Commands::Sync { account } => {
                handle_sync_command(account, &self.pool, &opts).await?;
            }
            Commands::Import {
                file,
                account,
                format,
            } => {
                handle_import_command(file, account, format, &self.pool, &opts).await?;
            }
            Commands::Config { command } => {
                handle_config_command(command, &self.pool, &opts).await?;
            }
            Commands::Shell => {
                // Already in shell mode, ignore
                println!("Already in shell mode.");
            }
        }

        Ok(())
    }

    /// Get a quick portfolio summary
    async fn get_portfolio_summary(&self) -> Result<PortfolioSummary> {
        let config = AppConfig::load()?;
        let use_testnet = self.opts.testnet || config.general.use_testnet;

        let account_repo = AccountRepository::new(&self.pool);
        let holding_repo = HoldingRepository::new(&self.pool);

        let accounts = account_repo.list_accounts().await?;
        let all_holdings = holding_repo.list_all().await?;

        if all_holdings.is_empty() {
            return Ok(PortfolioSummary {
                total_value: "$0.00".to_string(),
                pnl: "No holdings".to_string(),
            });
        }

        // Get unique assets
        let unique_assets: Vec<String> = all_holdings
            .iter()
            .map(|h| h.asset.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Fetch prices
        let client = BinanceClient::new(
            use_testnet,
            config.binance.api_key.clone(),
            config.binance.api_secret.clone(),
        );

        let asset_refs: Vec<&str> = unique_assets.iter().map(|s| s.as_str()).collect();
        let prices = client.get_prices(&asset_refs).await.unwrap_or_default();

        let price_map: std::collections::HashMap<String, rust_decimal::Decimal> = prices
            .into_iter()
            .map(|p| (p.symbol.to_uppercase(), p.price))
            .collect();

        // Calculate total value
        let mut total_value = rust_decimal::Decimal::ZERO;
        let mut total_cost = rust_decimal::Decimal::ZERO;

        for holding in &all_holdings {
            if let Some(price) = price_map.get(&holding.asset.to_uppercase()) {
                total_value += holding.quantity * price;
            }
            if let Some(cost) = holding.avg_cost_basis {
                total_cost += holding.quantity * cost;
            }
        }

        let pnl = total_value - total_cost;
        let pnl_percent = if total_cost > rust_decimal::Decimal::ZERO {
            (pnl / total_cost) * rust_decimal::Decimal::from(100)
        } else {
            rust_decimal::Decimal::ZERO
        };

        let pnl_str = if pnl >= rust_decimal::Decimal::ZERO {
            if colors_enabled() {
                format!("\x1b[32m+{} (+{:.2}%)\x1b[0m", format_usd(pnl), pnl_percent)
            } else {
                format!("+{} (+{:.2}%)", format_usd(pnl), pnl_percent)
            }
        } else {
            if colors_enabled() {
                format!("\x1b[31m{} ({:.2}%)\x1b[0m", format_usd(pnl), pnl_percent)
            } else {
                format!("{} ({:.2}%)", format_usd(pnl), pnl_percent)
            }
        };

        Ok(PortfolioSummary {
            total_value: format_usd(total_value),
            pnl: pnl_str,
        })
    }

    /// Print help message
    fn print_help(&self) {
        println!();
        println!("  \x1b[1mAvailable Commands:\x1b[0m");
        println!();
        println!("  \x1b[36mprice\x1b[0m <symbol>...      Get current prices");
        println!("  \x1b[36mmarket\x1b[0m <symbol>        Get detailed market data");
        println!("  \x1b[36mportfolio\x1b[0m              View portfolio with P&L");
        println!("  \x1b[36mholdings\x1b[0m list          List all holdings");
        println!("  \x1b[36mholdings\x1b[0m add           Add holdings");
        println!("  \x1b[36maccount\x1b[0m list           List accounts");
        println!("  \x1b[36maccount\x1b[0m add            Add new account");
        println!("  \x1b[36mtx\x1b[0m buy/sell/transfer  Record transactions");
        println!("  \x1b[36msync\x1b[0m                   Sync from exchanges");
        println!("  \x1b[36mconfig\x1b[0m show            Show configuration");
        println!();
        println!("  \x1b[1mShortcuts:\x1b[0m");
        println!();
        println!("  \x1b[36mp\x1b[0m = portfolio    \x1b[36mh\x1b[0m = holdings    \x1b[36ma\x1b[0m = account");
        println!("  \x1b[36ms\x1b[0m = sync         \x1b[36mm\x1b[0m = market      \x1b[36mc\x1b[0m = config");
        println!("  \x1b[36mbuy\x1b[0m = tx buy     \x1b[36msell\x1b[0m = tx sell  \x1b[36mls\x1b[0m = holdings list");
        println!();
        println!("  \x1b[1mShell Commands:\x1b[0m");
        println!();
        println!("  \x1b[36mhelp\x1b[0m                   Show this help");
        println!("  \x1b[36mclear\x1b[0m                  Clear screen");
        println!("  \x1b[36mexit\x1b[0m                   Exit shell");
        println!();
        println!("  Use Tab for completion, Up/Down for history.");

        // Show current context if any
        if let Some(ctx_summary) = self.context.summary() {
            println!();
            println!("  \x1b[1mCurrent context:\x1b[0m {}", ctx_summary);
        }

        println!();
    }
}

struct PortfolioSummary {
    total_value: String,
    pnl: String,
}
