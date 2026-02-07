use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use sqlx::SqlitePool;

use crate::db::AccountRepository;
use crate::error::Result;

/// Tab completion for cryptofolio commands
pub struct CryptofolioCompleter {
    commands: Vec<String>,
    subcommands: std::collections::HashMap<String, Vec<String>>,
    accounts: Vec<String>,
    assets: Vec<String>,
}

impl CryptofolioCompleter {
    pub async fn new(pool: &SqlitePool) -> Result<Self> {
        // Main commands
        let commands = vec![
            "price".to_string(),
            "market".to_string(),
            "portfolio".to_string(),
            "holdings".to_string(),
            "account".to_string(),
            "category".to_string(),
            "tx".to_string(),
            "sync".to_string(),
            "import".to_string(),
            "config".to_string(),
            "help".to_string(),
            "clear".to_string(),
            "exit".to_string(),
        ];

        // Subcommands for each main command
        let mut subcommands = std::collections::HashMap::new();
        subcommands.insert(
            "holdings".to_string(),
            vec![
                "list".to_string(),
                "add".to_string(),
                "remove".to_string(),
                "set".to_string(),
                "move".to_string(),
            ],
        );
        subcommands.insert(
            "account".to_string(),
            vec![
                "list".to_string(),
                "add".to_string(),
                "remove".to_string(),
                "show".to_string(),
                "address".to_string(),
            ],
        );
        subcommands.insert(
            "category".to_string(),
            vec![
                "list".to_string(),
                "add".to_string(),
                "remove".to_string(),
                "rename".to_string(),
            ],
        );
        subcommands.insert(
            "tx".to_string(),
            vec![
                "list".to_string(),
                "buy".to_string(),
                "sell".to_string(),
                "transfer".to_string(),
                "swap".to_string(),
            ],
        );
        subcommands.insert(
            "config".to_string(),
            vec![
                "show".to_string(),
                "set".to_string(),
                "use-testnet".to_string(),
                "use-mainnet".to_string(),
            ],
        );
        subcommands.insert(
            "portfolio".to_string(),
            vec![
                "--by-account".to_string(),
                "--by-category".to_string(),
                "--json".to_string(),
            ],
        );

        // Fetch accounts from database
        let account_repo = AccountRepository::new(pool);
        let accounts = account_repo
            .list_accounts()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.name)
            .collect();

        // Common cryptocurrency symbols
        let assets = vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "SOL".to_string(),
            "BNB".to_string(),
            "XRP".to_string(),
            "ADA".to_string(),
            "DOGE".to_string(),
            "DOT".to_string(),
            "MATIC".to_string(),
            "LINK".to_string(),
            "AVAX".to_string(),
            "UNI".to_string(),
            "ATOM".to_string(),
            "LTC".to_string(),
            "USDT".to_string(),
            "USDC".to_string(),
        ];

        Ok(Self {
            commands,
            subcommands,
            accounts,
            assets,
        })
    }

    fn complete_command(&self, line: &str, pos: usize) -> Vec<Pair> {
        let mut completions = Vec::new();
        let words: Vec<&str> = line[..pos].split_whitespace().collect();

        match words.len() {
            0 => {
                // Complete main commands
                for cmd in &self.commands {
                    completions.push(Pair {
                        display: cmd.clone(),
                        replacement: cmd.clone(),
                    });
                }
            }
            1 => {
                let partial = words[0].to_lowercase();

                // Check if the first word is complete
                if line.ends_with(' ') {
                    // First word is complete, suggest subcommands
                    if let Some(subs) = self.subcommands.get(&partial) {
                        for sub in subs {
                            completions.push(Pair {
                                display: sub.clone(),
                                replacement: sub.clone(),
                            });
                        }
                    }
                    // For price/market, suggest assets
                    if partial == "price" || partial == "market" {
                        for asset in &self.assets {
                            completions.push(Pair {
                                display: asset.clone(),
                                replacement: asset.clone(),
                            });
                        }
                    }
                } else {
                    // Still typing first word, complete commands
                    for cmd in &self.commands {
                        if cmd.starts_with(&partial) {
                            completions.push(Pair {
                                display: cmd.clone(),
                                replacement: cmd.clone(),
                            });
                        }
                    }
                }
            }
            _ => {
                let cmd = words[0].to_lowercase();
                let partial = words.last().unwrap_or(&"").to_lowercase();
                let is_complete_word = line.ends_with(' ');

                // Suggest based on context
                if is_complete_word || partial.is_empty() {
                    // Suggest based on previous words
                    if words.iter().any(|w| *w == "--account" || *w == "--from" || *w == "--to") {
                        // Suggest accounts
                        for account in &self.accounts {
                            completions.push(Pair {
                                display: account.clone(),
                                replacement: format!("\"{}\"", account),
                            });
                        }
                    } else if cmd == "price" || cmd == "market" {
                        // Suggest assets
                        for asset in &self.assets {
                            completions.push(Pair {
                                display: asset.clone(),
                                replacement: asset.clone(),
                            });
                        }
                    } else if cmd == "holdings" && words.get(1).map(|s| *s == "add" || *s == "remove" || *s == "set").unwrap_or(false) {
                        // Suggest assets for holdings commands
                        for asset in &self.assets {
                            completions.push(Pair {
                                display: asset.clone(),
                                replacement: asset.clone(),
                            });
                        }
                    }
                } else {
                    // Partial word completion
                    let partial_upper = partial.to_uppercase();

                    // Try completing as asset
                    for asset in &self.assets {
                        if asset.starts_with(&partial_upper) {
                            completions.push(Pair {
                                display: asset.clone(),
                                replacement: asset.clone(),
                            });
                        }
                    }

                    // Try completing as flag
                    if partial.starts_with('-') {
                        let flags = vec![
                            "--account",
                            "--from",
                            "--to",
                            "--price",
                            "--cost",
                            "--json",
                            "--quiet",
                            "--yes",
                            "--dry-run",
                        ];
                        for flag in flags {
                            if flag.starts_with(&partial) {
                                completions.push(Pair {
                                    display: flag.to_string(),
                                    replacement: flag.to_string(),
                                });
                            }
                        }
                    }

                    // Try completing as account
                    for account in &self.accounts {
                        if account.to_lowercase().starts_with(&partial) {
                            completions.push(Pair {
                                display: account.clone(),
                                replacement: format!("\"{}\"", account),
                            });
                        }
                    }
                }
            }
        }

        completions
    }
}

impl Completer for CryptofolioCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let completions = self.complete_command(line, pos);

        // Find the start of the word being completed
        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);

        Ok((start, completions))
    }
}

impl Hinter for CryptofolioCompleter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }

        let completions = self.complete_command(line, pos);

        // Show first completion as hint
        if completions.len() == 1 {
            let hint = &completions[0].replacement;
            let partial = line.split_whitespace().last().unwrap_or("");

            if hint.to_lowercase().starts_with(&partial.to_lowercase()) && hint.len() > partial.len() {
                return Some(hint[partial.len()..].to_string());
            }
        }

        None
    }
}

impl Highlighter for CryptofolioCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        // Could add syntax highlighting here
        std::borrow::Cow::Borrowed(line)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> std::borrow::Cow<'b, str> {
        std::borrow::Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        // Dim the hint text
        std::borrow::Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        false
    }
}

impl Validator for CryptofolioCompleter {}

impl Helper for CryptofolioCompleter {}
