#![allow(dead_code)]

use colored::Colorize;
use is_terminal::IsTerminal;
use rust_decimal::Decimal;
use std::io::stdout;
use std::sync::OnceLock;

use crate::config::settings::DisplayConfig;

/// Global color configuration
static COLOR_ENABLED: OnceLock<bool> = OnceLock::new();

/// Initialize color settings based on environment and TTY
pub fn init_color(force_no_color: bool) {
    let enabled = if force_no_color {
        false
    } else if std::env::var("NO_COLOR").is_ok() {
        false
    } else if std::env::var("CRYPTOFOLIO_NO_COLOR").is_ok() {
        false
    } else if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        false
    } else {
        stdout().is_terminal()
    };

    let _ = COLOR_ENABLED.set(enabled);

    if !enabled {
        colored::control::set_override(false);
    }
}

/// Check if colors are enabled
pub fn colors_enabled() -> bool {
    *COLOR_ENABLED.get_or_init(|| {
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }
        stdout().is_terminal()
    })
}

/// Format a decimal with the specified number of decimal places
pub fn format_decimal(value: Decimal, decimals: u8) -> String {
    let scale = value.scale();
    if scale <= decimals as u32 {
        value.to_string()
    } else {
        format!("{:.prec$}", value, prec = decimals as usize)
    }
}

/// Format a USD amount
pub fn format_usd(value: Decimal) -> String {
    format!("${:.2}", value)
}

/// Format a USD amount with custom config
pub fn format_usd_with_config(value: Decimal, config: &DisplayConfig) -> String {
    let formatted = format!("{:.prec$}", value, prec = config.price_decimals as usize);
    let with_separator = if config.thousands_separator {
        add_thousands_separator(&formatted)
    } else {
        formatted
    };
    format!("${}", with_separator)
}

/// Format a quantity with appropriate decimals
pub fn format_quantity(value: Decimal) -> String {
    if value >= Decimal::from(1000) {
        format!("{:.2}", value)
    } else if value >= Decimal::from(1) {
        format!("{:.4}", value)
    } else {
        format!("{:.8}", value)
    }
}

/// Format a quantity with custom config
pub fn format_quantity_with_config(value: Decimal, config: &DisplayConfig) -> String {
    let formatted = format!("{:.prec$}", value, prec = config.decimals as usize);
    if config.thousands_separator {
        add_thousands_separator(&formatted)
    } else {
        formatted
    }
}

/// Add thousands separator to a formatted number string
fn add_thousands_separator(num_str: &str) -> String {
    // Split on decimal point
    let parts: Vec<&str> = num_str.split('.').collect();
    let integer_part = parts[0];

    // Add commas to integer part
    let mut result = String::new();
    let chars: Vec<char> = integer_part.chars().collect();
    let len = chars.len();

    for (i, ch) in chars.iter().enumerate() {
        result.push(*ch);
        let pos = len - i - 1;
        if pos > 0 && pos % 3 == 0 {
            result.push(',');
        }
    }

    // Add decimal part if exists
    if parts.len() > 1 {
        result.push('.');
        result.push_str(parts[1]);
    }

    result
}

/// Format a percentage
pub fn format_percent(value: Decimal) -> String {
    format!("{:.2}%", value)
}

/// Format a P&L value with color
pub fn format_pnl(value: Decimal, with_color: bool) -> String {
    let formatted = if value >= Decimal::ZERO {
        format!("+{}", format_usd(value))
    } else {
        format_usd(value)
    };

    if with_color && colors_enabled() {
        if value > Decimal::ZERO {
            formatted.green().to_string()
        } else if value < Decimal::ZERO {
            formatted.red().to_string()
        } else {
            formatted
        }
    } else {
        formatted
    }
}

/// Format a P&L value with color and custom config
pub fn format_pnl_with_config(value: Decimal, config: &DisplayConfig) -> String {
    let formatted = if value >= Decimal::ZERO {
        format!("+{}", format_usd_with_config(value, config))
    } else {
        format_usd_with_config(value, config)
    };

    if config.color && colors_enabled() {
        if value > Decimal::ZERO {
            formatted.green().to_string()
        } else if value < Decimal::ZERO {
            formatted.red().to_string()
        } else {
            formatted
        }
    } else {
        formatted
    }
}

