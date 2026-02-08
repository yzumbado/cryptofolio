//! Status command - displays system diagnostics and configuration
//!
//! Shows information about:
//! - Configuration file location and contents
//! - Database file location
//! - Network mode (testnet/mainnet)
//! - AI provider status (Claude API, Ollama)
//! - Active AI mode and effective provider

use colored::Colorize;
use reqwest::Client;
use std::time::Duration;

use crate::cli::notifications::{ProviderStatus, SystemStatus};
use crate::cli::output::colors_enabled;
use crate::config::AppConfig;
use crate::error::Result;

/// Run the status command
pub async fn run(check: bool) -> Result<()> {
    let status = collect_status(check).await?;
    println!("{}", status.format());
    Ok(())
}

/// Collect system status information
pub async fn collect_status(run_checks: bool) -> Result<SystemStatus> {
    let config = AppConfig::load().ok();

    // Get paths
    let config_path = AppConfig::config_path()
        .ok()
        .map(|p| p.display().to_string());
    let db_path = AppConfig::database_path()
        .ok()
        .map(|p| p.display().to_string());

    // Check testnet mode
    let testnet_mode = config
        .as_ref()
        .map(|c| c.general.use_testnet)
        .unwrap_or(false);

    // AI configuration
    let ai_config = config.as_ref().and_then(|c| c.ai.as_ref());

    let ai_mode = ai_config
        .and_then(|ai| ai.mode.clone())
        .unwrap_or_else(|| "hybrid".to_string());

    // Check Claude status
    let claude_status = check_claude_status(&config, run_checks).await;

    // Check Ollama status
    let ollama_status = check_ollama_status(&config, run_checks).await;

    // Determine effective provider
    let effective_provider = determine_effective_provider(&ai_mode, &claude_status, &ollama_status);

    Ok(SystemStatus {
        config_path,
        db_path,
        testnet_mode,
        claude_status,
        ollama_status,
        ai_mode: format_ai_mode(&ai_mode),
        effective_provider,
    })
}

/// Check Claude API status
async fn check_claude_status(config: &Option<AppConfig>, run_checks: bool) -> ProviderStatus {
    // Check if API key is configured
    let api_key = config
        .as_ref()
        .and_then(|c| c.ai.as_ref())
        .and_then(|ai| ai.claude_api_key.clone())
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());

    let model = config
        .as_ref()
        .and_then(|c| c.ai.as_ref())
        .and_then(|ai| ai.claude_model.clone())
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    match api_key {
        None => ProviderStatus::unavailable("Claude", "API key not configured"),
        Some(key) => {
            if run_checks {
                // Actually test the API connection
                match test_claude_connection(&key).await {
                    Ok(()) => ProviderStatus::available("Claude", model),
                    Err(e) => ProviderStatus::unavailable("Claude", e),
                }
            } else {
                // Just check if key is present
                if key.starts_with("sk-") {
                    ProviderStatus::available("Claude", model)
                } else {
                    ProviderStatus::unavailable("Claude", "Invalid API key format")
                }
            }
        }
    }
}

/// Test Claude API connection
async fn test_claude_connection(api_key: &str) -> std::result::Result<(), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    // Use a minimal request to test authentication
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-sonnet-4-20250514","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let status = response.status();
    if status.is_success() || status.as_u16() == 400 {
        // 400 is OK - means auth worked but request was minimal
        Ok(())
    } else if status.as_u16() == 401 {
        Err("Invalid API key".to_string())
    } else if status.as_u16() == 429 {
        Err("Rate limited".to_string())
    } else {
        Err(format!("HTTP {}", status))
    }
}

/// Check Ollama status
async fn check_ollama_status(config: &Option<AppConfig>, run_checks: bool) -> ProviderStatus {
    let base_url = config
        .as_ref()
        .and_then(|c| c.ai.as_ref())
        .and_then(|ai| ai.ollama_url.clone())
        .or_else(|| std::env::var("OLLAMA_HOST").ok())
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    let model = config
        .as_ref()
        .and_then(|c| c.ai.as_ref())
        .and_then(|ai| ai.local_model.clone())
        .unwrap_or_else(|| "llama3.2:3b".to_string());

    if run_checks || true {
        // Always check Ollama since it's local
        match test_ollama_connection(&base_url).await {
            Ok(()) => ProviderStatus::available("Ollama", model),
            Err(e) => ProviderStatus::unavailable("Ollama", e),
        }
    } else {
        ProviderStatus::unavailable("Ollama", "Not checked")
    }
}

