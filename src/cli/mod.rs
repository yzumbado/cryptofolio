#![allow(dead_code)]

pub mod commands;
pub mod notifications;
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
    cryptofolio portfolio --by-category

    # Add a hardware wallet account
    cryptofolio account add "Ledger" --type hardware_wallet --category cold-storage

    # Add holdings with cost basis
    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000

    # Record transactions
    cryptofolio tx buy BTC 0.1 --account Binance --price 95000

    # Sync from Binance exchange
    cryptofolio sync --account "Binance"

    # Export as JSON for scripting/automation
    cryptofolio portfolio --json
    cryptofolio holdings list --json
    cryptofolio account list --json

WORKFLOW EXAMPLE:
    # 1. Configure API keys securely
    cryptofolio config set-secret binance.api_key
    cryptofolio config set-secret binance.api_secret

    # 2. Create accounts
    cryptofolio account add "Binance" --type exchange --category trading --sync

    # 3. Sync holdings
    cryptofolio sync --account "Binance"

    # 4. View portfolio
    cryptofolio portfolio

AUTOMATION EXAMPLE:
    # Extract portfolio value for monitoring
    TOTAL=$(cryptofolio portfolio --json --quiet | jq -r '.total_value_usd')
    echo "Portfolio value: $$TOTAL"

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
    #[command(after_help = "EXAMPLES:\n    # Get single price\n    cryptofolio price BTC\n\n    # Get multiple prices\n    cryptofolio price BTC ETH SOL\n\n    # JSON output for scripting\n    cryptofolio price BTC --json\n    cryptofolio price BTC ETH --json | jq '.[0].price'")]
    Price {
        /// Cryptocurrency symbols (e.g., BTC ETH SOL)
        #[arg(required = true)]
        symbols: Vec<String>,
    },

    /// Get detailed market data for a cryptocurrency
    #[command(after_help = "EXAMPLES:\n    # Get current market price\n    cryptofolio market BTC\n    cryptofolio market ETHUSDT\n\n    # Include 24-hour statistics\n    cryptofolio market BTC --24h\n\n    # JSON output with 24h data\n    cryptofolio market BTCUSDT --24h --json")]
    Market {
        /// Cryptocurrency symbol (e.g., BTC, BTCUSDT)
        symbol: String,

        /// Show 24-hour statistics (price change, volume, high/low)
        #[arg(long = "24h")]
        show_24h: bool,
    },

    /// Manage accounts (exchanges, wallets)
    #[command(after_help = "EXAMPLES:\n    # List all accounts\n    cryptofolio account list\n    cryptofolio account list --json\n\n    # Add different account types\n    cryptofolio account add \"Ledger\" --type hardware_wallet --category cold-storage\n    cryptofolio account add \"Binance\" --type exchange --category trading --sync\n    cryptofolio account add \"MetaMask\" --type software_wallet --category hot-wallets\n\n    # Show account details\n    cryptofolio account show \"Ledger\"\n    cryptofolio account show \"Binance\" --json")]
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
    #[command(after_help = "EXAMPLES:\n    # List all holdings\n    cryptofolio holdings list\n    cryptofolio holdings list --account Binance\n    cryptofolio holdings list --json\n\n    # Add holdings with cost basis\n    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000\n    cryptofolio holdings add ETH 2.0 --account MetaMask --cost 2800\n\n    # Move holdings between accounts\n    cryptofolio holdings move BTC 0.1 --from Binance --to Ledger --yes")]
    Holdings {
        #[command(subcommand)]
        command: HoldingsCommands,
    },

    /// View portfolio with P&L calculations
    #[command(after_help = "EXAMPLES:\n    # View full portfolio\n    cryptofolio portfolio\n\n    # Group by category or account\n    cryptofolio portfolio --by-category\n    cryptofolio portfolio --by-account\n\n    # Filter by account or category\n    cryptofolio portfolio --account Binance\n    cryptofolio portfolio --category cold-storage\n\n    # JSON output for automation\n    cryptofolio portfolio --json\n    cryptofolio portfolio --json | jq '.total_value_usd'")]
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
    #[command(after_help = "EXAMPLES:\n    # List transactions\n    cryptofolio tx list\n    cryptofolio tx list --limit 50 --json\n    cryptofolio tx list --account Binance\n\n    # Record buy/sell transactions\n    cryptofolio tx buy BTC 0.1 --account Binance --price 95000 --notes \"DCA purchase\"\n    cryptofolio tx sell ETH 1.0 --account Binance --price 3200\n\n    # Record transfers between accounts\n    cryptofolio tx transfer BTC 0.5 --from Binance --to Ledger --fee 0.0001\n\n    # Record swaps\n    cryptofolio tx swap --from-asset ETH --from-quantity 1.0 --to-asset BTC --to-quantity 0.05 --account Binance\n\n    # Export transactions to CSV\n    cryptofolio tx export transactions.csv\n    cryptofolio tx export 2024-trades.csv --from 2024-01-01 --to 2024-12-31")]
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
    #[command(after_help = "EXAMPLES:\n    # View current configuration\n    cryptofolio config show\n    cryptofolio config show --json\n\n    # Set API credentials securely (recommended)\n    cryptofolio config set-secret binance.api_key\n    cryptofolio config set-secret binance.api_secret\n\n    # Set general configuration\n    cryptofolio config set display.color true\n    cryptofolio config use-testnet")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Manage currencies and exchange rates
    #[command(after_help = "EXAMPLES:\n    # List all currencies\n    cryptofolio currency list\n    cryptofolio currency list --enabled\n    cryptofolio currency list --json\n\n    # Add a new currency\n    cryptofolio currency add MXN --name \"Mexican Peso\" --symbol \"₱\" --decimals 2 --type fiat\n\n    # Show currency details\n    cryptofolio currency show USD\n\n    # Set exchange rate\n    cryptofolio currency set-rate CRC USD 550 --notes \"Bank rate\"\n    cryptofolio currency show-rate CRC USD\n    cryptofolio currency show-rate CRC USD --history")]
    Currency {
        #[command(subcommand)]
        command: CurrencyCommands,
    },

    /// Start interactive shell mode
    #[command(after_help = "EXAMPLES:\n    cryptofolio shell\n\nIn shell mode, you can:\n    - Run commands without typing 'cryptofolio' prefix\n    - Use Tab for auto-completion\n    - Use Up/Down for command history\n    - Type natural language (AI mode)")]
    Shell,

    /// Show system status and diagnostics
    ///
    /// Displays information about the current configuration, database location,
    /// AI provider status (Claude API, Ollama), and network mode (testnet/mainnet).
    /// Useful for troubleshooting connectivity issues or verifying setup.
    #[command(after_help = "EXAMPLES:\n    cryptofolio status\n    cryptofolio status --check\n\nThis command shows:\n    - Configuration file location\n    - Database file location\n    - Testnet/Mainnet mode\n    - Claude API connection status\n    - Ollama local LLM status\n    - Active AI provider")]
    Status {
        /// Run connectivity checks for AI providers
        #[arg(long)]
        check: bool,
    },
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
    Bank,
}

