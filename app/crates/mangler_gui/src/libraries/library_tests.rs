use super::*;

/// `LibraryConfig` survives a JSON serialize/deserialize round-trip.
#[test]
fn test_library_config_roundtrip() {
    let config = LibraryConfig {
        name: "My Textures".to_string(),
        source: LibrarySource::Local {
            path: PathBuf::from("D:/textures"),
        },
    };

    let json = serde_json::to_string(&config).unwrap();
    let restored: LibraryConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored, config);
}

/// Pin the tagged-enum JSON shape so on-disk config files stay stable:
/// `{"type": "local", "path": "..."}` inside the `source` field.
#[test]
fn test_library_source_tagged_json_shape() {
    let source = LibrarySource::Local {
        path: PathBuf::from("C:/graphs"),
    };

    let json = serde_json::to_value(&source).unwrap();
    assert_eq!(json["type"], "local");
    assert_eq!(json["path"], "C:/graphs");
}

/// Local sources are always writable.
#[test]
fn test_local_source_is_not_read_only() {
    let source = LibrarySource::Local {
        path: PathBuf::from("C:/graphs"),
    };
    assert!(!source.read_only());
}

/// `local_path()` returns the wrapped path for a local source.
#[test]
fn test_local_source_local_path() {
    let source = LibrarySource::Local {
        path: PathBuf::from("C:/graphs"),
    };
    assert_eq!(source.local_path(), Some(Path::new("C:/graphs")));
}
