//! Serialization round-trip tests for core types.
//!
//! Verifies that `Value` variants, `Operation` enums, `Node`s, and
//! `GraphSaveData` survive JSON serialization and deserialization without
//! data loss or corruption.

#[cfg(test)]
mod roundtrip {
    use std::collections::HashMap;

    use crate::{
        get_id,
        node::Node,
        operations::Operation,
        value::{Value, ColorFormat},
        AddNodeType, GraphSaveData,
    };

    #[test]
    fn test_graph_save_data_roundtrip_empty() {
        let data = GraphSaveData {
            version: crate::APP_VERSION.to_string(),
            id: "test-id".to_string(),
            name: "test graph".to_string(),
            nodes: HashMap::new(),
        };

        let json = serde_json::to_string(&data).unwrap();
        let loaded: GraphSaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, crate::APP_VERSION);
        assert_eq!(loaded.id, "test-id");
        assert_eq!(loaded.name, "test graph");
        assert!(loaded.nodes.is_empty());
    }

    // Files saved before the `version` field existed must still load; the
    // missing field deserializes to an empty string via #[serde(default)].
    #[test]
    fn test_graph_save_data_missing_version_defaults_to_empty() {
        let json = r#"{"id":"old-id","name":"old graph","nodes":{}}"#;
        let loaded: GraphSaveData = serde_json::from_str(json).unwrap();

        assert_eq!(loaded.version, "");
        assert_eq!(loaded.id, "old-id");
    }

    #[test]
    fn test_graph_save_data_roundtrip_with_nodes() {
        let node = Node::new(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(100.0, 200.0),
        );

        let mut nodes = HashMap::new();
        let node_id = node.id.clone();
        nodes.insert(node_id.clone(), node);

        let data = GraphSaveData {
            version: crate::APP_VERSION.to_string(),
            id: "graph-1".to_string(),
            name: "my graph".to_string(),
            nodes,
        };

        let json = serde_json::to_string(&data).unwrap();
        let loaded: GraphSaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.nodes.len(), 1);
        let loaded_node = loaded.nodes.get(&node_id).unwrap();
        assert_eq!(loaded_node.settings.name, "add");
        assert_eq!(loaded_node.inputs.len(), 2);
        assert_eq!(loaded_node.outputs.len(), 1);
        assert_eq!(loaded_node.position, glam::Vec2::new(100.0, 200.0));
    }

    #[test]
    fn test_node_serialization_preserves_input_values() {
        let mut node = Node::new(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO,
        );
        node.inputs[0].value = Value::Decimal(42.0);
        node.inputs[1].value = Value::Integer(7);

        let json = serde_json::to_string(&node).unwrap();
        let loaded: Node = serde_json::from_str(&json).unwrap();

        match &loaded.inputs[0].value {
            Value::Decimal(v) => assert_eq!(*v, 42.0),
            other => panic!("Expected Decimal, got {:?}", other),
        }
        match &loaded.inputs[1].value {
            Value::Integer(v) => assert_eq!(*v, 7),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn test_value_bool_serialization() {
        let val = Value::Bool(true);
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Bool(v) => assert!(v),
            other => panic!("Expected Bool, got {:?}", other),
        }
    }

    #[test]
    fn test_value_integer_serialization() {
        let val = Value::Integer(42);
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Integer(v) => assert_eq!(v, 42),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn test_value_decimal_serialization() {
        let val = Value::Decimal(3.25);
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Decimal(v) => assert!((v - 3.25).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[test]
    fn test_value_text_serialization() {
        let val = Value::Text("hello world".to_string());
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Text(v) => assert_eq!(v, "hello world"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_value_string_alias_deserializes_as_text() {
        // Old saved graphs use "String" variant — ensure backward compatibility via serde alias
        let old_json = r#"{"String":"hello world"}"#;
        let loaded: Value = serde_json::from_str(old_json).unwrap();
        match loaded {
            Value::Text(v) => assert_eq!(v, "hello world"),
            other => panic!("Expected Text via String alias, got {:?}", other),
        }
    }

    #[test]
    fn test_value_color_serialization() {
        use crate::color::Color;
        let val = Value::Color(Color::from_srgb_float(0.5, 0.3, 0.7, 1.0));
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Color(c) => {
                assert_eq!(c.r, 0.5);
                assert_eq!(c.g, 0.3);
                assert_eq!(c.b, 0.7);
                assert_eq!(c.a, 1.0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_value_trigger_serialization() {
        let val = Value::Trigger;
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Trigger => {}
            other => panic!("Expected Trigger, got {:?}", other),
        }
    }

    #[test]
    fn test_value_color_format_serialization() {
        let val = Value::ColorFormat(ColorFormat::Rgba8);
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::ColorFormat(cf) => assert_eq!(cf, ColorFormat::Rgba8),
            other => panic!("Expected ColorFormat, got {:?}", other),
        }
    }

    #[test]
    fn test_value_path_serialization() {
        use std::path::PathBuf;
        let val = Value::Path(PathBuf::from("/test/path"));
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::Path(p) => assert_eq!(p, PathBuf::from("/test/path")),
            other => panic!("Expected Path, got {:?}", other),
        }
    }

    #[test]
    fn test_operation_serialization() {
        let op = Operation::OpNumberMathAdd;
        let json = serde_json::to_string(&op).unwrap();
        let loaded: Operation = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.settings().name, "add");
    }

    #[test]
    fn test_multiple_operations_serialization() {
        let operations = vec![
            Operation::OpNumberInputDecimal,
            Operation::OpNumberInputInteger,
            Operation::OpNumberMathAdd,
            Operation::OpColorBlendMode,
        ];

        for op in operations {
            let name = op.settings().name.clone();
            let json = serde_json::to_string(&op).unwrap();
            let loaded: Operation = serde_json::from_str(&json).unwrap();
            assert_eq!(loaded.settings().name, name);
        }
    }

    #[test]
    fn test_graph_with_multiple_node_types() {
        let decimal_node = Node::new(
            "node-1".to_string(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::new(0.0, 0.0),
        );
        let integer_node = Node::new(
            "node-2".to_string(),
            AddNodeType::Operation(Operation::OpNumberInputInteger),
            glam::Vec2::new(100.0, 0.0),
        );
        let add_node = Node::new(
            "node-3".to_string(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(200.0, 0.0),
        );

        let mut nodes = HashMap::new();
        nodes.insert("node-1".to_string(), decimal_node);
        nodes.insert("node-2".to_string(), integer_node);
        nodes.insert("node-3".to_string(), add_node);

        let data = GraphSaveData {
            version: crate::APP_VERSION.to_string(),
            id: "graph-multi".to_string(),
            name: "multi-node graph".to_string(),
            nodes,
        };

        let json = serde_json::to_string(&data).unwrap();
        let loaded: GraphSaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.nodes.len(), 3);
        assert_eq!(loaded.nodes.get("node-1").unwrap().settings.name, "decimal");
        assert_eq!(loaded.nodes.get("node-2").unwrap().settings.name, "integer");
        assert_eq!(loaded.nodes.get("node-3").unwrap().settings.name, "add");
    }

    // GraphSaveData's `nodes` field is deserialized/serialized tolerantly
    // through `crate::saved_nodes` (see version.rs / node_type.rs /
    // saved_nodes.rs): a node whose type this build doesn't recognize
    // becomes a placeholder instead of aborting the whole parse, and it
    // round-trips through a re-serialize without losing anything.
    #[test]
    fn test_graph_save_data_mixed_known_and_unknown_nodes_round_trips_json() {
        let known = Node::new(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(10.0, 20.0),
        );
        let known_id = known.id.clone();
        let mut nodes = HashMap::new();
        nodes.insert(known_id.clone(), known);

        let data = GraphSaveData {
            version: crate::APP_VERSION.to_string(),
            id: "mixed-graph".to_string(),
            name: "mixed".to_string(),
            nodes,
        };
        let mut json = serde_json::to_value(&data).unwrap();

        // Splice in a hand-crafted node whose operation string this build
        // doesn't recognize — simulating a node saved by a newer NodeMangler.
        let unknown_id = get_id();
        let template = Node::new(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(1.0, 1.0),
        );
        let mut unknown_raw = serde_json::to_value(&template).unwrap();
        unknown_raw["node_type"]["Operation"]["operation"] = serde_json::json!("OpFromTheFuture");
        json["nodes"][unknown_id.as_str()] = unknown_raw.clone();

        // Deserialize: the tolerant path folds the unknown node into a placeholder.
        let loaded: GraphSaveData = serde_json::from_value(json).unwrap();
        assert_eq!(loaded.nodes.len(), 2);
        assert!(matches!(
            loaded.nodes.get(&unknown_id).unwrap().node_type,
            crate::node_type::NodeType::Unknown { .. }
        ));

        // Serialize it straight back out. With no edits in between, the
        // unknown node's JSON must equal the original raw exactly.
        let reserialized = serde_json::to_value(&loaded).unwrap();
        assert_eq!(
            reserialized["nodes"][unknown_id.as_str()], unknown_raw,
            "an unrecognized node must round-trip byte-for-byte with no live edits"
        );
        assert_eq!(reserialized["nodes"][known_id.as_str()]["settings"]["name"], "add");
    }

    #[test]
    fn test_filter_type_serialization() {
        use image::imageops::FilterType;
        let val = Value::FilterType(FilterType::Lanczos3);
        let json = serde_json::to_string(&val).unwrap();
        let loaded: Value = serde_json::from_str(&json).unwrap();
        match loaded {
            Value::FilterType(ft) => assert_eq!(ft, FilterType::Lanczos3),
            other => panic!("Expected FilterType, got {:?}", other),
        }
    }
}