impl AccountTypeArg {
    pub fn to_string(&self) -> &'static str {
        match self {
            AccountTypeArg::Exchange => "exchange",
            AccountTypeArg::HardwareWallet => "hardware_wallet",
            AccountTypeArg::SoftwareWallet => "software_wallet",
            AccountTypeArg::CustodialService => "custodial_service",
            AccountTypeArg::Bank => "bank",
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

        /// Exchange rate (how many FROM per 1 TO) - for fiat swaps
        #[arg(long)]
        rate: Option<String>,

        /// Transaction notes
        #[arg(long)]
        notes: Option<String>,

        /// Simulate without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Export transactions to file
    #[command(after_help = "EXAMPLES:\n    # Export all transactions to CSV\n    cryptofolio tx export transactions.csv\n\n    # Export to JSON format\n    cryptofolio tx export transactions.json --format json\n\n    # Export to SQL format\n    cryptofolio tx export transactions.sql --format sql\n\n    # Export filtered transactions\n    cryptofolio tx export binance-2024.csv --account Binance\n    cryptofolio tx export btc-trades.json --asset BTC --format json\n\n    # Export with date range\n    cryptofolio tx export q1-2024.csv --from 2024-01-01 --to 2024-03-31\n\nFORMATS:\n    csv  - CSV format (default, compatible with import)\n    json - JSON array format\n    sql  - SQL INSERT statements")]
    Export {
        /// Output file path
        file: String,

        /// Export format (csv, json, sql)
        #[arg(long, default_value = "csv")]
        format: String,

        /// Filter by account name
        #[arg(long)]
        account: Option<String>,

        /// Filter by asset symbol
        #[arg(long)]
        asset: Option<String>,

        /// Start date (YYYY-MM-DD or ISO 8601)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD or ISO 8601)
        #[arg(long)]
        to: Option<String>,

        /// Maximum number of transactions (0 for unlimited)
        #[arg(long, default_value = "0")]
        limit: i64,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set a configuration value
    #[command(after_help = "EXAMPLES:\n    cryptofolio config set general.use_testnet true\n    cryptofolio config set display.color false\n    cryptofolio config set display.decimals 6\n    cryptofolio config set display.thousands_separator true\n\n⚠️  WARNING: For API keys/secrets, use 'config set-secret' instead!\n\nKEYS:\n    general.use_testnet          Enable testnet mode (true/false)\n    general.default_account       Default account name\n    display.color                 Enable colors (true/false)\n    display.decimals              Decimal places for quantities (0-18, default: 8)\n    display.price_decimals        Decimal places for prices (0-18, default: 2)\n    display.thousands_separator   Use thousands separator (true/false, default: true)")]
    Set {
        /// Configuration key (e.g., general.use_testnet)
        key: String,

        /// Configuration value
        value: String,
    },

    /// Set a secret configuration value securely
    ///
    /// SECURITY NOTICE (v0.3+):
    ///   On macOS: Secrets are stored in macOS Keychain with Touch ID protection
    ///   Other platforms: Secrets are stored in plaintext in ~/.config/cryptofolio/config.toml
    ///
    ///   IMPORTANT: Only use READ-ONLY API keys!
    ///   Never enable trading, withdrawal, or transfer permissions.
    ///
    /// BINANCE API KEY SETUP:
    ///   1. Go to Binance → API Management → Create API
    ///   2. Enable ONLY: "Enable Reading"
    ///   3. Disable: Trading, Withdrawals, Internal Transfer
    ///   4. IP restrictions recommended (optional but safer)
    #[command(name = "set-secret")]
    #[command(after_help = "EXAMPLES:\n    # Interactive (hidden input)\n    cryptofolio config set-secret binance.api_secret\n\n    # macOS: Store with Touch ID protection\n    cryptofolio config set-secret binance.api_secret --security-level touchid\n\n    # From stdin (for scripts)\n    echo \"secret\" | cryptofolio config set-secret binance.api_secret\n\n    # From file\n    cryptofolio config set-secret binance.api_secret --secret-file ~/.secrets/key\n\n    # From environment variable\n    cryptofolio config set-secret binance.api_secret --from-env MY_SECRET\n\nSECURITY LEVELS (macOS only):\n    standard          Protected by macOS encryption (good for automation)\n    touchid           Require Touch ID or password (recommended)\n    touchid-only      ONLY Touch ID, no password fallback (maximum security)")]
    SetSecret {
        /// Config key (e.g., binance.api_secret)
        key: String,

        /// Read secret from file instead of stdin/prompt
        #[arg(long)]
        secret_file: Option<std::path::PathBuf>,

        /// Read secret from environment variable
        #[arg(long)]
        from_env: Option<String>,

        /// Security level for keychain storage (macOS only): standard, touchid, touchid-only
        #[arg(long)]
        security_level: Option<String>,
    },

    /// Enable testnet mode
    #[command(name = "use-testnet")]
    UseTestnet,

    /// Disable testnet mode (use mainnet)
    #[command(name = "use-mainnet")]
    UseMainnet,

    /// Migrate secrets from TOML to macOS Keychain (macOS only)
    ///
    /// This command migrates API keys and secrets from plaintext storage
    /// in config.toml to encrypted macOS Keychain with optional Touch ID protection.
    ///
    /// Benefits:
    ///   - OS-level encryption (protected by your Mac login password)
    ///   - Touch ID authentication for each terminal session
    ///   - Protected from casual file access and backups
    ///   - Integration with macOS security features
    #[command(name = "migrate-to-keychain")]
    #[command(after_help = "EXAMPLES:\n    # Migrate all secrets to keychain\n    cryptofolio config migrate-to-keychain\n\n    # The wizard will:\n    #   1. Show all secrets found in config.toml\n    #   2. Let you choose security level (Standard, Touch ID, Touch ID Only)\n    #   3. Create a backup of config.toml\n    #   4. Migrate secrets to keychain\n    #   5. Clear secrets from config.toml\n\nSECURITY LEVELS:\n    Standard          Unlocked with Mac (good for automation)\n    Touch ID          Require Touch ID or password (recommended)\n    Touch ID Only     ONLY biometric, no password fallback")]
    MigrateToKeychain,

    /// Show keychain status and security levels (macOS only)
    ///
    /// Displays all secrets and their storage locations:
    ///   - Keychain (with security level)
    ///   - TOML config file
    ///   - Environment variables
    #[command(name = "keychain-status")]
    #[command(after_help = "EXAMPLES:\n    # Show all secrets and their locations\n    cryptofolio config keychain-status\n\n    # JSON output\n    cryptofolio config keychain-status --json\n\nOUTPUT:\n    Shows a table with:\n      - Key name (e.g., binance.api_secret)\n      - Storage type (keychain, toml, env)\n      - Security level (for keychain entries)\n      - Last accessed timestamp")]
    KeychainStatus,

    /// Upgrade security level for a keychain entry (macOS only)
    ///
    /// Increases the security level for an existing keychain secret.
    /// Requires authentication with current security level.
    #[command(name = "upgrade-security")]
    #[command(after_help = "EXAMPLES:\n    # Upgrade to Touch ID protection\n    cryptofolio config upgrade-security binance.api_secret --to touchid\n\n    # Upgrade to Touch ID only (maximum security)\n    cryptofolio config upgrade-security binance.api_secret --to touchid-only\n\nWARNING:\n    Touch ID Only mode has no password fallback.\n    You won't be able to access the secret in SSH sessions or without Touch ID.")]
    UpgradeSecurity {
        /// Secret key to upgrade (e.g., binance.api_secret)
        key: String,

        /// Target security level: touchid or touchid-only
        #[arg(long, value_parser = ["touchid", "touchid-only"])]
        to: String,
    },

    /// Downgrade security level for a keychain entry (macOS only)
    ///
    /// Decreases the security level for an existing keychain secret.
    /// Requires authentication with current security level.
    #[command(name = "downgrade-security")]
    #[command(after_help = "EXAMPLES:\n    # Downgrade to standard keychain\n    cryptofolio config downgrade-security binance.api_secret --to standard\n\n    # Downgrade to Touch ID Protected (from Touch ID Only)\n    cryptofolio config downgrade-security binance.api_secret --to touchid\n\nUSE CASES:\n    - Enable automation (scripts, cron jobs need Standard level)\n    - Resolve SSH access issues")]
    DowngradeSecurity {
        /// Secret key to downgrade (e.g., binance.api_secret)
        key: String,

        /// Target security level: standard or touchid
        #[arg(long, value_parser = ["standard", "touchid"])]
        to: String,
    },
}

#[derive(Subcommand)]
pub enum CurrencyCommands {
    /// List all currencies
    List {
        /// Show only enabled currencies
        #[arg(long)]
        enabled: bool,

        /// Filter by asset type (fiat, crypto, stablecoin)
        #[arg(long)]
        type_filter: Option<String>,
    },

    /// Show details for a currency
    Show {
        /// Currency code (e.g., USD, BTC)
        code: String,
    },

    /// Add a new currency
    Add {
        /// Currency code (e.g., MXN, JPY)
        code: String,

        /// Full name of the currency
        #[arg(long)]
        name: String,

        /// Symbol for display
        #[arg(long)]
        symbol: String,

        /// Number of decimal places
        #[arg(long, default_value = "2")]
        decimals: u8,

        /// Asset type: fiat, crypto, or stablecoin
        #[arg(long = "type", value_parser = ["fiat", "crypto", "stablecoin"])]
        type_name: String,
    },

    /// Remove a currency
    Remove {
        /// Currency code to remove
        code: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Enable or disable a currency
    Toggle {
        /// Currency code
        code: String,

        /// Enable the currency
        #[arg(long, conflicts_with = "disable")]
        enable: bool,

        /// Disable the currency
        #[arg(long, conflicts_with = "enable")]
        disable: bool,
    },

    /// Set exchange rate between two currencies
    #[command(name = "set-rate")]
    SetRate {
        /// From currency (e.g., CRC)
        from: String,

        /// To currency (e.g., USD)
        to: String,

        /// Exchange rate (how many FROM per 1 TO)
        rate: String,

        /// Optional notes
        #[arg(long)]
        notes: Option<String>,
    },

    /// Show exchange rate between two currencies
    #[command(name = "show-rate")]
    ShowRate {
        /// From currency (e.g., CRC)
        from: String,

        /// To currency (e.g., USD)
        to: String,

        /// Show all historical rates
        #[arg(long)]
        history: bool,
    },
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
