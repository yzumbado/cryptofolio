use std::collections::HashMap;
use strsim::jaro_winkler;

/// Command aliases and shortcuts
pub fn get_aliases() -> HashMap<&'static str, &'static str> {
    let mut aliases = HashMap::new();

    // Single letter shortcuts
    aliases.insert("p", "portfolio");
    aliases.insert("h", "holdings");
    aliases.insert("a", "account");
    aliases.insert("c", "config");
    aliases.insert("s", "sync");
    aliases.insert("m", "market");
    aliases.insert("t", "tx");
    aliases.insert("i", "import");

    // Common abbreviations
    aliases.insert("bal", "portfolio");
    aliases.insert("balance", "portfolio");
    aliases.insert("hold", "holdings");
    aliases.insert("acc", "account");
    aliases.insert("accounts", "account list");
    aliases.insert("acct", "account");
    aliases.insert("cfg", "config");
    aliases.insert("conf", "config");
    aliases.insert("pr", "price");
    aliases.insert("mkt", "market");
    aliases.insert("tx", "tx");
    aliases.insert("buy", "tx buy");
    aliases.insert("sell", "tx sell");
    aliases.insert("transfer", "tx transfer");
    aliases.insert("xfer", "tx transfer");
    aliases.insert("swap", "tx swap");
    aliases.insert("ls", "holdings list");
    aliases.insert("list", "holdings list");

    // Action shortcuts
    aliases.insert("add", "holdings add");
    aliases.insert("rm", "holdings remove");
    aliases.insert("remove", "holdings remove");
    aliases.insert("mv", "holdings move");
    aliases.insert("move", "holdings move");

    aliases
}

/// Check if input looks like natural language rather than a command
fn is_natural_language(input: &str) -> bool {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.len() < 2 {
        return false;
    }

    let first_word = words[0].to_lowercase();
    let second_word = words[1].to_lowercase();

    // Common natural language starters
    let nl_starters = [
        "i", "what", "how", "show", "can", "please", "could", "would", "tell", "give", "do",
        "did", "is", "are", "was", "were", "have", "has", "had", "will", "should",
    ];

    // Verbs that follow "I" in natural language
    let verbs_after_i = [
        "bought", "sold", "want", "need", "have", "had", "got", "transferred", "moved",
        "swapped", "exchanged", "received", "sent", "added", "removed", "think", "would",
        "am", "just", "recently", "already", "also", "currently",
    ];

    // If first word is "I" (capital) followed by a verb, it's natural language
    if words[0] == "I" && verbs_after_i.iter().any(|v| second_word == *v) {
        return true;
    }

    // Question patterns
    if nl_starters.contains(&first_word.as_str()) {
        // Check for question-like patterns
        let question_words = ["what", "how", "can", "could", "would", "is", "are", "do", "did"];
        if question_words.contains(&first_word.as_str()) {
            return true;
        }
    }

    false
}

/// Expand shortcuts and aliases in the input
pub fn expand_shortcuts(input: &str) -> String {
    // Don't expand shortcuts for natural language input
    if is_natural_language(input) {
        return input.to_string();
    }

    let aliases = get_aliases();
    let words: Vec<&str> = input.split_whitespace().collect();

    if words.is_empty() {
        return input.to_string();
    }

    let first_word = words[0].to_lowercase();

    // Check for exact alias match
    if let Some(expansion) = aliases.get(first_word.as_str()) {
        let rest: Vec<&str> = words[1..].to_vec();
        if rest.is_empty() {
            return expansion.to_string();
        } else {
            return format!("{} {}", expansion, rest.join(" "));
        }
    }

    // No alias found, return original
    input.to_string()
}

/// Get all valid commands for fuzzy matching
pub fn get_all_commands() -> Vec<&'static str> {
    vec![
        "price",
        "market",
        "portfolio",
        "holdings",
        "holdings list",
        "holdings add",
        "holdings remove",
        "holdings set",
        "holdings move",
        "account",
        "account list",
        "account add",
        "account remove",
        "account show",
        "category",
        "category list",
        "category add",
        "tx",
        "tx list",
        "tx buy",
        "tx sell",
        "tx transfer",
        "tx swap",
        "sync",
        "import",
        "config",
        "config show",
        "config set",
        "status",
        "help",
        "clear",
        "exit",
    ]
}

/// Find similar commands using fuzzy matching
pub fn find_similar_commands(input: &str, threshold: f64) -> Vec<(&'static str, f64)> {
    let commands = get_all_commands();
    let input_lower = input.to_lowercase();

    let mut matches: Vec<(&str, f64)> = commands
        .into_iter()
        .map(|cmd| (cmd, jaro_winkler(&input_lower, cmd)))
        .filter(|(_, score)| *score >= threshold)
        .collect();

    matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    matches.truncate(3);
    matches
}

/// Suggest correction for a mistyped command
pub fn suggest_correction(input: &str) -> Option<String> {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    let first_word = words[0];
    let matches = find_similar_commands(first_word, 0.7);

    if matches.is_empty() {
        return None;
    }

    let (best_match, score) = matches[0];

    // Only suggest if it's a good match but not exact
    if score > 0.7 && score < 1.0 {
        let rest: Vec<&str> = words[1..].to_vec();
        if rest.is_empty() {
            Some(best_match.to_string())
        } else {
            Some(format!("{} {}", best_match, rest.join(" ")))
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_shortcuts() {
        assert_eq!(expand_shortcuts("p"), "portfolio");
        assert_eq!(expand_shortcuts("h list"), "holdings list");
        assert_eq!(expand_shortcuts("buy BTC 0.1"), "tx buy BTC 0.1");
        assert_eq!(expand_shortcuts("ls"), "holdings list");
    }

    #[test]
    fn test_natural_language_not_expanded() {
        // "I" should not expand to "import" when followed by verbs
        assert_eq!(
            expand_shortcuts("I bought some bitcoin today"),
            "I bought some bitcoin today"
        );
        assert_eq!(
            expand_shortcuts("I want to buy ETH"),
            "I want to buy ETH"
        );
        assert_eq!(
            expand_shortcuts("I have 0.5 BTC"),
            "I have 0.5 BTC"
        );

        // Questions should not be expanded
        assert_eq!(
            expand_shortcuts("What is the price of BTC?"),
            "What is the price of BTC?"
        );
        assert_eq!(
            expand_shortcuts("How much ETH do I have?"),
            "How much ETH do I have?"
        );

        // But "i" as a standalone command should still work
        assert_eq!(expand_shortcuts("i"), "import");
        assert_eq!(
            expand_shortcuts("i transactions.csv --account Test"),
            "import transactions.csv --account Test"
        );
    }

    #[test]
    fn test_find_similar() {
        let matches = find_similar_commands("protfolio", 0.7);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].0, "portfolio");
    }
}