/// Test Ollama connection
async fn test_ollama_connection(base_url: &str) -> std::result::Result<(), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/tags", base_url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                "Not running".to_string()
            } else if e.is_timeout() {
                "Timeout".to_string()
            } else {
                format!("Connection failed: {}", e)
            }
        })?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}

/// Determine which provider will actually be used
fn determine_effective_provider(
    mode: &str,
    claude: &ProviderStatus,
    ollama: &ProviderStatus,
) -> String {
    match mode.to_lowercase().as_str() {
        "disabled" | "off" | "none" => "Disabled".to_string(),
        "online" | "claude" => {
            if claude.available {
                format!("Claude ({})", claude.model.as_deref().unwrap_or("?"))
            } else {
                "None (Claude unavailable)".to_string()
            }
        }
        "offline" | "local" | "ollama" => {
            if ollama.available {
                format!("Ollama ({})", ollama.model.as_deref().unwrap_or("?"))
            } else {
                "Pattern-based (Ollama unavailable)".to_string()
            }
        }
        "hybrid" | "auto" | _ => {
            if claude.available && ollama.available {
                "Hybrid (Ollama + Claude)".to_string()
            } else if claude.available {
                format!("Claude only ({})", claude.model.as_deref().unwrap_or("?"))
            } else if ollama.available {
                format!("Ollama only ({})", ollama.model.as_deref().unwrap_or("?"))
            } else {
                "Pattern-based (no LLM available)".to_string()
            }
        }
    }
}

/// Format AI mode for display
fn format_ai_mode(mode: &str) -> String {
    match mode.to_lowercase().as_str() {
        "online" | "claude" => "Online (Claude API)".to_string(),
        "offline" | "local" | "ollama" => "Offline (Ollama)".to_string(),
        "hybrid" | "auto" => "Hybrid (Local + Cloud)".to_string(),
        "disabled" | "off" | "none" => "Disabled".to_string(),
        _ => format!("Unknown ({})", mode),
    }
}