/// Format a P&L percentage with color
pub fn format_pnl_percent(value: Decimal, with_color: bool) -> String {
    let formatted = if value >= Decimal::ZERO {
        format!("+{}", format_percent(value))
    } else {
        format_percent(value)
    };

    if with_color && colors_enabled() {
        if value > Decimal::ZERO {
            formatted.green().to_string()
        } else if value < Decimal::ZERO {
            formatted.red().to_string()
        } else {
            formatted
        }
    } else {
        formatted
    }
}

/// Format a price change with color
pub fn format_price_change(value: Decimal, percent: Decimal, with_color: bool) -> String {
    let sign = if value >= Decimal::ZERO { "+" } else { "" };
    let formatted = format!("{}{} ({}{}%)", sign, format_usd(value.abs()), sign, format!("{:.2}", percent));

    if with_color && colors_enabled() {
        if value > Decimal::ZERO {
            formatted.green().to_string()
        } else if value < Decimal::ZERO {
            formatted.red().to_string()
        } else {
            formatted
        }
    } else {
        formatted
    }
}

/// Print a success message
pub fn success(message: &str) {
    if colors_enabled() {
        println!("{} {}", "✓".green(), message);
    } else {
        println!("[OK] {}", message);
    }
}

/// Print an error message
pub fn error(message: &str) {
    if colors_enabled() {
        eprintln!("{} {}", "✗".red(), message);
    } else {
        eprintln!("[ERROR] {}", message);
    }
}

/// Print a warning message
pub fn warning(message: &str) {
    if colors_enabled() {
        println!("{} {}", "!".yellow(), message);
    } else {
        println!("[WARN] {}", message);
    }
}

/// Print an info message
pub fn info(message: &str) {
    if colors_enabled() {
        println!("{} {}", "i".blue(), message);
    } else {
        println!("[INFO] {}", message);
    }
}

/// Print a table header
pub fn print_header(columns: &[(&str, usize)]) {
    let header: String = columns
        .iter()
        .map(|(name, width)| format!("{:width$}", name, width = width))
        .collect::<Vec<_>>()
        .join("  ");

    if colors_enabled() {
        println!("{}", header.bold());
    } else {
        println!("{}", header);
    }
    println!("{}", "-".repeat(header.len()));
}

/// Print a table row
pub fn print_row(values: &[(&str, usize)]) {
    let row: String = values
        .iter()
        .map(|(value, width)| format!("{:width$}", value, width = width))
        .collect::<Vec<_>>()
        .join("  ");

    println!("{}", row);
}

/// Print a simple key-value pair
pub fn print_kv(key: &str, value: &str) {
    if colors_enabled() {
        println!("  {}: {}", key.dimmed(), value);
    } else {
        println!("  {}: {}", key, value);
    }
}

/// Print a section title
pub fn print_section(title: &str) {
    println!();
    if colors_enabled() {
        println!("{}", title.bold().underline());
    } else {
        println!("{}", title);
        println!("{}", "=".repeat(title.len()));
    }
    println!();
}

/// Suggest next command to run
pub fn suggest_next(command: &str, description: &str) {
    println!();
    if colors_enabled() {
        println!("{} {}", "Next:".dimmed(), description);
        println!("  {}", command.cyan());
    } else {
        println!("Next: {}", description);
        println!("  {}", command);
    }
}

/// Find similar strings for "did you mean?" suggestions
pub fn find_similar<'a>(input: &str, candidates: &[&'a str], threshold: f64) -> Vec<&'a str> {
    use strsim::jaro_winkler;

    let input_lower = input.to_lowercase();
    let mut matches: Vec<(&str, f64)> = candidates
        .iter()
        .map(|&c| (c, jaro_winkler(&input_lower, &c.to_lowercase())))
        .filter(|(_, score)| *score >= threshold)
        .collect();

    // Sort by score descending, treating NaN as lowest priority
    matches.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches.into_iter().take(3).map(|(s, _)| s).collect()
}

/// Print "did you mean?" suggestions
pub fn print_did_you_mean(suggestions: &[&str]) {
    if suggestions.is_empty() {
        return;
    }

    println!();
    if suggestions.len() == 1 {
        if colors_enabled() {
            println!("Did you mean {}?", suggestions[0].yellow());
        } else {
            println!("Did you mean '{}'?", suggestions[0]);
        }
    } else {
        println!("Did you mean one of these?");
        for s in suggestions {
            if colors_enabled() {
                println!("  - {}", s.yellow());
            } else {
                println!("  - {}", s);
            }
        }
    }
}

