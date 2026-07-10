/// Application configuration that persists across sessions.
/// Stored as JSON in the platform-appropriate config directory
/// (e.g. %APPDATA%/nodemangler/config.json on Windows).

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::libraries::library::{LibraryConfig, LibrarySource};
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
    /// Linked libraries (name + folder). Only the link is stored; content is
    /// rescanned from disk each session.
    #[serde(default)]
    pub libraries: Vec<LibraryConfig>,
    /// Folder brand-new graphs auto-save into until the user picks somewhere
    /// else (see `ensure_default_library`). `None` means it hasn't been
    /// computed yet, or the computed location turned out to be unwritable.
    #[serde(default)]
    pub default_library: Option<PathBuf>,
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

    /// Ensures a folder exists on disk for brand-new graphs to auto-save
    /// into, and that a library entry named "NodeMangler" points at it, so it
    /// shows up in the Libraries panel without the user doing anything.
    ///
    /// Idempotent: once `default_library` is set, subsequent calls just
    /// re-verify the folder still exists (recreating it if it was deleted)
    /// and return the same path, without adding a duplicate library entry.
    /// Returns `None` if no candidate location could be created (e.g. every
    /// candidate directory is unwritable).
    ///
    /// Does **not** persist the change — callers must call [`Self::save`]
    /// afterwards, same as `default_layout`'s "set as default" mutator.
    pub fn ensure_default_library(&mut self) -> Option<PathBuf> {
        let candidates = default_library_candidate_dirs();
        self.ensure_default_library_in(&candidates)
    }

    /// Core logic behind [`Self::ensure_default_library`], parameterized on
    /// candidate base directories so tests can inject a temp dir instead of
    /// the real document/home dirs.
    fn ensure_default_library_in(&mut self, candidates: &[PathBuf]) -> Option<PathBuf> {
        if let Some(existing) = self.default_library.clone() {
            if std::fs::create_dir_all(&existing).is_ok() {
                self.link_default_library(&existing);
                return Some(existing);
            }
            // The configured folder can no longer be created (e.g. it lived
            // on a removable/network drive that's now gone) — fall through
            // and recompute a fresh default below.
        }

        let path = candidates.first()?.join("NodeMangler");
        std::fs::create_dir_all(&path).ok()?;

        self.default_library = Some(path.clone());
        self.link_default_library(&path);
        Some(path)
    }

    /// Adds a "NodeMangler" library entry pointing at `path`, unless one
    /// already exists at that path (same de-dup rule as
    /// `LibrariesState::add_library`).
    fn link_default_library(&mut self, path: &Path) {
        let already_linked = self
            .libraries
            .iter()
            .any(|lib| lib.source.local_path() == Some(path));
        if !already_linked {
            self.libraries.push(LibraryConfig {
                name: "NodeMangler".to_string(),
                source: LibrarySource::Local {
                    path: path.to_path_buf(),
                },
            });
        }
    }
}

/// Candidate base directories for the default library, in preference order:
/// the user's Documents folder, their home folder, then the config file's
/// own parent directory as a last resort (always writable if the app can
/// save config at all).
fn default_library_candidate_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = dirs::document_dir() {
        candidates.push(dir);
    }
    if let Some(dir) = dirs::home_dir() {
        candidates.push(dir);
    }
    if let Some(parent) = AppConfig::config_path().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
        candidates.push(parent);
    }
    candidates
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
