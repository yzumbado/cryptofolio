#![allow(dead_code)]

use colored::Colorize;
use is_terminal::IsTerminal;
use rust_decimal::Decimal;
use std::io::stdout;
use std::sync::OnceLock;

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

    matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
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
