#![allow(dead_code)]

//! Notification system for user feedback
//!
//! Provides a consistent way to display messages to users with appropriate
//! formatting based on severity level. Supports both immediate display and
//! queued notifications.

use colored::Colorize;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use super::output::colors_enabled;

/// Global flag to track if we've shown fallback warning this session
static FALLBACK_WARNING_SHOWN: AtomicBool = AtomicBool::new(false);

/// Notification severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Operation succeeded
    Success,
    /// Informational message
    Info,
    /// Degraded functionality, fallback in use
    Warning,
    /// Operation failed
    Error,
}

impl Level {
    /// Get the icon for this level
    pub fn icon(&self) -> &'static str {
        match self {
            Level::Success => "✓",
            Level::Info => "ℹ",
            Level::Warning => "⚠",
            Level::Error => "✗",
        }
    }

    /// Get the label for this level
    pub fn label(&self) -> &'static str {
        match self {
            Level::Success => "SUCCESS",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A notification message
#[derive(Debug, Clone)]
pub struct Notification {
    pub level: Level,
    pub message: String,
    pub context: Option<String>,
}

impl Notification {
    /// Create a success notification
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            level: Level::Success,
            message: message.into(),
            context: None,
        }
    }

    /// Create an info notification
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: Level::Info,
            message: message.into(),
            context: None,
        }
    }

    /// Create a warning notification
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            message: message.into(),
            context: None,
        }
    }

    /// Create an error notification
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            message: message.into(),
            context: None,
        }
    }

    /// Add context to the notification
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Display the notification immediately
    pub fn show(&self) {
        let formatted = self.format();
        eprintln!("{}", formatted);
    }

    /// Format the notification for display
    pub fn format(&self) -> String {
        let icon = self.level.icon();
        let message = &self.message;

        if colors_enabled() {
            let styled = match self.level {
                Level::Success => format!("{} {}", icon.green(), message.green()),
                Level::Info => format!("{} {}", icon.blue(), message.blue()),
                Level::Warning => format!("{} {}", icon.yellow(), message.yellow()),
                Level::Error => format!("{} {}", icon.red(), message.red()),
            };

            if let Some(ref ctx) = self.context {
                format!("{}\n  └─ {}", styled, ctx.dimmed())
            } else {
                styled
            }
        } else {
            let prefix = format!("[{}]", self.level.label());
            if let Some(ref ctx) = self.context {
                format!("{} {} {}\n  └─ {}", icon, prefix, message, ctx)
            } else {
                format!("{} {} {}", icon, prefix, message)
            }
        }
    }
}

/// Display a notification immediately
pub fn notify(notification: Notification) {
    notification.show();
}

/// Display a success message
pub fn success(message: impl Into<String>) {
    Notification::success(message).show();
}

/// Display an info message
pub fn info(message: impl Into<String>) {
    Notification::info(message).show();
}

/// Display a warning message
pub fn warning(message: impl Into<String>) {
    Notification::warning(message).show();
}

/// Display an error message
pub fn error(message: impl Into<String>) {
    Notification::error(message).show();
}

/// Display a warning about AI fallback (only once per session)
pub fn warn_ai_fallback(reason: &str) {
    if !FALLBACK_WARNING_SHOWN.swap(true, Ordering::SeqCst) {
        Notification::warning("Using pattern-based parsing (LLM unavailable)")
            .with_context(reason.to_string())
            .show();
    }
}

/// Reset the fallback warning flag (for testing)
#[cfg(test)]
pub fn reset_fallback_warning() {
    FALLBACK_WARNING_SHOWN.store(false, Ordering::SeqCst);
}

/// System status for display
#[derive(Debug)]
pub struct SystemStatus {
    /// Configuration file path
    pub config_path: Option<String>,
    /// Database file path
    pub db_path: Option<String>,
    /// Whether testnet mode is enabled
    pub testnet_mode: bool,
    /// Claude API status
    pub claude_status: ProviderStatus,
    /// Ollama status
    pub ollama_status: ProviderStatus,
    /// Active AI mode
    pub ai_mode: String,
    /// Effective provider being used
    pub effective_provider: String,
}

