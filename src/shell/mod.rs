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
use crate::db::HoldingRepository;
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
    pub async fn new(pool: SqlitePool, opts: GlobalOptions) -> Result<Self> {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut editor: Editor<CryptofolioCompleter, DefaultHistory> = Editor::with_config(config)
            .map_err(|e| crate::error::CryptofolioError::Shell(e.to_string()))?;

        let completer = CryptofolioCompleter::new(&pool).await?;
        editor.set_helper(Some(completer));

        let history_path = AppConfig::config_dir()?.join("history.txt");
        let _ = editor.load_history(&history_path);

        Ok(Self {
            pool,
            opts,
            editor,
            context: ShellContext::new(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        init_color(self.opts.no_color);
        self.print_welcome().await?;

        loop {
            let prompt = if colors_enabled() {
                "\x1b[1;32myou>\x1b[0m ".to_string()
            } else {
                "you> ".to_string()
            };

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    let _ = self.editor.add_history_entry(line);

                    if matches!(line.to_lowercase().as_str(), "exit" | "quit" | "q") {
                        println!("Goodbye!");
                        break;
                    }

                    if line == "help" || line == "?" {
                        self.print_help();
                        continue;
                    }

                    if line == "clear" || line == "cls" {
                        print!("\x1B[2J\x1B[1;1H");
                        continue;
                    }

                    if let Err(e) = self.execute_cli_command(line).await {
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

        let history_path = AppConfig::config_dir()?.join("history.txt");
        let _ = self.editor.save_history(&history_path);

        Ok(())
    }

    async fn print_welcome(&self) -> Result<()> {
        println!();
        if colors_enabled() {
            println!(
                "  \x1b[1;36m🪙 Cryptofolio\x1b[0m v{}",
                env!("CARGO_PKG_VERSION")
            );
        } else {
            println!("  Cryptofolio v{}", env!("CARGO_PKG_VERSION"));
        }
        println!("  Interactive Portfolio Shell");
        println!();

        if let Ok(summary) = self.get_portfolio_summary().await {
            println!("  💰 Portfolio: {} ({})", summary.total_value, summary.pnl);
        }

        crate::cli::commands::status::print_startup_summary().await;

        println!();
        println!("  Type 'help' for commands or 'exit' to quit.");
        println!("  Use Tab for auto-completion, Up/Down for history.");
        println!();

        Ok(())
    }

    async fn execute_cli_command(&mut self, input: &str) -> Result<()> {
        let expanded = expand_shortcuts(input);

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

        let mut full_args = vec!["cryptofolio".to_string()];
        full_args.extend(args);
        full_args = self.context.apply_defaults(&full_args);

        if self.opts.json {
            full_args.push("--json".to_string());
        }
        if self.opts.quiet {
            full_args.push("--quiet".to_string());
        }
        if self.opts.testnet {
            full_args.push("--testnet".to_string());
        }

        match crate::cli::Cli::try_parse_from(&full_args) {
            Ok(cli) => {
                self.context.update_from_command(&full_args);
                self.run_cli_command(cli).await?;
            }
            Err(e) => {
                let kind = e.kind();
                if kind == clap::error::ErrorKind::DisplayHelp
                    || kind == clap::error::ErrorKind::DisplayVersion
                {
                    print!("{}", e);
                } else if let Some(suggestion) = shortcuts::suggest_correction(input) {
                    if colors_enabled() {
                        println!(
                            "\x1b[33mUnknown command.\x1b[0m Did you mean '\x1b[36m{}\x1b[0m'?",
                            suggestion
                        );
                    } else {
                        println!("Unknown command. Did you mean '{}'?", suggestion);
                    }
                    println!("Type 'help' for available commands.");
                } else {
                    println!("{}", e);
                }
            }
        }

        Ok(())
    }

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
                handle_portfolio_command(
                    by_account,
                    by_category,
                    account,
                    category,
                    &self.pool,
                    &opts,
                )
                .await?;
            }
            Commands::Tx { command } => {
                handle_tx_command(command, &self.pool, &opts).await?;
            }
            Commands::Sync { account } => {
                handle_sync_command(account, &self.pool, &opts).await?;
            }
            Commands::SyncHistory {
                account,
                symbols,
                full_history,
                from,
                no_trades,
                no_deposits,
                no_withdrawals,
                no_fiat,
                no_transfers,
                dry_run,
            } => {
                handle_sync_history_command(
                    account,
                    symbols,
                    full_history,
                    from,
                    no_trades,
                    no_deposits,
                    no_withdrawals,
                    no_fiat,
                    no_transfers,
                    dry_run,
                    &self.pool,
                    &opts,
                )
                .await?;
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
            Commands::Currency { command } => {
                handle_currency_command(&self.pool, command).await?;
            }
            Commands::Pnl { command } => {
                handle_pnl_command(command, &self.pool, &opts).await?;
            }
            Commands::Wallet { command } => {
                handle_wallet_command(command, &self.pool, &opts).await?;
            }
            Commands::Audit { command } => {
                handle_audit_command(command, &self.pool, &opts).await?;
            }
            Commands::Shell => {
                println!("Already in shell mode.");
            }
            Commands::Status { check } => {
                handle_status_command(check).await?;
            }
        }

        Ok(())
    }

    async fn get_portfolio_summary(&self) -> Result<PortfolioSummary> {
        let config = AppConfig::load()?;
        let use_testnet = self.opts.testnet || config.general.use_testnet;

        let holding_repo = HoldingRepository::new(&self.pool);
        let all_holdings = holding_repo.list_all().await?;

        if all_holdings.is_empty() {
            return Ok(PortfolioSummary {
                total_value: "$0.00".to_string(),
                pnl: "No holdings".to_string(),
            });
        }

        let unique_assets: Vec<String> = all_holdings
            .iter()
            .map(|h| h.asset.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

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
        } else if colors_enabled() {
            format!("\x1b[31m{} ({:.2}%)\x1b[0m", format_usd(pnl), pnl_percent)
        } else {
            format!("{} ({:.2}%)", format_usd(pnl), pnl_percent)
        };

        Ok(PortfolioSummary {
            total_value: format_usd(total_value),
            pnl: pnl_str,
        })
    }

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
