use super::*;
use crate::libraries::library::{LibraryConfig, LibrarySource};
use crate::panels::{
    panel_kind::PanelKind,
    panel_tree::{PanelNode, SplitDirection},
};

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
    };

    let json = serde_json::to_string_pretty(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("light_blue"));
}
