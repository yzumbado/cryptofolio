use std::collections::HashMap;

/// Shell context for maintaining state across commands
#[derive(Debug, Default)]
pub struct ShellContext {
    /// Last used account name
    pub last_account: Option<String>,

    /// Last used asset symbol
    pub last_asset: Option<String>,

    /// Last used price (for quick reference)
    pub last_price: Option<String>,

    /// Custom variables set by user
    pub variables: HashMap<String, String>,
}

impl ShellContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update context from a command that was just executed
    pub fn update_from_command(&mut self, args: &[String]) {
        // Look for --account flag
        for (i, arg) in args.iter().enumerate() {
            if arg == "--account" || arg == "--from" || arg == "--to" {
                if let Some(account) = args.get(i + 1) {
                    self.last_account = Some(account.trim_matches('"').to_string());
                }
            }
        }

        // Look for asset symbols (usually first positional arg after subcommand)
        // Common patterns: "price BTC", "tx buy BTC", "holdings add BTC"
        let asset_position = if args.len() > 2 {
            match args.get(1).map(|s| s.as_str()) {
                Some("price") | Some("market") => Some(2),
                Some("tx") if args.len() > 3 => Some(3),
                Some("holdings") if args.len() > 3 => Some(3),
                _ => None,
            }
        } else {
            None
        };

        if let Some(pos) = asset_position {
            if let Some(asset) = args.get(pos) {
                // Check if it looks like an asset symbol (uppercase, 2-5 chars)
                if asset.chars().all(|c| c.is_ascii_uppercase())
                    && asset.len() >= 2
                    && asset.len() <= 5
                {
                    self.last_asset = Some(asset.clone());
                }
            }
        }
    }

    /// Apply context defaults to a command
    /// Returns the modified arguments with defaults filled in
    pub fn apply_defaults(&self, args: &[String]) -> Vec<String> {
        let mut result = args.to_vec();

        // Check if --account is missing but required
        let needs_account = args.iter().any(|a| {
            matches!(
                a.as_str(),
                "add" | "remove" | "set" | "buy" | "sell"
            )
        });

        let has_account = args.iter().any(|a| a == "--account");

        if needs_account && !has_account {
            if let Some(ref account) = self.last_account {
                result.push("--account".to_string());
                result.push(format!("\"{}\"", account));
            }
        }

        result
    }

    /// Get context summary for display
    pub fn summary(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(ref account) = self.last_account {
            parts.push(format!("account: {}", account));
        }
        if let Some(ref asset) = self.last_asset {
            parts.push(format!("asset: {}", asset));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_from_command() {
        let mut ctx = ShellContext::new();

        let args = vec![
            "cryptofolio".to_string(),
            "tx".to_string(),
            "buy".to_string(),
            "BTC".to_string(),
            "0.1".to_string(),
            "--account".to_string(),
            "Binance".to_string(),
        ];

        ctx.update_from_command(&args);

        assert_eq!(ctx.last_account, Some("Binance".to_string()));
        assert_eq!(ctx.last_asset, Some("BTC".to_string()));
    }
}
