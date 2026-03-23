use super::*;
use crate::graph::graph_node::GraphNode;
use eframe::egui::Pos2;
use mangler_core::input::Input;
use mangler_core::node_settings::NodeSettings;
use mangler_core::operations::Operation;
use mangler_core::output::Output;
use mangler_core::value::Value;
use mangler_core::AddNodeType;
use std::collections::{HashMap, HashSet};

/// Helper to create a test node with an operation type and a given position.
fn make_node(id: &str, pos: Pos2, inputs: Vec<Input>, outputs: Vec<Output>) -> GraphNode {
    GraphNode::new(
        id.to_string(),
        pos,
        NodeSettings::default(),
        inputs,
        outputs,
        false,
        Some(AddNodeType::Operation(Operation::OpNumberInputInteger)),
    )
}

/// Helper to create a simple input with a value.
fn make_input(name: &str, value: Value) -> Input {
    Input::new(name.to_string(), value, None, None)
}

/// Helper to create a simple output with a value.
fn make_output(name: &str, value: Value) -> Output {
    Output::new(name.to_string(), value, None)
}

#[test]
fn test_from_selection_empty_selection_returns_none() {
    // Copying with no nodes selected should return None.
    let graph_nodes: HashMap<String, GraphNode> = HashMap::new();
    let selected: HashSet<String> = HashSet::new();
    assert!(Clipboard::from_selection(&selected, &graph_nodes).is_none());
}

#[test]
fn test_from_selection_single_node() {
    // Copying a single node should produce a clipboard with one node and no connections.
    let mut graph_nodes = HashMap::new();
    let node = make_node(
        "a",
        Pos2::new(100.0, 200.0),
        vec![make_input("x", Value::Integer(42))],
        vec![make_output("out", Value::Integer(0))],
    );
    graph_nodes.insert("a".to_string(), node);

    let mut selected = HashSet::new();
    selected.insert("a".to_string());

    let cb = Clipboard::from_selection(&selected, &graph_nodes).unwrap();
    assert_eq!(cb.nodes.len(), 1);
    assert_eq!(cb.connections.len(), 0);
    assert_eq!(cb.nodes[0].original_id, "a");
    assert_eq!(cb.nodes[0].position, Pos2::new(100.0, 200.0));
    // Input value should be captured.
    assert_eq!(cb.nodes[0].input_values.len(), 1);
    assert_eq!(cb.nodes[0].input_values[0].0, 0);
    assert!(cb.nodes[0].is_enabled);
}

#[test]
fn test_from_selection_preserves_internal_connections() {
    // Two connected nodes: a -> b. Both selected. The connection should be captured.
    let mut graph_nodes = HashMap::new();

    let node_a = make_node(
        "a",
        Pos2::new(0.0, 0.0),
        vec![],
        vec![make_output("out", Value::Integer(0))],
    );

    let mut input_b = make_input("in", Value::Integer(0));
    input_b.connection = Some(("a".to_string(), 0));
    let node_b = make_node(
        "b",
        Pos2::new(200.0, 0.0),
        vec![input_b],
        vec![make_output("out", Value::Integer(0))],
    );

    graph_nodes.insert("a".to_string(), node_a);
    graph_nodes.insert("b".to_string(), node_b);

    let mut selected = HashSet::new();
    selected.insert("a".to_string());
    selected.insert("b".to_string());

    let cb = Clipboard::from_selection(&selected, &graph_nodes).unwrap();
    assert_eq!(cb.nodes.len(), 2);
    assert_eq!(cb.connections.len(), 1);

    let conn = &cb.connections[0];
    assert_eq!(conn.output_node_id, "a");
    assert_eq!(conn.output_index, 0);
    assert_eq!(conn.input_node_id, "b");
    assert_eq!(conn.input_index, 0);
}

#[test]
fn test_from_selection_excludes_external_connections() {
    // Node b connects to node a, but only b is selected.
    // The connection should not be in the clipboard.
    let mut graph_nodes = HashMap::new();

    let node_a = make_node(
        "a",
        Pos2::new(0.0, 0.0),
        vec![],
        vec![make_output("out", Value::Integer(0))],
    );

    let mut input_b = make_input("in", Value::Integer(0));
    input_b.connection = Some(("a".to_string(), 0));
    let node_b = make_node(
        "b",
        Pos2::new(200.0, 0.0),
        vec![input_b],
        vec![],
    );

    graph_nodes.insert("a".to_string(), node_a);
    graph_nodes.insert("b".to_string(), node_b);

    // Only select b.
    let mut selected = HashSet::new();
    selected.insert("b".to_string());

    let cb = Clipboard::from_selection(&selected, &graph_nodes).unwrap();
    assert_eq!(cb.nodes.len(), 1);
    assert_eq!(cb.connections.len(), 0); // Connection to 'a' is external.
}