/// Print data as JSON
pub fn print_json<T: serde::Serialize>(data: &T) -> crate::error::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Disable colors for testing to get predictable output
    fn setup() {
        colored::control::set_override(false);
    }

    #[test]
    fn test_format_decimal_no_rounding() {
        setup();
        let value = Decimal::from_str("123.45").unwrap();
        assert_eq!(format_decimal(value, 2), "123.45");
        assert_eq!(format_decimal(value, 3), "123.45");
        assert_eq!(format_decimal(value, 4), "123.45");
    }

    #[test]
    fn test_format_decimal_with_rounding() {
        setup();
        let value = Decimal::from_str("123.456789").unwrap();
        assert_eq!(format_decimal(value, 2), "123.45");
        assert_eq!(format_decimal(value, 4), "123.4567");
        assert_eq!(format_decimal(value, 6), "123.456789");
    }

    #[test]
    fn test_format_usd() {
        setup();
        assert_eq!(format_usd(Decimal::from_str("1234.56").unwrap()), "$1234.56");
        assert_eq!(format_usd(Decimal::from_str("0.99").unwrap()), "$0.99");
        assert_eq!(format_usd(Decimal::from_str("1000000").unwrap()), "$1000000.00");
    }

    #[test]
    fn test_format_usd_negative() {
        setup();
        assert_eq!(format_usd(Decimal::from_str("-123.45").unwrap()), "$-123.45");
    }

    #[test]
    fn test_format_quantity_large() {
        setup();
        // >= 1000: 2 decimals (truncated, not rounded)
        let value = Decimal::from_str("1234.567").unwrap();
        assert_eq!(format_quantity(value), "1234.56");
    }

    #[test]
    fn test_format_quantity_medium() {
        setup();
        // >= 1 and < 1000: 4 decimals (truncated, not rounded)
        let value = Decimal::from_str("12.3456789").unwrap();
        assert_eq!(format_quantity(value), "12.3456");
    }

    #[test]
    fn test_format_quantity_small() {
        setup();
        // < 1: 8 decimals (truncated, not rounded)
        let value = Decimal::from_str("0.123456789").unwrap();
        assert_eq!(format_quantity(value), "0.12345678");
    }

    #[test]
    fn test_add_thousands_separator_simple() {
        assert_eq!(add_thousands_separator("1234567"), "1,234,567");
        assert_eq!(add_thousands_separator("1234"), "1,234");
        assert_eq!(add_thousands_separator("123"), "123");
    }

    #[test]
    fn test_add_thousands_separator_with_decimals() {
        assert_eq!(add_thousands_separator("1234567.89"), "1,234,567.89");
        assert_eq!(add_thousands_separator("1234.5678"), "1,234.5678");
    }

    #[test]
    fn test_format_percent() {
        setup();
        assert_eq!(format_percent(Decimal::from_str("12.345").unwrap()), "12.34%");
        assert_eq!(format_percent(Decimal::from_str("0.5").unwrap()), "0.50%");
        assert_eq!(format_percent(Decimal::from_str("-5.67").unwrap()), "-5.67%");
    }

    #[test]
    fn test_format_pnl_positive() {
        setup();
        let value = Decimal::from_str("123.45").unwrap();
        assert_eq!(format_pnl(value, false), "+$123.45");
    }

    #[test]
    fn test_format_pnl_negative() {
        setup();
        let value = Decimal::from_str("-123.45").unwrap();
        assert_eq!(format_pnl(value, false), "$-123.45");
    }

    #[test]
    fn test_format_pnl_zero() {
        setup();
        let value = Decimal::ZERO;
        // Zero gets + sign because it's >= 0
        assert_eq!(format_pnl(value, false), "+$0.00");
    }

    #[test]
    fn test_format_pnl_percent_positive() {
        setup();
        let value = Decimal::from_str("15.5").unwrap();
        assert_eq!(format_pnl_percent(value, false), "+15.50%");
    }

    #[test]
    fn test_format_pnl_percent_negative() {
        setup();
        let value = Decimal::from_str("-8.25").unwrap();
        assert_eq!(format_pnl_percent(value, false), "-8.25%");
    }

    #[test]
    fn test_format_pnl_percent_zero() {
        setup();
        let value = Decimal::ZERO;
        assert_eq!(format_pnl_percent(value, false), "+0.00%");
    }

    #[test]
    fn test_format_price_change_positive() {
        setup();
        let value = Decimal::from_str("50.00").unwrap();
        let percent = Decimal::from_str("5.5").unwrap();
        assert_eq!(format_price_change(value, percent, false), "+$50.00 (+5.50%)");
    }

    #[test]
    fn test_format_price_change_negative() {
        setup();
        let value = Decimal::from_str("-30.00").unwrap();
        let percent = Decimal::from_str("-3.0").unwrap();
        assert_eq!(format_price_change(value, percent, false), "$30.00 (-3.00%)");
    }

    #[test]
    fn test_format_price_change_zero() {
        setup();
        let value = Decimal::ZERO;
        let percent = Decimal::ZERO;
        // Zero gets + sign because it's >= 0
        assert_eq!(format_price_change(value, percent, false), "+$0.00 (+0.00%)");
    }

    #[test]
    fn test_find_similar_exact_match() {
        let candidates = vec!["account", "portfolio", "balance"];
        let similar = find_similar("account", &candidates, 0.8);
        assert_eq!(similar, vec!["account"]);
    }

    #[test]
    fn test_find_similar_close_match() {
        let candidates = vec!["account", "portfolio", "balance"];
        let similar = find_similar("acount", &candidates, 0.8);
        assert!(!similar.is_empty());
        assert_eq!(similar[0], "account");
    }

    #[test]
    fn test_find_similar_no_match() {
        let candidates = vec!["account", "portfolio", "balance"];
        let similar = find_similar("xyz", &candidates, 0.8);
        assert!(similar.is_empty());
    }

    #[test]
    fn test_find_similar_multiple_matches() {
        let candidates = vec!["portfolio", "port", "export", "import"];
        let similar = find_similar("port", &candidates, 0.6);
        assert!(!similar.is_empty());
        // Should have "port" and "portfolio" as close matches
        assert!(similar.contains(&"port"));
        assert!(similar.contains(&"portfolio"));
    }

    #[test]
    fn test_find_similar_case_insensitive() {
        let candidates = vec!["Account", "Portfolio", "Balance"];
        let similar = find_similar("account", &candidates, 0.8);
        assert_eq!(similar[0], "Account");
    }

    #[test]
    fn test_find_similar_max_three_results() {
        let candidates = vec!["test1", "test2", "test3", "test4", "test5"];
        let similar = find_similar("test", &candidates, 0.6);
        assert!(similar.len() <= 3);
    }

    #[test]
    fn test_format_usd_with_config_thousands_separator() {
        setup();
        let config = DisplayConfig {
            color: false,
            price_decimals: 2,
            decimals: 8,
            thousands_separator: true,
        };
        let value = Decimal::from_str("1234567.89").unwrap();
        assert_eq!(format_usd_with_config(value, &config), "$1,234,567.89");
    }

    #[test]
    fn test_format_usd_with_config_no_separator() {
        setup();
        let config = DisplayConfig {
            color: false,
            price_decimals: 2,
            decimals: 8,
            thousands_separator: false,
        };
        let value = Decimal::from_str("1234567.89").unwrap();
        assert_eq!(format_usd_with_config(value, &config), "$1234567.89");
    }

    #[test]
    fn test_format_quantity_with_config() {
        setup();
        let config = DisplayConfig {
            color: false,
            price_decimals: 2,
            decimals: 4,
            thousands_separator: true,
        };
        let value = Decimal::from_str("1234.56789").unwrap();
        // Truncated, not rounded
        assert_eq!(format_quantity_with_config(value, &config), "1,234.5678");
    }

    #[test]
    fn test_format_pnl_with_config_positive() {
        setup();
        let config = DisplayConfig {
            color: false,
            price_decimals: 2,
            decimals: 8,
            thousands_separator: true,
        };
        let value = Decimal::from_str("1234.56").unwrap();
        assert_eq!(format_pnl_with_config(value, &config), "+$1,234.56");
    }

    #[test]
    fn test_format_pnl_with_config_negative() {
        setup();
        let config = DisplayConfig {
            color: false,
            price_decimals: 2,
            decimals: 8,
            thousands_separator: false,
        };
        let value = Decimal::from_str("-1234.56").unwrap();
        assert_eq!(format_pnl_with_config(value, &config), "$-1234.56");
    }
}