/// Status of an AI provider
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub name: &'static str,
    pub available: bool,
    pub model: Option<String>,
    pub reason: Option<String>,
}

impl ProviderStatus {
    pub fn available(name: &'static str, model: String) -> Self {
        Self {
            name,
            available: true,
            model: Some(model),
            reason: None,
        }
    }

    pub fn unavailable(name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            name,
            available: false,
            model: None,
            reason: Some(reason.into()),
        }
    }
}

impl SystemStatus {
    /// Format the system status for display
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(String::new());
        if colors_enabled() {
            lines.push("  📊 System Status".bold().to_string());
        } else {
            lines.push("  System Status".to_string());
        }
        lines.push("  ─────────────────────────────────────".to_string());

        // Config & DB
        lines.push(self.format_line(
            "📁",
            "Config",
            self.config_path.as_deref().unwrap_or("Not found"),
            self.config_path.is_some(),
        ));

        lines.push(self.format_line(
            "🗄️",
            "Database",
            self.db_path.as_deref().unwrap_or("Not found"),
            self.db_path.is_some(),
        ));

        // Network mode
        let mode_str = if self.testnet_mode { "Testnet (safe)" } else { "Mainnet (real funds)" };
        let mode_icon = if self.testnet_mode { "🧪" } else { "🌐" };
        lines.push(self.format_line(mode_icon, "Mode", mode_str, true));

        lines.push(String::new());

        // AI Providers header
        if colors_enabled() {
            lines.push("  🤖 AI Providers".bold().to_string());
        } else {
            lines.push("  AI Providers".to_string());
        }
        lines.push("  ─────────────────────────────────────".to_string());

        // Claude status
        lines.push(self.format_provider_line(&self.claude_status, "☁️"));

        // Ollama status
        lines.push(self.format_provider_line(&self.ollama_status, "🦙"));

        // Active mode
        lines.push(String::new());
        lines.push(self.format_line("⚡", "AI Mode", &self.ai_mode, true));
        lines.push(self.format_line(
            "🎯",
            "Active",
            &self.effective_provider,
            self.effective_provider != "None" && self.effective_provider != "Pattern-based",
        ));

        lines.push(String::new());

