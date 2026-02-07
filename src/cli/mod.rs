pub mod commands;
pub mod output;

use clap::{Parser, Subcommand, ValueEnum};

const AFTER_HELP: &str = r#"EXAMPLES:
    cryptofolio price BTC ETH              Get current prices
    cryptofolio portfolio                  View portfolio with P&L
    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000

LEARN MORE:
    Documentation: https://github.com/yzumbado/cryptofolio
    Report bugs:   https://github.com/yzumbado/cryptofolio/issues"#;

const AFTER_LONG_HELP: &str = r#"EXAMPLES:
    # Check cryptocurrency prices
    cryptofolio price BTC ETH SOL

    # View your portfolio with profit/loss
    cryptofolio portfolio

    # Add a hardware wallet account
    cryptofolio account add "Ledger" --type hardware_wallet --category cold-storage

    # Add holdings with cost basis
    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000

    # Sync from Binance exchange
    cryptofolio sync --account "Binance"

    # Export as JSON for scripting
    cryptofolio portfolio --json

ENVIRONMENT VARIABLES:
    CRYPTOFOLIO_TESTNET     Set to "1" to use testnet mode
    CRYPTOFOLIO_NO_COLOR    Set to disable colored output
    NO_COLOR                Standard flag to disable colors

CONFIGURATION:
    Config file: ~/.config/cryptofolio/config.toml
    Database:    ~/.config/cryptofolio/database.sqlite

LEARN MORE:
    Documentation: https://github.com/yzumbado/cryptofolio
    Report bugs:   https://github.com/yzumbado/cryptofolio/issues"#;

