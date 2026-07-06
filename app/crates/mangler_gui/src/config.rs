/// Application configuration that persists across sessions.
/// Stored as JSON in the platform-appropriate config directory
/// (e.g. %APPDATA%/nodemangler/config.json on Windows).

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::panels::panel_tree::PanelNode;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// The theme name to restore on startup (e.g. "dark_green").
    #[serde(default)]
    pub theme: Option<String>,
    /// The user's saved panel layout for the main window, set via "set panel
    /// layout as default". `None` means fall back to `PanelTree::system_default`.
    #[serde(default)]
    pub default_layout: Option<PanelNode>,
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
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
