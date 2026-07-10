use super::*;
use crate::libraries::library::{LibraryConfig, LibrarySource};
use crate::panels::{
    panel_kind::PanelKind,
    panel_tree::{PanelNode, SplitDirection},
};
use std::path::PathBuf;

/// Default config has no theme set.
#[test]
fn test_default_config() {
    let config = AppConfig::default();
    assert!(config.theme.is_none());
    assert!(config.default_layout.is_none());
    assert!(config.libraries.is_empty());
}

/// Config survives a JSON serialize/deserialize round-trip.
#[test]
fn test_serialize_deserialize_roundtrip() {
    let config = AppConfig {
        theme: Some("dark_green".to_string()),
        default_layout: None,
        libraries: Vec::new(),
        default_library: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("dark_green"));
}

/// Config with a `libraries` entry survives a JSON round-trip.
#[test]
fn test_serialize_deserialize_roundtrip_with_libraries() {
    let config = AppConfig {
        theme: Some("dark_green".to_string()),
        default_layout: None,
        libraries: vec![LibraryConfig {
            name: "My Textures".to_string(),
            source: LibrarySource::Local {
                path: std::path::PathBuf::from("D:/textures"),
            },
        }],
        default_library: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.libraries, config.libraries);
}

/// Config with a nested `default_layout` survives a JSON round-trip.
#[test]
fn test_serialize_deserialize_roundtrip_with_layout() {
    let layout = PanelNode::Split {
        direction: SplitDirection::Row,
        fraction: 0.25,
        children: [
            Box::new(PanelNode::Leaf {
                id: 0,
                kind: PanelKind::NodeList,
            }),
            Box::new(PanelNode::Split {
                direction: SplitDirection::Row,
                fraction: 0.75,
                children: [
                    Box::new(PanelNode::Leaf {
                        id: 1,
                        kind: PanelKind::Graph,
                    }),
                    Box::new(PanelNode::Leaf {
                        id: 2,
                        kind: PanelKind::Settings,
                    }),
                ],
            }),
        ],
    };
    let config = AppConfig {
        theme: Some("dark_green".to_string()),
        default_layout: Some(layout.clone()),
        libraries: Vec::new(),
        default_library: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("dark_green"));
    assert_eq!(restored.default_layout, Some(layout));
}

/// Existing theme-only config JSON (pre-Phase-4) still loads, with
/// `default_layout` defaulting to `None`.
#[test]
fn test_theme_only_json_back_compat() {
    let json = r#"{"theme": "dark_green"}"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.theme.as_deref(), Some("dark_green"));
    assert!(config.default_layout.is_none());
    assert!(config.libraries.is_empty());
}

/// An empty JSON object parses to an all-default config.
#[test]
fn test_empty_json_object() {
    let config: AppConfig = serde_json::from_str("{}").unwrap();
    assert!(config.theme.is_none());
    assert!(config.default_layout.is_none());
    assert!(config.libraries.is_empty());
}

/// Invalid JSON returns default config.
#[test]
fn test_load_corrupted_json() {
    let result: AppConfig = serde_json::from_str("this is not json").unwrap_or_default();
    assert!(result.theme.is_none());
}

/// JSON with only some fields loads correctly (serde defaults fill the rest).
#[test]
fn test_partial_config() {
    let json = r#"{"theme": "light"}"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.theme.as_deref(), Some("light"));
}

/// Config path returns a valid path on this platform.
#[test]
fn test_config_path_is_some() {
    let path = AppConfig::config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.to_str().unwrap().contains("nodemangler"));
}

/// Save/load serialization round-trip.
#[test]
fn test_save_and_load_roundtrip() {
    let config = AppConfig {
        theme: Some("light_blue".to_string()),
        default_layout: None,
        libraries: Vec::new(),
        default_library: None,
    };

    let json = serde_json::to_string_pretty(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("light_blue"));
}

/// Monotonic counter so parallel `cargo test` runs never collide on the same
/// temp directory name (mirrors `library_scanner_tests::make_temp_dir`).
static UNIQUE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Builds a fresh, uniquely-named directory under the OS temp dir. Caller is
/// responsible for cleanup via `std::fs::remove_dir_all`.
fn make_temp_dir(label: &str) -> PathBuf {
    let n = UNIQUE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "mangler_gui_config_test_{}_{}_{}",
        std::process::id(),
        label,
        n
    ));
    dir
}

/// `ensure_default_library_in` creates the "NodeMangler" folder under the
/// first candidate dir and registers a matching library entry.
#[test]
fn test_ensure_default_library_creates_dir_and_registers_library() {
    let base = make_temp_dir("creates");
    let expected = base.join("NodeMangler");

    let mut config = AppConfig::default();
    let result = config.ensure_default_library_in(&[base.clone()]);

    assert_eq!(result, Some(expected.clone()));
    assert!(expected.is_dir());
    assert_eq!(config.default_library, Some(expected.clone()));
    assert_eq!(config.libraries.len(), 1);
    assert_eq!(config.libraries[0].name, "NodeMangler");
    assert_eq!(config.libraries[0].source.local_path(), Some(expected.as_path()));

    std::fs::remove_dir_all(&base).ok();
}

/// A second call returns the same path and does not add a duplicate library
/// entry.
#[test]
fn test_ensure_default_library_is_idempotent() {
    let base = make_temp_dir("idempotent");
    let expected = base.join("NodeMangler");

    let mut config = AppConfig::default();
    let first = config.ensure_default_library_in(&[base.clone()]);
    let second = config.ensure_default_library_in(&[base.clone()]);

    assert_eq!(first, second);
    assert_eq!(first, Some(expected));
    assert_eq!(config.libraries.len(), 1);

    std::fs::remove_dir_all(&base).ok();
}

/// A pre-existing "NodeMangler" folder is tolerated (not recreated/cleared),
/// and its content survives the call.
#[test]
fn test_ensure_default_library_tolerates_pre_existing_folder() {
    let base = make_temp_dir("pre_existing");
    let expected = base.join("NodeMangler");
    std::fs::create_dir_all(&expected).unwrap();
    std::fs::write(expected.join("marker.mangler.json"), "{}").unwrap();

    let mut config = AppConfig::default();
    let result = config.ensure_default_library_in(&[base.clone()]);

    assert_eq!(result, Some(expected.clone()));
    assert!(expected.join("marker.mangler.json").is_file());

    std::fs::remove_dir_all(&base).ok();
}

/// If `default_library` is already set and still exists, later calls reuse
/// it even when given different (irrelevant) candidates.
#[test]
fn test_ensure_default_library_reuses_existing_configured_path() {
    let base = make_temp_dir("reuse");
    let expected = base.join("NodeMangler");
    std::fs::create_dir_all(&expected).unwrap();

    let mut config = AppConfig {
        default_library: Some(expected.clone()),
        ..AppConfig::default()
    };
    // Candidates list is deliberately unrelated — should be ignored since
    // `default_library` is already set and usable.
    let other = make_temp_dir("reuse_unused_candidate");
    let result = config.ensure_default_library_in(&[other.clone()]);

    assert_eq!(result, Some(expected));
    assert_eq!(config.libraries.len(), 1);

    std::fs::remove_dir_all(&base).ok();
}

/// Empty candidate list (e.g. every real base dir was `None`) fails cleanly.
#[test]
fn test_ensure_default_library_no_candidates_returns_none() {
    let mut config = AppConfig::default();
    let result = config.ensure_default_library_in(&[]);
    assert!(result.is_none());
    assert!(config.default_library.is_none());
    assert!(config.libraries.is_empty());
}