#[derive(Parser)]
#[command(name = "cryptofolio")]
#[command(author = "Your Name")]
#[command(version)]
#[command(about = "A CLI tool for managing crypto portfolios across exchanges and wallets")]
#[command(long_about = "Cryptofolio helps you track cryptocurrency holdings across multiple locations - exchanges, hardware wallets, and software wallets - with real-time P&L calculations.")]
#[command(after_help = AFTER_HELP)]
#[command(after_long_help = AFTER_LONG_HELP)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Use testnet mode
    #[arg(long, global = true)]
    pub testnet: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Enable verbose/debug output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get current price for one or more cryptocurrencies
    #[command(after_help = "EXAMPLES:\n    cryptofolio price BTC\n    cryptofolio price BTC ETH SOL\n    cryptofolio price BTC --json")]
    Price {
        /// Cryptocurrency symbols (e.g., BTC ETH SOL)
        #[arg(required = true)]
        symbols: Vec<String>,
    },

    /// Get detailed market data for a cryptocurrency
    #[command(after_help = "EXAMPLES:\n    cryptofolio market BTC\n    cryptofolio market ETH --24h")]
    Market {
        /// Cryptocurrency symbol (e.g., BTC)
        symbol: String,

        /// Show 24-hour statistics
        #[arg(long = "24h")]
        show_24h: bool,
    },

    /// Manage accounts (exchanges, wallets)
    #[command(after_help = "EXAMPLES:\n    cryptofolio account list\n    cryptofolio account add \"Ledger\" --type hardware_wallet --category cold-storage\n    cryptofolio account show \"Ledger\"")]
    Account {
        #[command(subcommand)]
        command: AccountCommands,
    },

    /// Manage categories for organizing accounts
    #[command(after_help = "EXAMPLES:\n    cryptofolio category list\n    cryptofolio category add \"DeFi\"")]
    Category {
        #[command(subcommand)]
        command: CategoryCommands,
    },

    /// Manage holdings across accounts
    #[command(after_help = "EXAMPLES:\n    cryptofolio holdings list\n    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000\n    cryptofolio holdings move BTC 0.1 --from Binance --to Ledger")]
    Holdings {
        #[command(subcommand)]
        command: HoldingsCommands,
    },

    /// View portfolio with P&L calculations
    #[command(after_help = "EXAMPLES:\n    cryptofolio portfolio\n    cryptofolio portfolio --by-category\n    cryptofolio portfolio --json")]
    Portfolio {
        /// Group by account
        #[arg(long = "by-account")]
        by_account: bool,

        /// Group by category
        #[arg(long = "by-category")]
        by_category: bool,

        /// Filter by account name
        #[arg(long)]
        account: Option<String>,

        /// Filter by category name
        #[arg(long)]
        category: Option<String>,
    },

    /// Record and view transactions
    #[command(after_help = "EXAMPLES:\n    cryptofolio tx list\n    cryptofolio tx buy BTC 0.1 --account Binance --price 95000\n    cryptofolio tx transfer BTC 0.5 --from Binance --to Ledger")]
    Tx {
        #[command(subcommand)]
        command: TxCommands,
    },

    /// Sync holdings from exchange accounts
    #[command(after_help = "EXAMPLES:\n    cryptofolio sync\n    cryptofolio sync --account \"Binance\"")]
    Sync {
        /// Account to sync (syncs all exchange accounts if not specified)
        #[arg(long)]
        account: Option<String>,
    },

    /// Import transactions from CSV file
    #[command(after_help = "EXAMPLES:\n    cryptofolio import transactions.csv --account Ledger\n\nCSV FORMAT:\n    date,type,asset,quantity,price_usd,fee,notes\n    2024-01-15,buy,BTC,0.5,45000,0.001,First purchase")]
    Import {
        /// Path to CSV file
        file: String,

        /// Account to import into
        #[arg(long, required = true)]
        account: String,

        /// File format (csv)
        #[arg(long, default_value = "csv")]
        format: String,
    },

    /// Manage configuration settings
    #[command(after_help = "EXAMPLES:\n    cryptofolio config show\n    cryptofolio config set binance.api_key YOUR_KEY\n    cryptofolio config use-testnet")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Start interactive shell mode
    #[command(after_help = "EXAMPLES:\n    cryptofolio shell\n\nIn shell mode, you can:\n    - Run commands without typing 'cryptofolio' prefix\n    - Use Tab for auto-completion\n    - Use Up/Down for command history\n    - Type natural language (AI mode)")]
    Shell,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// List all accounts
    List,

    /// Add a new account
    #[command(after_help = "EXAMPLES:\n    cryptofolio account add \"Ledger\" --type hardware_wallet --category cold-storage\n    cryptofolio account add \"Binance\" --type exchange --category trading --sync --testnet")]
    Add {
        /// Account name
        name: String,

        /// Account type
        #[arg(long = "type", required = true, value_enum)]
        account_type: AccountTypeArg,

        /// Category (trading, cold-storage, hot-wallets, or custom)
        #[arg(long, required = true)]
        category: String,

        /// Mark as testnet account (for exchanges)
        #[arg(long)]
        testnet: bool,

        /// Enable auto-sync (for exchanges)
        #[arg(long)]
        sync: bool,
    },

    /// Remove an account
    #[command(after_help = "EXAMPLES:\n    cryptofolio account remove \"Old Wallet\"")]
    Remove {
        /// Account name
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Show account details
    Show {
        /// Account name
        name: String,
    },

    /// Manage wallet addresses
    Address {
        #[command(subcommand)]
        command: AddressCommands,
    },
}

#[derive(Clone, ValueEnum)]
pub enum AccountTypeArg {
    Exchange,
    HardwareWallet,
    SoftwareWallet,
    CustodialService,
}

impl AccountTypeArg {
    pub fn to_string(&self) -> &'static str {
        match self {
            AccountTypeArg::Exchange => "exchange",
            AccountTypeArg::HardwareWallet => "hardware_wallet",
            AccountTypeArg::SoftwareWallet => "software_wallet",
            AccountTypeArg::CustodialService => "custodial_service",
        }
    }
}

#[derive(Subcommand)]
pub enum AddressCommands {
    /// Add a wallet address
    Add {
        /// Account name
        account: String,

        /// Blockchain (bitcoin, ethereum, solana, etc.)
        blockchain: String,

        /// Wallet address
        address: String,

        /// Optional label
        #[arg(long)]
        label: Option<String>,
    },

    /// List addresses for an account
    List {
        /// Account name
        account: String,
    },

    /// Remove a wallet address
    Remove {
        /// Account name
        account: String,

        /// Wallet address
        address: String,
    },
}

#[derive(Subcommand)]
pub enum CategoryCommands {
    /// List all categories
    List,

    /// Add a new category
    Add {
        /// Category name
        name: String,
    },

    /// Rename a category
    Rename {
        /// Current name
        old_name: String,

        /// New name
        new_name: String,
    },