#[test]
fn test_from_selection_skips_nodes_without_node_type() {
    // A node loaded from a save file has node_type = None and should be skipped.
    let mut graph_nodes = HashMap::new();
    let mut node = make_node("a", Pos2::ZERO, vec![], vec![]);
    node.node_type = None; // Simulate loaded node.
    graph_nodes.insert("a".to_string(), node);

    let mut selected = HashSet::new();
    selected.insert("a".to_string());

    // No copyable nodes, should return None.
    assert!(Clipboard::from_selection(&selected, &graph_nodes).is_none());
}

#[test]
fn test_from_selection_captures_disabled_state() {
    // A disabled node should have is_enabled = false in the clipboard.
    let mut graph_nodes = HashMap::new();
    let mut node = make_node("a", Pos2::ZERO, vec![], vec![]);
    node.is_enabled = false;
    graph_nodes.insert("a".to_string(), node);

    let mut selected = HashSet::new();
    selected.insert("a".to_string());

    let cb = Clipboard::from_selection(&selected, &graph_nodes).unwrap();
    assert!(!cb.nodes[0].is_enabled);
}

#[test]
fn test_centroid_single_node() {
    // Centroid of a single node is its own position.
    let cb = Clipboard {
        nodes: vec![ClipboardNode {
            original_id: "a".to_string(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputInteger),
            position: Pos2::new(100.0, 200.0),
            input_values: vec![],
            is_enabled: true,
        }],
        connections: vec![],
    };
    let c = cb.centroid();
    assert!((c.x - 100.0).abs() < 0.001);
    assert!((c.y - 200.0).abs() < 0.001);
}

#[test]
fn test_centroid_multiple_nodes() {
    // Centroid should be the average of all node positions.
    let cb = Clipboard {
        nodes: vec![
            ClipboardNode {
                original_id: "a".to_string(),
                node_type: AddNodeType::Operation(Operation::OpNumberInputInteger),
                position: Pos2::new(0.0, 0.0),
                input_values: vec![],
                is_enabled: true,
            },
            ClipboardNode {
                original_id: "b".to_string(),
                node_type: AddNodeType::Operation(Operation::OpNumberInputInteger),
                position: Pos2::new(200.0, 100.0),
                input_values: vec![],
                is_enabled: true,
            },
        ],
        connections: vec![],
    };
    let c = cb.centroid();
    assert!((c.x - 100.0).abs() < 0.001);
    assert!((c.y - 50.0).abs() < 0.001);
}

#[test]
fn test_from_selection_excludes_image_values() {
    // Image values should be excluded from the clipboard to avoid memory bloat.
    use mangler_core::float_image::FloatImage;
    use std::sync::Arc;

    let mut graph_nodes = HashMap::new();
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0]));
    let node = make_node(
        "a",
        Pos2::ZERO,
        vec![
            make_input("num", Value::Integer(5)),
            make_input("img", Value::Image { data: img, change_id: "0".to_string() }),
        ],
        vec![],
    );
    graph_nodes.insert("a".to_string(), node);

    let mut selected = HashSet::new();
    selected.insert("a".to_string());

    let cb = Clipboard::from_selection(&selected, &graph_nodes).unwrap();
    // Only the integer input should be captured, not the image.
    assert_eq!(cb.nodes[0].input_values.len(), 1);
    assert_eq!(cb.nodes[0].input_values[0].0, 0); // index 0 = "num"
}

#[test]
fn test_from_selection_nonexistent_node_id() {
    // Selecting a node ID that doesn't exist in the graph should be handled gracefully.
    let graph_nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut selected = HashSet::new();
    selected.insert("nonexistent".to_string());

    assert!(Clipboard::from_selection(&selected, &graph_nodes).is_none());
}

#[test]
fn test_clipboard_clone() {
    // Clipboard should be cloneable (needed for paste to work with a borrowed clipboard).
    let cb = Clipboard {
        nodes: vec![ClipboardNode {
            original_id: "a".to_string(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputInteger),
            position: Pos2::new(10.0, 20.0),
            input_values: vec![(0, Value::Integer(7))],
            is_enabled: true,
        }],
        connections: vec![ClipboardConnection {
            output_node_id: "a".to_string(),
            output_index: 0,
            input_node_id: "b".to_string(),
            input_index: 0,
        }],
    };

    let cb2 = cb.clone();
    assert_eq!(cb2.nodes.len(), 1);
    assert_eq!(cb2.connections.len(), 1);
    assert_eq!(cb2.nodes[0].original_id, "a");
}