/// Print a compact status summary (for shell startup)
pub async fn print_startup_summary() {
    let status = match collect_status(true).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  {} Failed to collect status: {}", "⚠".yellow(), e);
            return;
        }
    };

    // Print compact AI status
    let ai_status = if status.claude_status.available && status.ollama_status.available {
        if colors_enabled() {
            format!("🤖 {} (Cloud + Local)", "AI Ready".green())
        } else {
            "🤖 AI Ready (Cloud + Local)".to_string()
        }
    } else if status.claude_status.available {
        if colors_enabled() {
            format!("☁️ {} (Claude)", "AI Ready".green())
        } else {
            "☁️ AI Ready (Claude)".to_string()
        }
    } else if status.ollama_status.available {
        if colors_enabled() {
            format!("🦙 {} (Ollama)", "AI Ready".green())
        } else {
            "🦙 AI Ready (Ollama)".to_string()
        }
    } else {
        if colors_enabled() {
            format!("📝 {} (pattern matching)", "Basic Mode".yellow())
        } else {
            "📝 Basic Mode (pattern matching)".to_string()
        }
    };

    let mode_status = if status.testnet_mode {
        if colors_enabled() {
            format!("🧪 {}", "Testnet".cyan())
        } else {
            "🧪 Testnet".to_string()
        }
    } else {
        if colors_enabled() {
            format!("🌐 {}", "Mainnet".green())
        } else {
            "🌐 Mainnet".to_string()
        }
    };

    println!("  {}  •  {}", mode_status, ai_status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ai_mode() {
        assert_eq!(format_ai_mode("hybrid"), "Hybrid (Local + Cloud)");
        assert_eq!(format_ai_mode("online"), "Online (Claude API)");
        assert_eq!(format_ai_mode("offline"), "Offline (Ollama)");
        assert_eq!(format_ai_mode("disabled"), "Disabled");
    }

    #[test]
    fn test_format_ai_mode_aliases() {
        // Test various aliases
        assert_eq!(format_ai_mode("auto"), "Hybrid (Local + Cloud)");
        assert_eq!(format_ai_mode("claude"), "Online (Claude API)");
        assert_eq!(format_ai_mode("local"), "Offline (Ollama)");
        assert_eq!(format_ai_mode("ollama"), "Offline (Ollama)");
        assert_eq!(format_ai_mode("off"), "Disabled");
        assert_eq!(format_ai_mode("none"), "Disabled");
    }

    #[test]
    fn test_format_ai_mode_unknown() {
        let result = format_ai_mode("foobar");
        assert!(result.contains("Unknown"));
        assert!(result.contains("foobar"));
    }

    #[test]
    fn test_determine_effective_provider() {
        let claude_ok = ProviderStatus::available("Claude", "claude-sonnet-4-20250514".to_string());
        let claude_no = ProviderStatus::unavailable("Claude", "No key");
        let ollama_ok = ProviderStatus::available("Ollama", "llama3.2:3b".to_string());
        let ollama_no = ProviderStatus::unavailable("Ollama", "Not running");

        // Hybrid mode tests
        assert!(determine_effective_provider("hybrid", &claude_ok, &ollama_ok).contains("Hybrid"));
        assert!(determine_effective_provider("hybrid", &claude_ok, &ollama_no).contains("Claude only"));
        assert!(determine_effective_provider("hybrid", &claude_no, &ollama_ok).contains("Ollama only"));
        assert!(determine_effective_provider("hybrid", &claude_no, &ollama_no).contains("Pattern-based"));
    }

    #[test]
    fn test_determine_effective_provider_online_mode() {
        let claude_ok = ProviderStatus::available("Claude", "claude-sonnet-4-20250514".to_string());
        let claude_no = ProviderStatus::unavailable("Claude", "No key");
        let ollama_ok = ProviderStatus::available("Ollama", "llama3.2:3b".to_string());

        // Online mode - only uses Claude
        assert!(determine_effective_provider("online", &claude_ok, &ollama_ok).contains("Claude"));
        assert!(determine_effective_provider("online", &claude_no, &ollama_ok).contains("None"));
    }

    #[test]
    fn test_determine_effective_provider_offline_mode() {
        let claude_ok = ProviderStatus::available("Claude", "claude-sonnet-4-20250514".to_string());
        let ollama_ok = ProviderStatus::available("Ollama", "llama3.2:3b".to_string());
        let ollama_no = ProviderStatus::unavailable("Ollama", "Not running");

        // Offline mode - only uses Ollama
        assert!(determine_effective_provider("offline", &claude_ok, &ollama_ok).contains("Ollama"));
        assert!(determine_effective_provider("offline", &claude_ok, &ollama_no).contains("Pattern-based"));
    }

    #[test]
    fn test_determine_effective_provider_disabled() {
        let claude_ok = ProviderStatus::available("Claude", "claude-sonnet-4-20250514".to_string());
        let ollama_ok = ProviderStatus::available("Ollama", "llama3.2:3b".to_string());

        assert_eq!(determine_effective_provider("disabled", &claude_ok, &ollama_ok), "Disabled");
        assert_eq!(determine_effective_provider("off", &claude_ok, &ollama_ok), "Disabled");
        assert_eq!(determine_effective_provider("none", &claude_ok, &ollama_ok), "Disabled");
    }

    #[test]
    fn test_provider_status_creation() {
        let available = ProviderStatus::available("Test", "model-v1".to_string());
        assert!(available.available);
        assert_eq!(available.name, "Test");
        assert_eq!(available.model.unwrap(), "model-v1");
        assert!(available.reason.is_none());

        let unavailable = ProviderStatus::unavailable("Test", "Connection refused");
        assert!(!unavailable.available);
        assert_eq!(unavailable.name, "Test");
        assert!(unavailable.model.is_none());
        assert_eq!(unavailable.reason.unwrap(), "Connection refused");
    }
}