    /// Remove a category
    Remove {
        /// Category name
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum HoldingsCommands {
    /// List holdings
    List {
        /// Filter by account
        #[arg(long)]
        account: Option<String>,
    },

    /// Add to holdings
    #[command(after_help = "EXAMPLES:\n    cryptofolio holdings add BTC 0.5 --account Ledger\n    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000")]
    Add {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Quantity to add
        quantity: String,

        /// Account name
        #[arg(long, required = true)]
        account: String,

        /// Cost per unit in USD
        #[arg(long)]
        cost: Option<String>,
    },

    /// Remove from holdings
    Remove {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Quantity to remove
        quantity: String,

        /// Account name
        #[arg(long, required = true)]
        account: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Set exact holding amount
    Set {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Exact quantity
        quantity: String,

        /// Account name
        #[arg(long, required = true)]
        account: String,

        /// Cost per unit in USD
        #[arg(long)]
        cost: Option<String>,
    },

    /// Move holdings between accounts
    Move {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Quantity to move
        quantity: String,

        /// Source account
        #[arg(long, required = true)]
        from: String,

        /// Destination account
        #[arg(long, required = true)]
        to: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum TxCommands {
    /// List transactions
    List {
        /// Filter by account
        #[arg(long)]
        account: Option<String>,

        /// Maximum number of transactions
        #[arg(long, default_value = "50")]
        limit: i64,
    },

    /// Record a buy transaction
    #[command(after_help = "EXAMPLES:\n    cryptofolio tx buy BTC 0.1 --account Binance --price 95000\n    cryptofolio tx buy ETH 2.0 --account Binance --price 3200 --notes \"DCA\"")]
    Buy {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Quantity
        quantity: String,

        /// Account name
        #[arg(long, required = true)]
        account: String,

        /// Price per unit in USD
        #[arg(long, required = true)]
        price: String,

        /// Transaction notes
        #[arg(long)]
        notes: Option<String>,

        /// Simulate without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Record a sell transaction
    Sell {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Quantity
        quantity: String,

        /// Account name
        #[arg(long, required = true)]
        account: String,

        /// Price per unit in USD
        #[arg(long, required = true)]
        price: String,

        /// Transaction notes
        #[arg(long)]
        notes: Option<String>,

        /// Simulate without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Record a transfer between accounts
    Transfer {
        /// Asset symbol (e.g., BTC)
        asset: String,

        /// Quantity
        quantity: String,

        /// Source account
        #[arg(long, required = true)]
        from: String,

        /// Destination account
        #[arg(long, required = true)]
        to: String,

        /// Transfer fee amount
        #[arg(long)]
        fee: Option<String>,

        /// Transaction notes
        #[arg(long)]
        notes: Option<String>,

        /// Simulate without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Record a swap transaction
    Swap {
        /// Source asset (e.g., ETH)
        from_asset: String,

        /// Source quantity
        from_quantity: String,

        /// Destination asset (e.g., BTC)
        to_asset: String,

        /// Destination quantity
        to_quantity: String,

        /// Account name
        #[arg(long, required = true)]
        account: String,

        /// Transaction notes
        #[arg(long)]
        notes: Option<String>,

        /// Simulate without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set a configuration value
    #[command(after_help = "EXAMPLES:\n    cryptofolio config set binance.api_key YOUR_KEY\n    cryptofolio config set general.use_testnet true\n\nKEYS:\n    binance.api_key        Binance API key\n    binance.api_secret     Binance API secret\n    general.use_testnet    Enable testnet mode (true/false)\n    general.default_account Default account name\n    display.color          Enable colors (true/false)")]
    Set {
        /// Configuration key (e.g., binance.api_key)
        key: String,

        /// Configuration value (omit to read from stdin for secrets)
        value: Option<String>,
    },

    /// Enable testnet mode
    #[command(name = "use-testnet")]
    UseTestnet,

    /// Disable testnet mode (use mainnet)
    #[command(name = "use-mainnet")]
    UseMainnet,
}

/// Global options that affect command behavior
#[derive(Debug, Clone)]
pub struct GlobalOptions {
    pub no_color: bool,
    pub testnet: bool,
    pub json: bool,
    pub quiet: bool,
    pub verbose: bool,
}

impl GlobalOptions {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            no_color: cli.no_color,
            testnet: cli.testnet || std::env::var("CRYPTOFOLIO_TESTNET").is_ok(),
            json: cli.json,
            quiet: cli.quiet,
            verbose: cli.verbose,
        }
    }
}
