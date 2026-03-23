/// Application configuration that persists across sessions.
/// Stored as JSON in the platform-appropriate config directory
/// (e.g. %APPDATA%/nodemangler/config.json on Windows).

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// The theme name to restore on startup (e.g. "dark_green").
    #[serde(default)]
    pub theme: Option<String>,

    /// API keys for external services.
    #[serde(default)]
    pub api_keys: ApiKeys,
}

/// API keys for AI providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    /// OpenAI API key (used for DALL-E, GPT-Image, etc.).
    #[serde(default)]
    pub openai: String,
}

impl AppConfig {
    /// Returns the path to the config file.
    /// On Windows: %APPDATA%/nodemangler/config.json
    /// On Linux: ~/.config/nodemangler/config.json
    /// On macOS: ~/Library/Application Support/nodemangler/config.json
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("nodemangler").join("config.json"))
    }

    /// Load the config from disk. Returns default config if the file
    /// doesn't exist or can't be parsed.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Self::default();
        };

        serde_json::from_str(&contents).unwrap_or_default()
    }

    /// Save the config to disk. Creates parent directories if needed.
    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };

        // Create parent directory if it doesn't exist.
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Set the OPENAI_API_KEY env var from config if it's non-empty
    /// and the env var isn't already set.
    pub fn apply_api_keys_to_env(&self) {
        let key = self.api_keys.openai.trim();
        if !key.is_empty() {
            // Only set if not already provided by the user's environment.
            if std::env::var("OPENAI_API_KEY").is_err() {
                // SAFETY: single-threaded at startup before tokio runtime spawns tasks.
                unsafe { std::env::set_var("OPENAI_API_KEY", key); }
            }
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
