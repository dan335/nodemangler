use super::*;

/// Default config has no theme set.
#[test]
fn test_default_config() {
    let config = AppConfig::default();
    assert!(config.theme.is_none());
}

/// Config survives a JSON serialize/deserialize round-trip.
#[test]
fn test_serialize_deserialize_roundtrip() {
    let config = AppConfig {
        theme: Some("dark_green".to_string()),
    };

    let json = serde_json::to_string(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("dark_green"));
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
    };

    let json = serde_json::to_string_pretty(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("light_blue"));
}