        lines.join("\n")
    }

    fn format_line(&self, icon: &str, label: &str, value: &str, ok: bool) -> String {
        if colors_enabled() {
            let status = if ok {
                value.green().to_string()
            } else {
                value.red().to_string()
            };
            format!("  {} {:<12} {}", icon, label, status)
        } else {
            let status_icon = if ok { "✓" } else { "✗" };
            format!("  {} {:<12} {} {}", icon, label, status_icon, value)
        }
    }

    fn format_provider_line(&self, provider: &ProviderStatus, icon: &str) -> String {
        if provider.available {
            let model = provider.model.as_deref().unwrap_or("unknown");
            if colors_enabled() {
                format!(
                    "  {} {:<12} {} ({})",
                    icon,
                    provider.name,
                    "Connected".green(),
                    model.dimmed()
                )
            } else {
                format!("  {} {:<12} ✓ Connected ({})", icon, provider.name, model)
            }
        } else {
            let reason = provider.reason.as_deref().unwrap_or("unavailable");
            if colors_enabled() {
                format!(
                    "  {} {:<12} {} ({})",
                    icon,
                    provider.name,
                    "Offline".red(),
                    reason.dimmed()
                )
            } else {
                format!("  {} {:<12} ✗ Offline ({})", icon, provider.name, reason)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_levels() {
        assert_eq!(Level::Success.icon(), "✓");
        assert_eq!(Level::Warning.icon(), "⚠");
        assert_eq!(Level::Error.icon(), "✗");
        assert_eq!(Level::Info.icon(), "ℹ");
    }

    #[test]
    fn test_level_labels() {
        assert_eq!(Level::Success.label(), "SUCCESS");
        assert_eq!(Level::Info.label(), "INFO");
        assert_eq!(Level::Warning.label(), "WARNING");
        assert_eq!(Level::Error.label(), "ERROR");
    }

    #[test]
    fn test_level_display() {
        assert_eq!(format!("{}", Level::Success), "SUCCESS");
        assert_eq!(format!("{}", Level::Warning), "WARNING");
    }

    #[test]
    fn test_notification_format() {
        let n = Notification::warning("Test warning");
        assert!(n.format().contains("Test warning"));
        assert!(n.format().contains("⚠"));
    }

    #[test]
    fn test_notification_with_context() {
        let n = Notification::error("Failed").with_context("Connection refused");
        let formatted = n.format();
        assert!(formatted.contains("Failed"));
        assert!(formatted.contains("Connection refused"));
    }

    #[test]
    fn test_notification_success() {
        let n = Notification::success("Operation completed");
        assert_eq!(n.level, Level::Success);
        assert_eq!(n.message, "Operation completed");
        assert!(n.context.is_none());
    }

    #[test]
    fn test_notification_info() {
        let n = Notification::info("Status update");
        assert_eq!(n.level, Level::Info);
        assert_eq!(n.message, "Status update");
    }

    #[test]
    fn test_notification_warning() {
        let n = Notification::warning("Degraded mode");
        assert_eq!(n.level, Level::Warning);
        assert_eq!(n.message, "Degraded mode");
    }

    #[test]
    fn test_notification_error() {
        let n = Notification::error("Connection failed");
        assert_eq!(n.level, Level::Error);
        assert_eq!(n.message, "Connection failed");
    }

    #[test]
    fn test_provider_status_available() {
        let status = ProviderStatus::available("Claude", "claude-sonnet-4-20250514".to_string());
        assert!(status.available);
        assert_eq!(status.name, "Claude");
        assert_eq!(status.model, Some("claude-sonnet-4-20250514".to_string()));
        assert!(status.reason.is_none());
    }

    #[test]
    fn test_provider_status_unavailable() {
        let status = ProviderStatus::unavailable("Ollama", "Not running");
        assert!(!status.available);
        assert_eq!(status.name, "Ollama");
        assert!(status.model.is_none());
        assert_eq!(status.reason, Some("Not running".to_string()));
    }

    #[test]
    fn test_system_status_format() {
        let status = SystemStatus {
            config_path: Some("/path/to/config.toml".to_string()),
            db_path: Some("/path/to/database.sqlite".to_string()),
            testnet_mode: true,
            claude_status: ProviderStatus::unavailable("Claude", "No API key"),
            ollama_status: ProviderStatus::available("Ollama", "llama3.2:3b".to_string()),
            ai_mode: "Hybrid (Local + Cloud)".to_string(),
            effective_provider: "Ollama only (llama3.2:3b)".to_string(),
        };

        let formatted = status.format();
        assert!(formatted.contains("System Status"));
        assert!(formatted.contains("Config"));
        assert!(formatted.contains("Database"));
        assert!(formatted.contains("Testnet"));
        assert!(formatted.contains("AI Providers"));
        assert!(formatted.contains("Claude"));
        assert!(formatted.contains("Ollama"));
        assert!(formatted.contains("AI Mode"));
    }

    #[test]
    fn test_system_status_missing_paths() {
        let status = SystemStatus {
            config_path: None,
            db_path: None,
            testnet_mode: false,
            claude_status: ProviderStatus::unavailable("Claude", "No API key"),
            ollama_status: ProviderStatus::unavailable("Ollama", "Not running"),
            ai_mode: "Disabled".to_string(),
            effective_provider: "None".to_string(),
        };

        let formatted = status.format();
        assert!(formatted.contains("Not found"));
    }
}
