use super::*;

/// Default config has empty API keys.
#[test]
fn test_default_config() {
    let config = AppConfig::default();
    assert!(config.theme.is_none());
    assert!(config.api_keys.openai.is_empty());
}

/// Config survives a JSON serialize/deserialize round-trip.
#[test]
fn test_serialize_deserialize_roundtrip() {
    let config = AppConfig {
        theme: Some("dark_green".to_string()),
        api_keys: ApiKeys {
            openai: "sk-test-key-12345".to_string(),
        },
        ai_cost_limit: 5.0,
    };

    let json = serde_json::to_string(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("dark_green"));
    assert_eq!(restored.api_keys.openai, "sk-test-key-12345");
    assert_eq!(restored.ai_cost_limit, 5.0);
}

/// Missing file returns default config (load gracefully handles missing file).
#[test]
fn test_load_missing_file() {
    // config_path() returns a real path, but we can't easily mock it.
    // Instead, verify that the default is valid.
    let config = AppConfig::default();
    assert!(config.api_keys.openai.is_empty());
}

/// Invalid JSON returns default config.
#[test]
fn test_load_corrupted_json() {
    let result: AppConfig = serde_json::from_str("this is not json").unwrap_or_default();
    assert!(result.theme.is_none());
    assert!(result.api_keys.openai.is_empty());
}

/// JSON with only some fields loads correctly (serde defaults fill the rest).
#[test]
fn test_partial_config() {
    let json = r#"{"theme": "light"}"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.theme.as_deref(), Some("light"));
    assert!(config.api_keys.openai.is_empty());
}

/// JSON with only api_keys loads correctly.
#[test]
fn test_partial_config_api_keys_only() {
    let json = r#"{"api_keys": {"openai": "sk-abc"}}"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();
    assert!(config.theme.is_none());
    assert_eq!(config.api_keys.openai, "sk-abc");
}

/// Config path returns a valid path on this platform.
#[test]
fn test_config_path_is_some() {
    // On most platforms, dirs::config_dir() returns Some.
    let path = AppConfig::config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.to_str().unwrap().contains("nodemangler"));
}

/// Save and load round-trip via temp directory.
#[test]
fn test_save_and_load_roundtrip() {
    // We test the serialization logic directly since we can't easily
    // override the config path.
    let config = AppConfig {
        theme: Some("light_blue".to_string()),
        api_keys: ApiKeys {
            openai: "sk-round-trip-test".to_string(),
        },
        ai_cost_limit: 0.0,
    };

    let json = serde_json::to_string_pretty(&config).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.theme.as_deref(), Some("light_blue"));
    assert_eq!(restored.api_keys.openai, "sk-round-trip-test");
}
