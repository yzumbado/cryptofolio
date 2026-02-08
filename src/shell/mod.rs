mod completer;
mod context;
mod shortcuts;

use std::io::{self, Write};

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Config, Editor};
use sqlx::SqlitePool;

use crate::ai::{AiMode, AiService, ConversationAction, ConversationManager};
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
    ai_service: Option<AiService>,
    conversation: ConversationManager,
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

        // Initialize AI service
        let app_config = AppConfig::load()?;
        let ai_service = AiService::new(&app_config).ok();

        Ok(Self {
            pool,
            opts,
            editor,
            context: ShellContext::new(),
            ai_service,
            conversation: ConversationManager::new(),
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
                    if let Err(e) = self.execute_input(line).await {
                        crate::cli::output::error(&e.to_string());
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Cancel current operation
                    self.conversation.state_mut().clear_operation();
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
            println!("  \x1b[1;36m🪙 Cryptofolio\x1b[0m v{}", env!("CARGO_PKG_VERSION"));
        } else {
            println!("  Cryptofolio v{}", env!("CARGO_PKG_VERSION"));
        }
        println!("  AI-Powered Portfolio Assistant");
        println!();

        // Show portfolio summary
        if let Ok(summary) = self.get_portfolio_summary().await {
            println!("  💰 Portfolio: {} ({})", summary.total_value, summary.pnl);
        }

        // Show system status (network mode + AI status)
        crate::cli::commands::status::print_startup_summary().await;

        println!();
        println!("  Type 'help' for commands, or describe what you want to do.");
        println!("  Use 'status' for full system diagnostics.");
        println!("  Press Ctrl+C to cancel, 'exit' to quit.");
        println!();

        Ok(())
    }

    /// Build the prompt string
    async fn build_prompt(&self) -> String {
        // Show different prompt during conversation
        if self.conversation.state().confirmation_pending {
            if colors_enabled() {
                return "\x1b[1;33m[Y/n]\x1b[0m ".to_string();
            } else {
                return "[Y/n] ".to_string();
            }
        }

        if self.conversation.state().current_intent.is_some() {
            if colors_enabled() {
                return "\x1b[1;34m  >\x1b[0m ".to_string();
            } else {
                return "  > ".to_string();
            }
        }

        if colors_enabled() {
            "\x1b[1;32myou>\x1b[0m ".to_string()
        } else {
            "you> ".to_string()
        }
    }

    /// Execute user input - either as CLI command or natural language
    async fn execute_input(&mut self, input: &str) -> Result<()> {
        // Check if we're in the middle of a conversation
        if self.conversation.state().confirmation_pending {
            return self.handle_confirmation(input).await;
        }

        if self.conversation.state().current_intent.is_some() {
            return self.handle_conversation_input(input).await;
        }

        // First, try to parse as CLI command
        let expanded = expand_shortcuts(input);
        let args = match shell_words::split(&expanded) {
            Ok(args) => args,
            Err(_) => vec![],
        };

        if !args.is_empty() {
            // Check if first word looks like a CLI command
            let first_word = args[0].to_lowercase();
            let cli_commands = [
                "price", "market", "portfolio", "holdings", "account",
                "category", "tx", "sync", "import", "config", "status",
            ];

            if cli_commands.contains(&first_word.as_str()) {
                return self.execute_cli_command(&expanded).await;
            }
        }

        // Try AI natural language processing
        if let Some(ref ai) = self.ai_service {
            if ai.is_available() {
                return self.process_natural_language(input).await;
            }
        }

        // Fallback: try as CLI command anyway
        self.execute_cli_command(&expanded).await
    }

    /// Process natural language input with AI
    async fn process_natural_language(&mut self, input: &str) -> Result<()> {
        let ai = match &self.ai_service {
            Some(ai) => ai,
            None => {
                // No AI available, show error
                println!("AI not available. Please use CLI commands.");
                return Ok(());
            }
        };

        // Update conversation context from shell context
        self.conversation.state_mut().last_account = self.context.last_account.clone();
        self.conversation.state_mut().last_asset = self.context.last_asset.clone();

        // Parse with AI
        let parsed = ai.parse_input(input, self.conversation.state()).await?;

        // Process through conversation manager
        let action = self.conversation.process(parsed);

        self.handle_conversation_action(action).await
    }

    /// Handle a conversation action
    async fn handle_conversation_action(&mut self, action: ConversationAction) -> Result<()> {
        match action {
            ConversationAction::Clarify { question, field: _, suggestions } => {
                println!();
                if colors_enabled() {
                    println!("  \x1b[36m{}\x1b[0m", question);
                } else {
                    println!("  {}", question);
                }
                if !suggestions.is_empty() {
                    println!("  ({})", suggestions.join(", "));
                }
                println!();
            }
            ConversationAction::Confirm { summary, command: _, details } => {
                println!();
                println!("  {}", summary);
                println!();
                for (key, value) in details {
                    println!("  {}: {}", key, value);
                }
                println!();
                if colors_enabled() {
                    print!("  \x1b[1mConfirm?\x1b[0m ");
                } else {
                    print!("  Confirm? ");
                }
                io::stdout().flush().ok();
            }
            ConversationAction::Execute { command } => {
                // Update shell context
                if let Some(ref account) = self.conversation.state().last_account {
                    self.context.last_account = Some(account.clone());
                }
                if let Some(ref asset) = self.conversation.state().last_asset {
                    self.context.last_asset = Some(asset.clone());
                }

                // Execute the command
                self.execute_cli_command(&command).await?;
            }
            ConversationAction::Cancel { message } => {
                println!();
                if colors_enabled() {
                    println!("  \x1b[33m{}\x1b[0m", message);
                } else {
                    println!("  {}", message);
                }
                println!();
            }
            ConversationAction::Disambiguate { message, options } => {
                println!();
                println!("  {}", message);
                for (i, option) in options.iter().enumerate() {
                    println!("  {}. {}", i + 1, option);
                }
                println!();
            }
            ConversationAction::Respond { message } => {
                println!();
                println!("  {}", message);
                println!();
            }
            ConversationAction::OutOfScope { message } => {
                println!();
                if colors_enabled() {
                    println!("  \x1b[33m{}\x1b[0m", message);
                } else {
                    println!("  {}", message);
                }
                println!();
            }
        }

        Ok(())
    }

    /// Handle confirmation response
    async fn handle_confirmation(&mut self, input: &str) -> Result<()> {
        let action = self.conversation.handle_confirmation(input);
        self.handle_conversation_action(action).await
    }

    /// Handle input during ongoing conversation
    async fn handle_conversation_input(&mut self, input: &str) -> Result<()> {
        let state = self.conversation.state();

        // Determine what field we're collecting
        if let Some(field) = state.missing_entities.first() {
            let field = field.clone();
            if let Some(entity) = self.conversation.handle_entity_input(input, &field) {
                // Add entity to collected
                self.conversation.state_mut().collected_entities.insert(field.clone(), entity);
                self.conversation.state_mut().missing_entities.retain(|f| f != &field);

                // Check if we have everything
                if self.conversation.state().missing_entities.is_empty() {
                    // All collected, show confirmation
                    if let Some(ref intent) = self.conversation.state().current_intent.clone() {
                        if intent.requires_confirmation() {
                            self.conversation.state_mut().confirmation_pending = true;
                            let (summary, details) = self.build_confirmation(&intent);
                            println!();
                            println!("  {}", summary);
                            println!();
                            for (key, value) in details {
                                println!("  {}: {}", key, value);
                            }
                            println!();
                            print!("  Confirm? [Y/n] ");
                            io::stdout().flush().ok();
                        } else {
                            // Execute immediately
                            let command = self.build_command(&intent);
                            self.conversation.state_mut().clear_operation();
                            self.execute_cli_command(&command).await?;
                        }
                    }
                } else {
                    // Ask for next missing field
                    let next_field = self.conversation.state().missing_entities[0].clone();
                    let question = self.get_question_for_field(&next_field);
                    println!();
                    if colors_enabled() {
                        println!("  \x1b[36m{}\x1b[0m", question);
                    } else {
                        println!("  {}", question);
                    }
                }
            } else {
                // Couldn't parse input
                println!("  Please provide a valid value.");
            }
        }

        Ok(())
    }

    /// Build confirmation summary for an intent
    fn build_confirmation(&self, intent: &crate::ai::Intent) -> (String, Vec<(String, String)>) {
        use crate::ai::intent::Entity;

        let mut details = Vec::new();
        let state = self.conversation.state();

        let action = match intent {
            crate::ai::Intent::TxBuy => "BUY",
            crate::ai::Intent::TxSell => "SELL",
            crate::ai::Intent::TxTransfer | crate::ai::Intent::HoldingsMove => "TRANSFER",
            crate::ai::Intent::HoldingsAdd => "ADD",
            _ => "EXECUTE",
        };

        if let Some(Entity::String(asset)) = state.collected_entities.get("asset") {
            details.push(("Asset".to_string(), asset.clone()));
        }
        if let Some(Entity::Number(qty)) = state.collected_entities.get("quantity") {
            details.push(("Quantity".to_string(), format!("{}", qty)));
        }
        if let Some(Entity::Number(price)) = state.collected_entities.get("price") {
            details.push(("Price".to_string(), format!("${:.2}", price)));
        }
        if let Some(Entity::String(account)) = state.collected_entities.get("account") {
            details.push(("Account".to_string(), account.clone()));
        }
        if let Some(Entity::String(from)) = state.collected_entities.get("from_account") {
            details.push(("From".to_string(), from.clone()));
        }
        if let Some(Entity::String(to)) = state.collected_entities.get("to_account") {
            details.push(("To".to_string(), to.clone()));
        }

        // Calculate total for buy/sell
        if matches!(intent, crate::ai::Intent::TxBuy | crate::ai::Intent::TxSell) {
            if let (Some(Entity::Number(qty)), Some(Entity::Number(price))) = (
                state.collected_entities.get("quantity"),
                state.collected_entities.get("price"),
            ) {
                let total = qty * price;
                details.push(("Total".to_string(), format!("${:.2}", total)));
            }
        }

        (format!("Transaction: {}", action), details)
    }

    /// Build CLI command from conversation state
    fn build_command(&self, intent: &crate::ai::Intent) -> String {
        use crate::ai::intent::{Entity, ParsedInput};

        let parsed = ParsedInput {
            intent: intent.clone(),
            entities: self.conversation.state().collected_entities.clone(),
            missing: vec![],
            confidence: 1.0,
            raw_input: String::new(),
        };

        parsed.to_cli_command().unwrap_or_default()
    }

    /// Get question for a missing field
    fn get_question_for_field(&self, field: &str) -> String {
        match field {
            "quantity" => "How much?".to_string(),
            "price" => "What price per unit?".to_string(),
            "account" => "Which account?".to_string(),
            "from_account" => "From which account?".to_string(),
            "to_account" => "To which account?".to_string(),
            "asset" => "Which cryptocurrency?".to_string(),
            _ => format!("Please provide {}:", field),
        }
    }

    /// Execute a CLI command
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
                println!("Already in shell mode.");
            }
            Commands::Status { check } => {
                handle_status_command(check).await?;
            }
        }

        Ok(())
    }

    /// Get a quick portfolio summary
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
        println!("  \x1b[1mNatural Language:\x1b[0m");
        println!();
        println!("  You can also type naturally, e.g.:");
        println!("    \"I bought 0.1 BTC on Binance\"");
        println!("    \"What's the price of ethereum?\"");
        println!("    \"Show my portfolio\"");
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
