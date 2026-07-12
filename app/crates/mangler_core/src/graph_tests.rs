use tokio::sync::mpsc;

use crate::{
    get_id, graph::Graph, operations::Operation, value::Value, AddNodeType,
    GraphChangedMessage, NodeChangedMessage,
};

fn create_test_graph() -> Graph {
    let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(32);
    Graph::new(get_id(), tx_node_changed, tx_graph_changed, false).unwrap()
}

#[tokio::test]
async fn test_graph_new() {
    let graph = create_test_graph();
    assert!(graph.nodes.is_empty());
    assert!(!graph.is_subgraph);
}

#[tokio::test]
async fn test_add_node() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    assert!(graph.nodes.contains_key(&node_id));

    let node = graph.nodes.get(&node_id).unwrap();
    assert_eq!(node.inputs.len(), 2); // a, b
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.settings.name, "add");
}

#[tokio::test]
async fn disconnecting_restores_input_value_type() {
    use crate::value::ValueType;
    let mut graph = create_test_graph();
    let int_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    // The add node's first input is declared as a Decimal.
    assert_eq!(graph.nodes.get(&add_id).unwrap().inputs[0].value.value_type(), ValueType::Decimal);

    // Connecting an Integer source coerces the input's stored value to Integer
    // (the source value is propagated into the input for display).
    graph.add_connection(add_id.clone(), 0, int_id.clone(), 0).await;
    assert_eq!(graph.nodes.get(&add_id).unwrap().inputs[0].value.value_type(), ValueType::Integer);

    // Disconnecting must restore the input's declared type so its editing
    // widget (a Decimal slider/drag) comes back instead of staying an Integer.
    graph.remove_connection(add_id.clone(), 0).await;
    assert_eq!(graph.nodes.get(&add_id).unwrap().inputs[0].value.value_type(), ValueType::Decimal);
}

#[tokio::test]
async fn test_add_decimal_input_node() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.settings.name, "decimal");
}

#[tokio::test]
async fn test_remove_node() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    assert!(graph.nodes.contains_key(&node_id));
    graph.remove_node(node_id.clone()).await;
    assert!(!graph.nodes.contains_key(&node_id));
}

#[tokio::test]
async fn test_set_input() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    graph.set_input(node_id.clone(), 0, Value::Decimal(42.0));

    let node = graph.nodes.get(&node_id).unwrap();
    match &node.inputs[0].value {
        Value::Decimal(v) => assert_eq!(*v, 42.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
    assert!(node.is_dirty);
}

#[tokio::test]
async fn test_add_connection() {
    let mut graph = create_test_graph();

    let decimal_node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::new(0.0, 0.0), true, None,
        Vec::new())
        .await;

    let add_node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(200.0, 0.0), true, None,
        Vec::new())
        .await;

    // Connect decimal output 0 -> add input 0
    graph
        .add_connection(add_node_id.clone(), 0, decimal_node_id.clone(), 0)
        .await;

    // Verify input side
    let add_node = graph.nodes.get(&add_node_id).unwrap();
    assert!(add_node.inputs[0].connection.is_some());
    let (conn_node_id, conn_output_idx) = add_node.inputs[0].connection.as_ref().unwrap();
    assert_eq!(conn_node_id, &decimal_node_id);
    assert_eq!(*conn_output_idx, 0);

    // Verify output side
    let decimal_node = graph.nodes.get(&decimal_node_id).unwrap();
    assert!(decimal_node.outputs[0].connection.is_some());
}

#[tokio::test]
async fn test_run_single_node() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));

    graph.run().await;

    let node = graph.nodes.get(&node_id).unwrap();
    match &node.outputs[0].value {
        Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-6, "Expected 15.0, got {}", v),
        other => panic!("Expected Decimal output, got {:?}", other),
    }
}

#[tokio::test]
async fn test_run_connected_nodes() {
    let mut graph = create_test_graph();

    // Create two decimal input nodes
    let input_a_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::new(0.0, 0.0), true, None,
        Vec::new())
        .await;
    let input_b_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::new(0.0, 100.0), true, None,
        Vec::new())
        .await;

    // Create add node
    let add_node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(200.0, 0.0), true, None,
        Vec::new())
        .await;

    // Set input values
    graph.set_input(input_a_id.clone(), 0, Value::Decimal(7.0));
    graph.set_input(input_b_id.clone(), 0, Value::Decimal(3.0));

    // Connect: input_a output 0 -> add input 0
    graph
        .add_connection(add_node_id.clone(), 0, input_a_id.clone(), 0)
        .await;
    // Connect: input_b output 0 -> add input 1
    graph
        .add_connection(add_node_id.clone(), 1, input_b_id.clone(), 0)
        .await;

    graph.run().await;

    let add_node = graph.nodes.get(&add_node_id).unwrap();
    match &add_node.outputs[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6, "Expected 10.0, got {}", v),
        other => panic!("Expected Decimal output, got {:?}", other),
    }
}

#[tokio::test]
async fn test_set_node_position() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    graph.set_node_position(node_id.clone(), glam::Vec2::new(100.0, 200.0));

    let node = graph.nodes.get(&node_id).unwrap();
    assert_eq!(node.position, glam::Vec2::new(100.0, 200.0));
}

#[tokio::test]
async fn test_multiple_nodes_multiple_types() {
    let mut graph = create_test_graph();

    // Integer + Integer through add
    let add_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    graph.set_input(add_id.clone(), 0, Value::Integer(100));
    graph.set_input(add_id.clone(), 1, Value::Integer(200));

    graph.run().await;

    let node = graph.nodes.get(&add_id).unwrap();
    match &node.outputs[0].value {
        Value::Integer(v) => assert_eq!(*v, 300),
        other => panic!("Expected Integer output, got {:?}", other),
    }
}

// === new() edge cases ===

#[tokio::test]
async fn test_graph_new_subgraph() {
    let (tx_graph_changed, _rx) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_node_changed, _rx) = mpsc::channel::<NodeChangedMessage>(32);
    let graph = Graph::new(get_id(), tx_node_changed, tx_graph_changed, true).unwrap();
    assert!(graph.is_subgraph);
    assert!(graph.save_path.is_none());
    assert_eq!(graph.name, "new graph");
}

// === remove_connection ===

#[tokio::test]
async fn test_remove_connection() {
    let mut graph = create_test_graph();

    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

    // Verify connection exists
    assert!(graph.nodes.get(&add_id).unwrap().inputs[0].connection.is_some());

    // Remove it
    graph.remove_connection(add_id.clone(), 0).await;

    // Input side cleared
    assert!(graph.nodes.get(&add_id).unwrap().inputs[0].connection.is_none());

    // Output side cleared
    let decimal_node = graph.nodes.get(&decimal_id).unwrap();
    let conns = decimal_node.outputs[0].connection.as_ref();
    assert!(conns.is_none() || conns.unwrap().is_empty());
}

// === remove_node with connections ===

#[tokio::test]
async fn test_remove_node_cleans_up_connections() {
    let mut graph = create_test_graph();

    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

    // Remove the decimal node (has outgoing connection to add)
    graph.remove_node(decimal_id.clone()).await;

    assert!(!graph.nodes.contains_key(&decimal_id));
    // The add node's input connection should be cleaned up
    let add_node = graph.nodes.get(&add_id).unwrap();
    assert!(add_node.inputs[0].connection.is_none());
}

#[tokio::test]
async fn test_remove_connected_downstream_node() {
    let mut graph = create_test_graph();

    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

    // Remove the downstream add node
    graph.remove_node(add_id.clone()).await;

    assert!(!graph.nodes.contains_key(&add_id));
    // The decimal node's output connection should be cleaned up
    let decimal_node = graph.nodes.get(&decimal_id).unwrap();
    let conns = decimal_node.outputs[0].connection.as_ref();
    assert!(conns.is_none() || conns.unwrap().is_empty());
}

// === add_connection edge cases ===

#[tokio::test]
async fn test_add_connection_nonexistent_input_node() {
    let mut graph = create_test_graph();
    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // Try to connect to a node that doesn't exist — should be a no-op
    graph.add_connection("nonexistent".to_string(), 0, decimal_id.clone(), 0).await;

    // decimal node output should have no connection
    let decimal_node = graph.nodes.get(&decimal_id).unwrap();
    assert!(decimal_node.outputs[0].connection.is_none());
}

#[tokio::test]
async fn test_add_connection_nonexistent_output_node() {
    let mut graph = create_test_graph();
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    graph.add_connection(add_id.clone(), 0, "nonexistent".to_string(), 0).await;

    let add_node = graph.nodes.get(&add_id).unwrap();
    assert!(add_node.inputs[0].connection.is_none());
}

// === set_input edge cases ===

#[tokio::test]
async fn test_set_input_nonexistent_node() {
    let mut graph = create_test_graph();
    // Should be a no-op, not panic
    graph.set_input("nonexistent".to_string(), 0, Value::Decimal(1.0));
    assert!(graph.nodes.is_empty());
}

#[tokio::test]
async fn test_set_input_out_of_bounds_index() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // Add node has 2 inputs (indices 0, 1). Index 99 should be a no-op.
    graph.set_input(node_id.clone(), 99, Value::Decimal(1.0));

    // Node should still have original values
    let node = graph.nodes.get(&node_id).unwrap();
    assert_eq!(node.inputs.len(), 2);
}

// === set_node_position edge cases ===

#[tokio::test]
async fn test_set_position_nonexistent_node() {
    let mut graph = create_test_graph();
    // Should be a no-op, not panic
    graph.set_node_position("nonexistent".to_string(), glam::Vec2::new(100.0, 200.0));
}

// === set_save_path ===

#[test]
fn test_set_save_path() {
    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

    assert!(graph.save_path.is_none());
    graph.set_save_path(std::path::PathBuf::from("/tmp/test.mangler.json"));
    assert_eq!(graph.save_path, Some(std::path::PathBuf::from("/tmp/test.mangler.json")));
}

// === run() edge cases ===

#[tokio::test]
async fn test_run_empty_graph() {
    let mut graph = create_test_graph();
    // Should return immediately, not panic
    graph.run().await;
    assert!(graph.nodes.is_empty());
}

#[tokio::test]
async fn test_run_clean_graph_no_dirty_nodes() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    graph.set_input(node_id.clone(), 0, Value::Decimal(1.0));
    graph.set_input(node_id.clone(), 1, Value::Decimal(2.0));
    graph.run().await;

    // After run, nodes are no longer dirty. Running again should be a no-op.
    let output_before = match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
        Value::Decimal(v) => *v,
        _ => panic!("Expected Decimal"),
    };

    graph.run().await;

    let output_after = match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
        Value::Decimal(v) => *v,
        _ => panic!("Expected Decimal"),
    };

    assert_eq!(output_before, output_after);
}

#[tokio::test]
async fn test_run_caching_same_inputs() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));
    graph.run().await;

    // Set same values again — should use cache
    graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));
    graph.run().await;

    match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_run_cache_invalidation_on_changed_input() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));
    graph.run().await;

    // Change one input — should invalidate cache and recompute
    graph.set_input(node_id.clone(), 1, Value::Decimal(20.0));
    graph.run().await;

    match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 25.0).abs() < 1e-6, "Expected 25.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

// === run() with chains and fan-out ===

#[tokio::test]
async fn test_run_three_node_chain() {
    let mut graph = create_test_graph();

    // decimal(5) → add(_, 10) → add(_, 100)
    let input_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add1_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;
    let add2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(400.0, 0.0), true, None, Vec::new())
        .await;

    graph.set_input(input_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(add1_id.clone(), 1, Value::Decimal(10.0));
    graph.set_input(add2_id.clone(), 1, Value::Decimal(100.0));

    graph.add_connection(add1_id.clone(), 0, input_id.clone(), 0).await;
    graph.add_connection(add2_id.clone(), 0, add1_id.clone(), 0).await;

    graph.run().await;

    // 5 + 10 = 15, then 15 + 100 = 115
    match &graph.nodes.get(&add2_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 115.0).abs() < 1e-6, "Expected 115.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_run_fan_out() {
    let mut graph = create_test_graph();

    // decimal(10) → add1(_, 1) and add2(_, 2)
    let input_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add1_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;
    let add2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 100.0), true, None, Vec::new())
        .await;

    graph.set_input(input_id.clone(), 0, Value::Decimal(10.0));
    graph.set_input(add1_id.clone(), 1, Value::Decimal(1.0));
    graph.set_input(add2_id.clone(), 1, Value::Decimal(2.0));

    // Same output feeds both add nodes
    graph.add_connection(add1_id.clone(), 0, input_id.clone(), 0).await;
    graph.add_connection(add2_id.clone(), 0, input_id.clone(), 0).await;

    graph.run().await;

    match &graph.nodes.get(&add1_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 11.0).abs() < 1e-6, "Expected 11.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
    match &graph.nodes.get(&add2_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 12.0).abs() < 1e-6, "Expected 12.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_run_value_propagation_through_connection() {
    let mut graph = create_test_graph();

    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.set_input(decimal_id.clone(), 0, Value::Decimal(42.0));
    graph.set_input(add_id.clone(), 1, Value::Decimal(0.0));
    graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

    graph.run().await;

    // The add node's input 0 should have received the propagated value
    match &graph.nodes.get(&add_id).unwrap().inputs[0].value {
        Value::Decimal(v) => assert!((*v - 42.0).abs() < 1e-6, "Expected propagated 42.0, got {}", v),
        other => panic!("Expected Decimal input, got {:?}", other),
    }
}

// === save_to_file / load round-trip ===

#[tokio::test]
async fn test_save_and_load_round_trip() {
    let mut graph = create_test_graph();
    let graph_id = graph.id.clone();

    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(50.0, 75.0), true, None, Vec::new())
        .await;
    graph.set_input(node_id.clone(), 0, Value::Decimal(42.0));

    let tmp_path = std::env::temp_dir().join(format!("test_graph_{}.mangler.json", get_id()));
    graph.set_save_path(tmp_path.clone());
    graph.save_to_file().unwrap();

    // Load it back
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let loaded = Graph::load(tmp_path.clone(), Some(tx_nc), Some(tx_gc), false).unwrap();

    assert_eq!(loaded.id, graph_id);
    assert!(loaded.nodes.contains_key(&node_id));
    let loaded_node = loaded.nodes.get(&node_id).unwrap();
    assert_eq!(loaded_node.settings.name, "add");
    assert_eq!(loaded_node.position, glam::Vec2::new(50.0, 75.0));
    match &loaded_node.inputs[0].value {
        Value::Decimal(v) => assert_eq!(*v, 42.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }

    // Clean up
    let _ = std::fs::remove_file(tmp_path);
}

// save_to_file serializes through a borrowing mirror of GraphSaveData
// (GraphSaveRef); this guards the mirror staying in sync by checking the
// written JSON carries the app version stamp.
#[tokio::test]
async fn test_save_to_file_stamps_app_version() {
    let mut graph = create_test_graph();

    let tmp_path = std::env::temp_dir().join(format!("test_version_{}.mangler.json", get_id()));
    graph.set_save_path(tmp_path.clone());
    graph.save_to_file().unwrap();

    let raw = std::fs::read_to_string(&tmp_path).unwrap();
    let saved: crate::GraphSaveData = serde_json::from_str(&raw).unwrap();
    assert_eq!(saved.version, crate::APP_VERSION);

    // Clean up
    let _ = std::fs::remove_file(tmp_path);
}

#[tokio::test]
async fn test_save_to_file_subgraph_is_noop() {
    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, true).unwrap();

    let tmp_path = std::env::temp_dir().join(format!("test_subgraph_{}.mangler.json", get_id()));
    graph.set_save_path(tmp_path.clone());
    assert_eq!(graph.save_to_file(), Ok(()));

    // File should NOT be created for subgraphs
    assert!(!tmp_path.exists());
}

#[tokio::test]
async fn test_save_to_file_no_path_is_noop() {
    let mut graph = create_test_graph();
    assert!(graph.save_path.is_none());
    // Should be a no-op, not panic
    assert_eq!(graph.save_to_file(), Ok(()));
}

// === load() error cases ===

#[test]
fn test_load_nonexistent_file() {
    let result = Graph::load(
        std::path::PathBuf::from("/nonexistent/path/graph.mangler.json"),
        None, None, false,
    );
    assert!(result.is_err());
}

#[test]
fn test_load_invalid_json() {
    let tmp_path = std::env::temp_dir().join(format!("test_bad_json_{}.mangler.json", get_id()));
    std::fs::write(&tmp_path, "this is not valid json").unwrap();

    let result = Graph::load(tmp_path.clone(), None, None, false);
    assert!(result.is_err());

    let _ = std::fs::remove_file(tmp_path);
}

// === remove_node on nonexistent node ===

#[tokio::test]
async fn test_remove_nonexistent_node() {
    let mut graph = create_test_graph();
    // Should be a no-op, not panic
    graph.remove_node("nonexistent".to_string()).await;
    assert!(graph.nodes.is_empty());
}

// === remove_connection on unconnected input ===

#[tokio::test]
async fn test_remove_connection_when_none_exists() {
    let mut graph = create_test_graph();
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // Input 0 has no connection — should be a no-op, not panic
    graph.remove_connection(add_id.clone(), 0).await;

    let add_node = graph.nodes.get(&add_id).unwrap();
    assert!(add_node.inputs[0].connection.is_none());
}

// === add multiple nodes, remove all ===

#[tokio::test]
async fn test_add_and_remove_multiple_nodes() {
    let mut graph = create_test_graph();

    let id1 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new()).await;
    let id2 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO, true, None, Vec::new()).await;
    let id3 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new()).await;

    assert_eq!(graph.nodes.len(), 3);

    graph.remove_node(id1).await;
    graph.remove_node(id2).await;
    graph.remove_node(id3).await;

    assert_eq!(graph.nodes.len(), 0);
}

// === run() propagates updated upstream value downstream ===

// === accepts_any_type adaptation on connect/disconnect ===

#[tokio::test]
async fn test_connect_adapts_select_inputs_and_output_to_source_type() {
    // When an Integer output is connected to a select node's "if true" input,
    // both accepts_any_type inputs and the output should adapt to Integer.
    let mut graph = create_test_graph();

    let int_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    // Before connection: select inputs default to Decimal
    let select_node = graph.nodes.get(&select_id).unwrap();
    assert!(matches!(select_node.inputs[1].value, Value::Decimal(_)), "if_true should start as Decimal");
    assert!(matches!(select_node.inputs[2].value, Value::Decimal(_)), "if_false should start as Decimal");
    assert!(matches!(select_node.outputs[0].value, Value::Decimal(_)), "output should start as Decimal");

    // Connect integer output -> select "if true" (index 1)
    graph.add_connection(select_id.clone(), 1, int_id.clone(), 0).await;

    // After connection: all accepts_any_type inputs and outputs should be Integer
    let select_node = graph.nodes.get(&select_id).unwrap();
    assert!(matches!(select_node.inputs[1].value, Value::Integer(_)), "if_true should adapt to Integer");
    assert!(matches!(select_node.inputs[2].value, Value::Integer(_)), "if_false should adapt to Integer");
    assert!(matches!(select_node.outputs[0].value, Value::Integer(_)), "output should adapt to Integer");

    // condition input (index 0) should remain Bool — it is not accepts_any_type
    assert!(matches!(select_node.inputs[0].value, Value::Bool(_)), "condition should stay Bool");
}

#[tokio::test]
async fn test_disconnect_only_connection_reverts_to_decimal() {
    // When the only connection to a select node is removed, the accepts_any_type
    // inputs and outputs should revert to their default Decimal type.
    let mut graph = create_test_graph();

    let int_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    // Connect and then disconnect
    graph.add_connection(select_id.clone(), 1, int_id.clone(), 0).await;
    graph.remove_connection(select_id.clone(), 1).await;

    // Should revert to Decimal
    let select_node = graph.nodes.get(&select_id).unwrap();
    assert!(matches!(select_node.inputs[1].value, Value::Decimal(_)), "if_true should revert to Decimal");
    assert!(matches!(select_node.inputs[2].value, Value::Decimal(_)), "if_false should revert to Decimal");
    assert!(matches!(select_node.outputs[0].value, Value::Decimal(_)), "output should revert to Decimal");
}

#[tokio::test]
async fn test_disconnect_one_of_two_keeps_remaining_type() {
    // When two connections exist and one is removed, the types should stay
    // adapted to the remaining connection's source type.
    let mut graph = create_test_graph();

    let int1_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let int2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::new(0.0, 100.0), true, None, Vec::new())
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    // Connect both branch inputs to integer sources
    graph.add_connection(select_id.clone(), 1, int1_id.clone(), 0).await;
    graph.add_connection(select_id.clone(), 2, int2_id.clone(), 0).await;

    // Remove the "if true" connection (index 1)
    graph.remove_connection(select_id.clone(), 1).await;

    // "if false" still connected to Integer, so types should remain Integer
    let select_node = graph.nodes.get(&select_id).unwrap();
    assert!(matches!(select_node.inputs[2].value, Value::Integer(_)), "if_false should stay Integer (still connected)");
    assert!(matches!(select_node.outputs[0].value, Value::Integer(_)), "output should stay Integer");
    // The disconnected input should also match the remaining type
    assert!(matches!(select_node.inputs[1].value, Value::Integer(_)), "if_true should match remaining Integer type");
}

#[tokio::test]
async fn test_disconnect_typed_input_resets_and_marks_dirty() {
    // Removing a connection to a normal (same-typed) input must reset the input
    // to its default value AND mark the node dirty, so it re-runs and refreshes
    // its output/thumbnail. Regression: deleting a node feeding e.g. a blend node
    // left the downstream node holding the stale value and never re-running,
    // because the reset/dirty path only fired on a type *mismatch*.
    let mut graph = create_test_graph();

    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

    // Simulate a value having been propagated in, and a prior run having cached
    // the input hash and cleared the dirty flag.
    {
        let add = graph.nodes.get_mut(&add_id).unwrap();
        add.inputs[0].value = Value::Decimal(42.0);
        add.is_dirty = false;
        add.cached_input_hash = Some(123);
    }

    graph.remove_connection(add_id.clone(), 0).await;

    let add = graph.nodes.get(&add_id).unwrap();
    let default = match add.inputs[0].default_value {
        Value::Decimal(d) => d,
        ref other => panic!("expected Decimal default, got {other:?}"),
    };
    assert!(
        matches!(add.inputs[0].value, Value::Decimal(v) if v == default),
        "disconnected input should reset to its default value, got {:?}",
        add.inputs[0].value,
    );
    assert!(add.is_dirty, "node must be marked dirty so it re-runs after disconnect");
    assert!(add.cached_input_hash.is_none(), "cached input hash must be cleared");
}

#[tokio::test]
async fn test_connect_to_condition_does_not_adapt_types() {
    // Connecting to the condition input (index 0) should NOT trigger type
    // adaptation since condition is not accepts_any_type.
    let mut graph = create_test_graph();

    let bool_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicInputBool), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(select_id.clone(), 0, bool_id.clone(), 0).await;

    // Branch inputs and output should remain Decimal (unchanged)
    let select_node = graph.nodes.get(&select_id).unwrap();
    assert!(matches!(select_node.inputs[1].value, Value::Decimal(_)), "if_true should stay Decimal");
    assert!(matches!(select_node.inputs[2].value, Value::Decimal(_)), "if_false should stay Decimal");
    assert!(matches!(select_node.outputs[0].value, Value::Decimal(_)), "output should stay Decimal");
}

#[tokio::test]
async fn test_select_run_after_type_adaptation() {
    // End-to-end: connect integer sources, run the graph, verify the select
    // node correctly forwards the chosen integer value.
    let mut graph = create_test_graph();

    let bool_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicInputBool), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let int_true_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::new(0.0, 100.0), true, None, Vec::new())
        .await;
    let int_false_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::new(0.0, 200.0), true, None, Vec::new())
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    // Set source values
    graph.set_input(bool_id.clone(), 0, Value::Bool(true));
    graph.set_input(int_true_id.clone(), 0, Value::Integer(42));
    graph.set_input(int_false_id.clone(), 0, Value::Integer(99));

    // Wire up: condition, if_true, if_false
    graph.add_connection(select_id.clone(), 0, bool_id.clone(), 0).await;
    graph.add_connection(select_id.clone(), 1, int_true_id.clone(), 0).await;
    graph.add_connection(select_id.clone(), 2, int_false_id.clone(), 0).await;

    graph.run().await;

    // condition is true, so output should be 42
    match &graph.nodes.get(&select_id).unwrap().outputs[0].value {
        Value::Integer(v) => assert_eq!(*v, 42, "Expected 42, got {}", v),
        other => panic!("Expected Integer output, got {:?}", other),
    }

    // Flip condition to false and re-run
    graph.set_input(bool_id.clone(), 0, Value::Bool(false));
    graph.run().await;

    match &graph.nodes.get(&select_id).unwrap().outputs[0].value {
        Value::Integer(v) => assert_eq!(*v, 99, "Expected 99, got {}", v),
        other => panic!("Expected Integer output, got {:?}", other),
    }
}

// === run() propagates updated upstream value downstream ===

#[tokio::test]
async fn test_run_upstream_change_propagates() {
    let mut graph = create_test_graph();

    let input_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.set_input(input_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(add_id.clone(), 1, Value::Decimal(10.0));
    graph.add_connection(add_id.clone(), 0, input_id.clone(), 0).await;

    graph.run().await;

    match &graph.nodes.get(&add_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }

    // Change the upstream input
    graph.set_input(input_id.clone(), 0, Value::Decimal(100.0));
    graph.run().await;

    match &graph.nodes.get(&add_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 110.0).abs() < 1e-6, "Expected 110.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

/// Reconnecting an input to a different source must remove the stale entry
/// from the old source's output connection list. Otherwise the old source
/// continues to propagate its value into the input during graph execution,
/// overwriting the value from the new source.
#[tokio::test]
async fn test_reconnect_input_cleans_up_old_output_connection() {
    let mut graph = create_test_graph();

    // Create three decimal-input nodes (source_a, source_b) and an add node (consumer).
    let source_a = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let source_b = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let consumer = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // Connect source_a output 0 → consumer input 0
    graph.add_connection(consumer.clone(), 0, source_a.clone(), 0).await;

    // Verify source_a output 0 lists the consumer connection
    let conns_a = graph.nodes.get(&source_a).unwrap().outputs[0].connection.as_ref().unwrap();
    assert!(conns_a.contains(&(consumer.clone(), 0)));

    // Now reconnect: source_b output 0 → consumer input 0 (replacing source_a)
    graph.add_connection(consumer.clone(), 0, source_b.clone(), 0).await;

    // The consumer's input should now point to source_b
    let (conn_id, conn_idx) = graph.nodes.get(&consumer).unwrap().inputs[0].connection.as_ref().unwrap();
    assert_eq!(conn_id, &source_b);
    assert_eq!(*conn_idx, 0);

    // source_b output 0 should list the consumer
    let conns_b = graph.nodes.get(&source_b).unwrap().outputs[0].connection.as_ref().unwrap();
    assert!(conns_b.contains(&(consumer.clone(), 0)));

    // source_a output 0 must NO LONGER list the consumer (stale entry cleaned up)
    let conns_a_after = graph.nodes.get(&source_a).unwrap().outputs[0].connection.as_ref();
    let has_stale = conns_a_after
        .map(|c| c.contains(&(consumer.clone(), 0)))
        .unwrap_or(false);
    assert!(!has_stale, "Old source still has a stale output connection after reconnect");
}

/// After reconnecting an input, only the new source's value should propagate
/// during graph execution — the old source must not overwrite it.
#[tokio::test]
async fn test_reconnect_input_propagates_correct_value() {
    let mut graph = create_test_graph();

    let source_a = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let source_b = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let consumer = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // source_a = 10, source_b = 42
    graph.set_input(source_a.clone(), 0, Value::Decimal(10.0));
    graph.set_input(source_b.clone(), 0, Value::Decimal(42.0));

    // Connect source_a → consumer input 0, leave input 1 at default (0)
    graph.add_connection(consumer.clone(), 0, source_a.clone(), 0).await;
    graph.run().await;

    // consumer = 10 + 0 = 10
    match &graph.nodes.get(&consumer).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6, "Expected 10.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }

    // Reconnect: source_b → consumer input 0
    graph.add_connection(consumer.clone(), 0, source_b.clone(), 0).await;
    // Mark source_a dirty so it runs and would propagate if stale connection exists
    graph.set_input(source_a.clone(), 0, Value::Decimal(999.0));
    graph.run().await;

    // consumer should use source_b (42), not source_a (999)
    // consumer = 42 + 0 = 42
    match &graph.nodes.get(&consumer).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 42.0).abs() < 1e-6, "Expected 42.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

// === add_node with is_enabled and custom_name ===

#[tokio::test]
async fn test_add_node_disabled() {
    // Adding a node with is_enabled=false should create a disabled node.
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, false, None,
        Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    assert!(!node.is_enabled);
    assert!(node.custom_name.is_none());
}

#[tokio::test]
async fn test_add_node_with_custom_name() {
    // Adding a node with a custom name should store it on the node.
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, true, Some("my add node".to_string()),
        Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    assert!(node.is_enabled);
    assert_eq!(node.custom_name.as_deref(), Some("my add node"));
}

#[tokio::test]
async fn test_add_node_with_disabled_and_custom_name() {
    // Adding a node with both disabled and a custom name.
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO, false, Some("constants".to_string()),
        Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    assert!(!node.is_enabled);
    assert_eq!(node.custom_name.as_deref(), Some("constants"));
}

#[tokio::test]
async fn test_add_node_defaults() {
    // Adding a node with is_enabled=true and custom_name=None should match default behavior.
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    assert!(node.is_enabled);
    assert!(node.custom_name.is_none());
}

// End-to-end subgraph integration test.
//
// Builds a tiny child graph (a single decimal passthrough with exposed I/O),
// writes it to disk, loads it into a parent graph via a Subgraph node, drives
// the parent's exposed input, runs the parent, and verifies the value flowed
// all the way back out through the parent's exposed output.
#[tokio::test]
async fn test_subgraph_propagates_value_end_to_end() {
    use std::fs;
    use crate::GraphSaveData;

    // Build a child graph containing one decimal passthrough node, with its
    // input and output both marked exposed so the parent can surface them.
    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();

    let child_node_id = child
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO,
            true,
            None,
        Vec::new())
        .await;

    {
        let node = child.nodes.get_mut(&child_node_id).unwrap();
        node.inputs[0].is_exposed = true;
        node.outputs[0].is_exposed = true;
    }

    // Persist the child graph to a unique tempfile.
    let tmp_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_int_test_{}.mangler.json", get_id()));
    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&tmp_path, serde_json::to_string(&save_data).unwrap())
        .expect("failed to write child graph tempfile");

    // Build the parent graph and add an empty subgraph node.
    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(
            get_id(),
            AddNodeType::Subgraph,
            glam::Vec2::ZERO,
            true,
            None,
        Vec::new())
        .await;

    // Load the child graph via the dedicated API. The parent node's
    // inputs/outputs are populated from the child's exposed slots.
    parent.set_subgraph_path(subgraph_node_id.clone(), tmp_path.clone());

    // After load the parent surfaces just the exposed child input and output.
    {
        let parent_node = parent.nodes.get(&subgraph_node_id).unwrap();
        assert_eq!(
            parent_node.inputs.len(),
            1,
            "exposed child input should surface as a parent input"
        );
        assert_eq!(
            parent_node.outputs.len(),
            1,
            "exposed child output should surface as a parent output"
        );
        assert!(
            parent_node.inputs[0].link.is_some(),
            "exposed parent input must be linked back to the child node"
        );
    }

    // Drive the exposed input (index 0 now — no more synthetic file path slot).
    parent.set_input(subgraph_node_id.clone(), 0, Value::Decimal(42.0));

    parent.run().await;

    // The parent subgraph node's output should reflect the value that
    // passed through the child decimal node.
    let parent_node = parent.nodes.get(&subgraph_node_id).unwrap();
    match &parent_node.outputs[0].value {
        Value::Decimal(v) => assert!(
            (*v - 42.0).abs() < 1e-6,
            "expected 42.0 out of subgraph, got {}",
            v
        ),
        other => panic!("expected Decimal output from subgraph, got {:?}", other),
    }

    let _ = fs::remove_file(&tmp_path);
}

// Regression test: exposed outputs of a *many-node* subgraph must not be
// dropped. Results used to round-trip through a bounded (32-slot) mpsc
// channel drained with try_recv after the child ran; every child node emits
// several messages per run (busy on/off, info, output-changed), so a child
// graph with ~10 nodes overflowed the channel and the tail OutputChanged
// messages — including the exposed output, which runs last in a chain — were
// silently dropped, leaving the parent's exposed output stale. Outputs are
// now read directly from the child graph's node storage, which is lossless.
//
// Child graph: decimal input -> 9 chained increments (10 nodes total). The
// decimal's input is exposed, the last increment's output is exposed, so the
// expected exposed output is input + 9. Runs twice to verify repeated runs
// stay correct.
#[tokio::test]
async fn test_subgraph_many_nodes_exposed_output_not_dropped() {
    use std::fs;
    use crate::GraphSaveData;

    // Build the child graph: one decimal input node feeding a chain of
    // nine increment nodes.
    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();

    let decimal_id = child
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO,
            true,
            None,
        Vec::new())
        .await;

    let mut previous_id = decimal_id.clone();
    let mut last_increment_id = String::new();
    for _ in 0..9 {
        let increment_id = child
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathIncrement),
                glam::Vec2::ZERO,
                true,
                None,
            Vec::new())
            .await;
        child
            .add_connection(increment_id.clone(), 0, previous_id.clone(), 0)
            .await;
        previous_id = increment_id.clone();
        last_increment_id = increment_id;
    }

    // Expose the decimal's input and the final increment's output.
    child.nodes.get_mut(&decimal_id).unwrap().inputs[0].is_exposed = true;
    child
        .nodes
        .get_mut(&last_increment_id)
        .unwrap()
        .outputs[0]
        .is_exposed = true;

    // Persist the child graph to a unique tempfile.
    let tmp_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_overflow_test_{}.mangler.json", get_id()));
    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&tmp_path, serde_json::to_string(&save_data).unwrap())
        .expect("failed to write child graph tempfile");

    // Build the parent and attach the child.
    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), tmp_path.clone());

    {
        let parent_node = parent.nodes.get(&subgraph_node_id).unwrap();
        assert_eq!(parent_node.inputs.len(), 1, "one exposed input expected");
        assert_eq!(parent_node.outputs.len(), 1, "one exposed output expected");
    }

    // First run: 1.0 through nine increments must come back as 10.0.
    parent.set_input(subgraph_node_id.clone(), 0, Value::Decimal(1.0));
    parent.run().await;

    let parent_node = parent.nodes.get(&subgraph_node_id).unwrap();
    match &parent_node.outputs[0].value {
        Value::Decimal(v) => assert!(
            (*v - 10.0).abs() < 1e-6,
            "expected 10.0 from 10-node subgraph, got {} (exposed output dropped?)",
            v
        ),
        other => panic!("expected Decimal output from subgraph, got {:?}", other),
    }

    // Second run with a new input: repeated runs must stay correct.
    parent.set_input(subgraph_node_id.clone(), 0, Value::Decimal(100.0));
    parent.run().await;

    let parent_node = parent.nodes.get(&subgraph_node_id).unwrap();
    match &parent_node.outputs[0].value {
        Value::Decimal(v) => assert!(
            (*v - 109.0).abs() < 1e-6,
            "expected 109.0 on second run, got {}",
            v
        ),
        other => panic!("expected Decimal output from subgraph, got {:?}", other),
    }

    let _ = fs::remove_file(&tmp_path);
}

// A detached() snapshot must still execute its
// subgraph nodes. NodeType::clone drops the loaded child graph to
// None, so without rehydration the snapshot silently skips the subgraph and
// emits a stale/default output. This test drives an exposed input, snapshots
// the parent WITHOUT running it live, then runs only the snapshot and asserts
// the value flowed through the child — i.e. the subgraph really executed.
#[tokio::test]
async fn test_detached_snapshot_executes_subgraph() {
    use std::fs;
    use crate::GraphSaveData;

    // Build a child graph: one exposed decimal passthrough node, saved to disk.
    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();

    let child_node_id = child
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO,
            true,
            None,
        Vec::new())
        .await;

    {
        let node = child.nodes.get_mut(&child_node_id).unwrap();
        node.inputs[0].is_exposed = true;
        node.outputs[0].is_exposed = true;
    }

    let tmp_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_detached_test_{}.mangler.json", get_id()));
    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&tmp_path, serde_json::to_string(&save_data).unwrap())
        .expect("failed to write child graph tempfile");

    // Build the parent, attach the child, and drive the exposed input — but do
    // NOT run the parent. If we ran it, the live output would already hold the
    // value and get cloned into the snapshot, masking a skipped subgraph.
    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), tmp_path.clone());
    parent.set_input(subgraph_node_id.clone(), 0, Value::Decimal(42.0));

    // Sanity: before any run the surfaced output is still the child default,
    // so a later 42.0 can only come from actually executing the subgraph.
    {
        let parent_node = parent.nodes.get(&subgraph_node_id).unwrap();
        match &parent_node.outputs[0].value {
            Value::Decimal(v) => assert!(
                (*v - 42.0).abs() > 1e-6,
                "precondition: output should not already be 42.0"
            ),
            other => panic!("expected Decimal output, got {:?}", other),
        }
    }

    // Snapshot and run only the snapshot.
    let mut snapshot = parent.detached();
    snapshot.run().await;

    let snap_node = snapshot
        .nodes
        .get(&subgraph_node_id)
        .expect("subgraph node should survive detach");
    match &snap_node.outputs[0].value {
        Value::Decimal(v) => assert!(
            (*v - 42.0).abs() < 1e-6,
            "detached snapshot must execute the subgraph; expected 42.0, got {}",
            v
        ),
        other => panic!("expected Decimal output from detached subgraph, got {:?}", other),
    }

    let _ = fs::remove_file(&tmp_path);
}

// Graph::load restores Input.default_value, Output.value, and Output.default_value
// from each Operation node's create_inputs()/create_outputs(), since those fields
// are #[serde(skip)] and otherwise come back as Value::Bool(false).
#[tokio::test]
async fn test_load_restores_typed_input_and_output_defaults() {
    use std::fs;
    use crate::GraphSaveData;

    // Build a graph containing a Decimal input node, save it.
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO,
            true,
            None,
        Vec::new())
        .await;

    let tmp_path = std::env::temp_dir()
        .join(format!("mangler_load_defaults_test_{}.mangler.json", get_id()));
    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: graph.id.clone(),
        name: graph.name.clone(),
        nodes: graph.nodes.clone(),
    };
    fs::write(&tmp_path, serde_json::to_string(&save_data).unwrap())
        .expect("failed to write graph tempfile");

    // Load it back into a fresh Graph.
    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let loaded = Graph::load(tmp_path.clone(), Some(tx_nc), Some(tx_gc), false)
        .expect("failed to load graph");

    let node = loaded.nodes.get(&node_id).expect("node should round-trip");

    // Without the fix these would be Value::Bool(false) (Value::default()).
    match &node.inputs[0].default_value {
        Value::Decimal(_) => {}
        other => panic!("expected Decimal input default_value, got {:?}", other),
    }
    match &node.outputs[0].value {
        Value::Decimal(_) => {}
        other => panic!("expected Decimal output value, got {:?}", other),
    }
    match &node.outputs[0].default_value {
        Value::Decimal(_) => {}
        other => panic!("expected Decimal output default_value, got {:?}", other),
    }

    let _ = fs::remove_file(&tmp_path);
}

// Saving and reloading a parent graph that contains a Subgraph node should
// re-hydrate the child graph (since Subgraph.graph is #[serde(skip)]) using
// the path that survived serialization in NodeType::Subgraph.path.
#[tokio::test]
async fn test_load_graph_with_saved_subgraph_node_auto_reloads() {
    use std::fs;
    use crate::{GraphSaveData, node_type::NodeType};

    // Build a tiny child graph (exposed decimal passthrough) and save it.
    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();
    let child_node_id = child
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;
    {
        let n = child.nodes.get_mut(&child_node_id).unwrap();
        n.inputs[0].is_exposed = true;
        n.outputs[0].is_exposed = true;
    }
    let child_path = std::env::temp_dir()
        .join(format!("mangler_autoreload_child_{}.mangler.json", get_id()));
    let child_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&child_path, serde_json::to_string(&child_save).unwrap()).unwrap();

    // Build a parent graph, add a subgraph node pointing at the child, and save.
    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(
            get_id(),
            AddNodeType::Subgraph,
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    let parent_path = std::env::temp_dir()
        .join(format!("mangler_autoreload_parent_{}.mangler.json", get_id()));
    let parent_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: parent.id.clone(),
        name: parent.name.clone(),
        nodes: parent.nodes.clone(),
    };
    fs::write(&parent_path, serde_json::to_string(&parent_save).unwrap()).unwrap();

    // Load the parent fresh. The subgraph node should auto-reload its child.
    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut loaded_parent =
        Graph::load(parent_path.clone(), Some(tx_nc), Some(tx_gc), false)
            .expect("failed to load parent graph");

    let loaded_node = loaded_parent
        .nodes
        .get(&subgraph_node_id)
        .expect("subgraph node should round-trip");

    // The auto-reload should have repopulated inputs/outputs and restored the
    // child graph on the node_type.
    assert_eq!(loaded_node.inputs.len(), 1, "exposed input should auto-reload");
    assert_eq!(loaded_node.outputs.len(), 1, "exposed output should auto-reload");
    assert!(
        matches!(
            loaded_node.node_type,
            NodeType::Subgraph { graph: Some(_), .. }
        ),
        "child graph should be rehydrated on the subgraph node"
    );

    // End-to-end sanity check: drive the exposed input, run, assert output.
    loaded_parent.set_input(subgraph_node_id.clone(), 0, Value::Decimal(7.0));
    loaded_parent.run().await;
    let final_node = loaded_parent.nodes.get(&subgraph_node_id).unwrap();
    match &final_node.outputs[0].value {
        Value::Decimal(v) => assert!((*v - 7.0).abs() < 1e-6, "got {}", v),
        other => panic!("expected Decimal, got {:?}", other),
    }

    let _ = fs::remove_file(&child_path);
    let _ = fs::remove_file(&parent_path);
}

// Mirrors the GUI flow: add subgraph → pick file → run. No parent save/load
// cycle, because the user's reported bug happens before they ever save.
//
// Uses a distinctive RED color on the child so pixel-data assertions can prove
// the real image (red 64x64) propagated, not the default placeholder (white 1x1)
// or a same-sized default (black 64x64 from Color::default()).
#[tokio::test]
async fn test_subgraph_image_output_in_memory_flow() {
    use std::fs;
    use crate::GraphSaveData;
    use crate::color::Color;

    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();
    let child_node_id = child
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpImageInputColor),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;
    // Color = RED (1,0,0,1), 64x64 so pixel data is verifiable.
    child.set_input(child_node_id.clone(), 0, Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)));
    child.set_input(child_node_id.clone(), 1, Value::Integer(64));
    child.set_input(child_node_id.clone(), 2, Value::Integer(64));
    {
        let n = child.nodes.get_mut(&child_node_id).unwrap();
        n.outputs[0].is_exposed = true;
    }

    let child_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_image_inmem_child_{}.mangler.json", get_id()));
    let child_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&child_path, serde_json::to_string(&child_save).unwrap()).unwrap();

    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    parent.run().await;

    let node = parent.nodes.get(&subgraph_node_id).unwrap();
    match &node.outputs[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 64, "parent output width after run");
            assert_eq!(data.height(), 64, "parent output height after run");
            // Verify a center pixel is red, not white/black/something else.
            let px = data.get_pixel(32, 32);
            assert!(px.len() >= 3, "expected at least 3 channels, got {}", px.len());
            assert!((px[0] - 1.0).abs() < 0.01, "R channel should be 1.0, got {}", px[0]);
            assert!(px[1] < 0.01, "G channel should be 0.0, got {}", px[1]);
            assert!(px[2] < 0.01, "B channel should be 0.0, got {}", px[2]);
        }
        other => panic!("expected Image, got {:?}", other),
    }

    let _ = fs::remove_file(&child_path);
}

// Verifies that the OutputChanged MESSAGE sent through the parent's tx_node_changed
// channel (= the channel the GUI listens to) carries the real image value, not
// the 1x1 placeholder. The earlier tests checked in-memory state only.
#[tokio::test]
async fn test_subgraph_emits_output_changed_with_real_image_through_channel() {
    use std::fs;
    use crate::GraphSaveData;
    use crate::color::Color;

    // Build child with exposed red 64x64 image output.
    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();
    let child_node_id = child
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageInputColor), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    child.set_input(child_node_id.clone(), 0, Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)));
    child.set_input(child_node_id.clone(), 1, Value::Integer(64));
    child.set_input(child_node_id.clone(), 2, Value::Integer(64));
    {
        let n = child.nodes.get_mut(&child_node_id).unwrap();
        n.outputs[0].is_exposed = true;
    }

    let child_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_channel_child_{}.mangler.json", get_id()));
    let child_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&child_path, serde_json::to_string(&child_save).unwrap()).unwrap();

    // Build parent with a KEPT rx so we can inspect messages.
    let (parent_tx_nc, mut parent_rx_nc) = mpsc::channel::<NodeChangedMessage>(256);
    let (parent_tx_gc, _parent_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut parent = Graph::new(get_id(), parent_tx_nc, parent_tx_gc, false).unwrap();

    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    parent.run().await;

    // Drain messages until we find the OutputChanged for the subgraph node's
    // output[0]. It should carry a 64x64 red image, not a 1x1 placeholder.
    let mut found_real_image = false;
    while let Ok(msg) = parent_rx_nc.try_recv() {
        if let NodeChangedMessage::OutputChanged { node_id, output_index, value, .. } = msg {
            if node_id == subgraph_node_id && output_index == 0 {
                match value {
                    Value::Image { data, .. } => {
                        assert_eq!(data.width(), 64, "channel msg image width");
                        assert_eq!(data.height(), 64, "channel msg image height");
                        let px = data.get_pixel(32, 32);
                        assert!(px.len() >= 3);
                        assert!((px[0] - 1.0).abs() < 0.01, "R should be 1.0, got {}", px[0]);
                        assert!(px[1] < 0.01, "G should be 0.0");
                        assert!(px[2] < 0.01, "B should be 0.0");
                        found_real_image = true;
                        break;
                    }
                    other => panic!("expected Image in channel msg, got {:?}", other),
                }
            }
        }
    }
    assert!(found_real_image, "no OutputChanged for subgraph node's exposed output reached the parent's channel");

    let _ = fs::remove_file(&child_path);
}

// Reproduces user-reported issue: a subgraph whose exposed output is an image
// should propagate the real generated image to the parent after run, not the
// 1x1 white placeholder that `create_outputs()` returns.
#[tokio::test]
async fn test_subgraph_image_output_propagates_real_image() {
    use std::fs;
    use crate::GraphSaveData;

    // Build a child with a 64x64 from-color image node whose output is exposed.
    let (child_tx_nc, _child_rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (child_tx_gc, _child_rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), child_tx_nc, child_tx_gc, true).unwrap();
    let child_node_id = child
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpImageInputColor),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;
    // width = 64, height = 64 so the real image is clearly not the 1x1 placeholder.
    child.set_input(child_node_id.clone(), 1, Value::Integer(64));
    child.set_input(child_node_id.clone(), 2, Value::Integer(64));
    {
        let n = child.nodes.get_mut(&child_node_id).unwrap();
        n.outputs[0].is_exposed = true;
    }

    let child_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_image_child_{}.mangler.json", get_id()));
    let child_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&child_path, serde_json::to_string(&child_save).unwrap()).unwrap();

    // Build parent, reference the child, save, load fresh, run.
    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    let parent_path = std::env::temp_dir()
        .join(format!("mangler_subgraph_image_parent_{}.mangler.json", get_id()));
    let parent_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: parent.id.clone(),
        name: parent.name.clone(),
        nodes: parent.nodes.clone(),
    };
    fs::write(&parent_path, serde_json::to_string(&parent_save).unwrap()).unwrap();

    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut loaded_parent =
        Graph::load(parent_path.clone(), Some(tx_nc), Some(tx_gc), false)
            .expect("failed to load parent");

    // Before run: the parent output holds the placeholder value that
    // `create_outputs()` returns — a 1x1 white image.
    {
        let node = loaded_parent.nodes.get(&subgraph_node_id).unwrap();
        assert_eq!(node.outputs.len(), 1);
        if let Value::Image { data, .. } = &node.outputs[0].value {
            assert_eq!(data.width(), 1, "pre-run: expected 1x1 placeholder");
        } else {
            panic!("pre-run: expected Image, got {:?}", node.outputs[0].value);
        }
    }

    loaded_parent.run().await;

    // After run: parent's exposed output should reflect the 64x64 image
    // produced by the child's run, not the placeholder.
    let final_node = loaded_parent.nodes.get(&subgraph_node_id).unwrap();
    match &final_node.outputs[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 64, "parent output image width after run");
            assert_eq!(data.height(), 64, "parent output image height after run");
        }
        other => panic!("expected Image, got {:?}", other),
    }

    let _ = fs::remove_file(&child_path);
    let _ = fs::remove_file(&parent_path);
}

// ── hot-reload (cross-tab subgraph edits) ─────────────────────────────────

/// Helper: build a child graph file with a single exposed image-from-color node
/// producing a 32x32 image of the given color. Returns the tempfile path.
#[cfg(test)]
async fn write_child_with_color(color: crate::color::Color, label: &str) -> std::path::PathBuf {
    use std::fs;
    use crate::GraphSaveData;

    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), tx_nc, tx_gc, true).unwrap();
    let child_node_id = child
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageInputColor),
                  glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    child.set_input(child_node_id.clone(), 0, Value::Color(color));
    child.set_input(child_node_id.clone(), 1, Value::Integer(32));
    child.set_input(child_node_id.clone(), 2, Value::Integer(32));
    {
        let n = child.nodes.get_mut(&child_node_id).unwrap();
        n.outputs[0].is_exposed = true;
    }

    let path = std::env::temp_dir()
        .join(format!("mangler_hotreload_{}_{}.mangler.json", label, get_id()));
    let save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&path, serde_json::to_string(&save).unwrap()).unwrap();
    path
}

#[tokio::test]
async fn test_subgraph_reloads_when_file_mtime_changes() {
    use std::fs;
    use std::time::{Duration, SystemTime};
    use crate::color::Color;

    // Build an initial RED child graph.
    let child_path = write_child_with_color(
        Color::from_srgb_float(1.0, 0.0, 0.0, 1.0),
        "red",
    ).await;

    // Parent references the child — expect a red 32x32 image.
    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());
    parent.run().await;
    {
        let node = parent.nodes.get(&subgraph_node_id).unwrap();
        match &node.outputs[0].value {
            Value::Image { data, .. } => {
                let px = data.get_pixel(16, 16);
                assert!((px[0] - 1.0).abs() < 0.01, "pre-reload red check, got R={}", px[0]);
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    // Simulate another tab overwriting the child with a GREEN version. Rewrite
    // the file entirely. On fast filesystems mtime can have 1-second granularity,
    // so force an explicit newer timestamp to guarantee the check detects it.
    let green_path = write_child_with_color(
        Color::from_srgb_float(0.0, 1.0, 0.0, 1.0),
        "green",
    ).await;
    let green_content = fs::read_to_string(&green_path).unwrap();
    let _ = fs::remove_file(&green_path);
    fs::write(&child_path, green_content).unwrap();
    let future = SystemTime::now() + Duration::from_secs(2);
    filetime::set_file_mtime(&child_path, filetime::FileTime::from_system_time(future)).unwrap();

    // Detect the change and re-run.
    parent.check_subgraphs_for_changes();
    parent.run().await;

    let node = parent.nodes.get(&subgraph_node_id).unwrap();
    match &node.outputs[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(16, 16);
            assert!(px[0] < 0.01, "post-reload R should be 0.0, got {}", px[0]);
            assert!((px[1] - 1.0).abs() < 0.01, "post-reload G should be 1.0, got {}", px[1]);
            assert!(px[2] < 0.01, "post-reload B should be 0.0, got {}", px[2]);
        }
        other => panic!("expected Image, got {:?}", other),
    }

    let _ = fs::remove_file(&child_path);
}

#[tokio::test]
async fn test_check_subgraphs_noop_when_unchanged() {
    use crate::color::Color;

    let child_path = write_child_with_color(
        Color::from_srgb_float(0.0, 0.0, 1.0, 1.0),
        "noop",
    ).await;

    let (tx_nc, mut rx_nc) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut parent = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    // Drain the initial SubgraphLoaded emitted by set_subgraph_path itself.
    while rx_nc.try_recv().is_ok() {}

    // File unchanged — check should be a no-op.
    parent.check_subgraphs_for_changes();

    let mut reloaded = false;
    while let Ok(msg) = rx_nc.try_recv() {
        if matches!(msg, NodeChangedMessage::SubgraphLoaded { .. }) {
            reloaded = true;
            break;
        }
    }
    assert!(!reloaded, "subgraph was reloaded despite no mtime change");

    let _ = std::fs::remove_file(&child_path);
}

#[tokio::test]
async fn test_check_subgraphs_handles_missing_file() {
    use crate::color::Color;
    use crate::node_type::NodeType;

    let child_path = write_child_with_color(
        Color::from_srgb_float(0.5, 0.5, 0.5, 1.0),
        "missing",
    ).await;

    let mut parent = create_test_graph();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    // Delete the child file. The check must not panic, and the existing
    // in-memory snapshot should be preserved.
    let _ = std::fs::remove_file(&child_path);
    parent.check_subgraphs_for_changes();

    let node = parent.nodes.get(&subgraph_node_id).unwrap();
    assert!(
        matches!(node.node_type, NodeType::Subgraph { graph: Some(_), .. }),
        "child graph snapshot should be preserved when file is missing"
    );
}

/// Regression guard for Phase 15 (async thumbnail service):
/// when a node with a `Value::Image` output runs, the `OutputChanged`
/// message sent to the UI must carry `thumbnail: None` — the actual
/// thumbnail follows asynchronously via `ThumbnailReady`. This protects
/// against accidental re-inlining of `create_thumbnail()` on the engine
/// thread in the image output path.
#[tokio::test]
async fn test_image_output_defers_thumbnail() {
    use std::time::Duration;
    use tokio::time::timeout;

    let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_node_changed, mut rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let mut graph =
        Graph::new(get_id(), tx_node_changed, tx_graph_changed, false).unwrap();

    assert!(
        graph.thumbnail_service.is_some(),
        "tokio::test provides a runtime; the thumbnail service should spawn"
    );

    // image_from_color: outputs a Value::Image with no file I/O. First output
    // slot is the image.
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpImageInputColor),
            glam::Vec2::ZERO, true, None,
        Vec::new())
        .await;
    graph.run().await;

    // Drain messages for up to a couple of seconds, collecting any
    // OutputChanged for slot 0 and any ThumbnailReady for slot 0.
    let mut output_saw_image_with_no_thumb = false;
    let mut thumbnail_ready_seen = false;
    let deadline = Duration::from_millis(2000);
    for _ in 0..64 {
        let Ok(Some(m)) = timeout(deadline, rx_node_changed.recv()).await else { break };
        match m {
            NodeChangedMessage::OutputChanged {
                node_id: id,
                output_index: 0,
                value: Value::Image { .. },
                thumbnail,
            } if id == node_id => {
                assert!(
                    thumbnail.is_none(),
                    "engine should NOT inline thumbnail for Image outputs \
                     when the async service is available — got {:?}",
                    thumbnail,
                );
                output_saw_image_with_no_thumb = true;
            }
            NodeChangedMessage::ThumbnailReady {
                node_id: id,
                output_index: 0,
                ..
            } if id == node_id => {
                thumbnail_ready_seen = true;
                break;
            }
            _ => {}
        }
    }
    assert!(output_saw_image_with_no_thumb, "never saw the image OutputChanged");
    assert!(
        thumbnail_ready_seen,
        "async service did not deliver ThumbnailReady within 2s"
    );
}

// ── add_connection bounds checks ──────────────────────────────────────────
// Regression: validation used `len() >= index`, which passed when
// index == len and then panicked on direct indexing.

#[tokio::test]
async fn test_add_connection_output_index_equal_to_len_no_panic() {
    let mut graph = create_test_graph();
    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // decimal node has exactly 1 output — index 1 == len, the old off-by-one.
    graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 1).await;

    // No connection should have been made, and no panic.
    assert!(graph.nodes.get(&add_id).unwrap().inputs[0].connection.is_none());
}

#[tokio::test]
async fn test_add_connection_input_index_equal_to_len_no_panic() {
    let mut graph = create_test_graph();
    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // add node has exactly 2 inputs — index 2 == len, the old off-by-one.
    graph.add_connection(add_id.clone(), 2, decimal_id.clone(), 0).await;

    let decimal_node = graph.nodes.get(&decimal_id).unwrap();
    let conns = decimal_node.outputs[0].connection.as_ref();
    assert!(conns.is_none() || conns.unwrap().is_empty());
}

#[tokio::test]
async fn test_add_connection_way_out_of_range_indices_no_panic() {
    let mut graph = create_test_graph();
    let decimal_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    graph.add_connection(add_id.clone(), 99, decimal_id.clone(), 99).await;

    assert!(graph.nodes.get(&add_id).unwrap().inputs[0].connection.is_none());
    assert!(graph.nodes.get(&add_id).unwrap().inputs[1].connection.is_none());
}

// ── add_connection cycle rejection ────────────────────────────────────────

#[tokio::test]
async fn test_add_connection_rejects_self_connection() {
    let mut graph = create_test_graph();
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // A -> A: type-compatible (Decimal -> Decimal) but must be refused.
    graph.add_connection(add_id.clone(), 0, add_id.clone(), 0).await;

    let node = graph.nodes.get(&add_id).unwrap();
    assert!(node.inputs[0].connection.is_none(), "self-connection must be refused");
    let conns = node.outputs[0].connection.as_ref();
    assert!(conns.is_none() || conns.unwrap().is_empty());
}

#[tokio::test]
async fn test_add_connection_rejects_two_node_cycle() {
    let mut graph = create_test_graph();
    let a_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let b_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    // A -> B is fine.
    graph.add_connection(b_id.clone(), 0, a_id.clone(), 0).await;
    assert!(graph.nodes.get(&b_id).unwrap().inputs[0].connection.is_some());

    // B -> A would close the cycle A -> B -> A: refused.
    graph.add_connection(a_id.clone(), 0, b_id.clone(), 0).await;
    assert!(
        graph.nodes.get(&a_id).unwrap().inputs[0].connection.is_none(),
        "cycle-closing connection must be refused"
    );

    // The original A -> B connection must be untouched.
    let (conn_id, _) = graph.nodes.get(&b_id).unwrap().inputs[0].connection.as_ref().unwrap();
    assert_eq!(conn_id, &a_id);
}

#[tokio::test]
async fn test_add_connection_rejects_three_node_cycle() {
    let mut graph = create_test_graph();
    let a_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let b_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;
    let c_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(400.0, 0.0), true, None, Vec::new())
        .await;

    // A -> B -> C
    graph.add_connection(b_id.clone(), 0, a_id.clone(), 0).await;
    graph.add_connection(c_id.clone(), 0, b_id.clone(), 0).await;

    // C -> A would close a 3-node cycle: refused.
    graph.add_connection(a_id.clone(), 0, c_id.clone(), 0).await;
    assert!(
        graph.nodes.get(&a_id).unwrap().inputs[0].connection.is_none(),
        "3-node cycle-closing connection must be refused"
    );
}

#[tokio::test]
async fn test_add_connection_allows_diamond() {
    // A feeds B and C; both feed D. Reconvergence is NOT a cycle and must
    // still be allowed by the cycle check.
    let mut graph = create_test_graph();
    let a_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let b_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, -100.0), true, None, Vec::new())
        .await;
    let c_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 100.0), true, None, Vec::new())
        .await;
    let d_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(400.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(b_id.clone(), 0, a_id.clone(), 0).await;
    graph.add_connection(c_id.clone(), 0, a_id.clone(), 0).await;
    graph.add_connection(d_id.clone(), 0, b_id.clone(), 0).await;
    graph.add_connection(d_id.clone(), 1, c_id.clone(), 0).await;

    let d_node = graph.nodes.get(&d_id).unwrap();
    assert!(d_node.inputs[0].connection.is_some(), "diamond edge B->D must be allowed");
    assert!(d_node.inputs[1].connection.is_some(), "diamond edge C->D must be allowed");

    // Sanity: the diamond computes. a=5 -> b=5+1=6, c=5+2=7, d=6+7=13.
    graph.set_input(a_id.clone(), 0, Value::Decimal(5.0));
    graph.set_input(b_id.clone(), 1, Value::Decimal(1.0));
    graph.set_input(c_id.clone(), 1, Value::Decimal(2.0));
    graph.run().await;
    match &graph.nodes.get(&d_id).unwrap().outputs[0].value {
        Value::Decimal(v) => assert!((*v - 13.0).abs() < 1e-6, "expected 13.0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

// ── stale out-of-range connection indices during run() ────────────────────
// Regression: propagation used `connected_node.inputs[input_index]` on
// indices stored in output.connection, panicking when the target's inputs
// had shrunk (e.g. a reloaded subgraph exposing fewer inputs).

#[tokio::test]
async fn test_run_with_stale_out_of_range_connection_does_not_panic() {
    let mut graph = create_test_graph();
    let source_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let consumer_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(consumer_id.clone(), 1, source_id.clone(), 0).await;

    // Simulate a stale persisted connection: shrink the consumer's inputs so
    // index 1 no longer exists while the source's output still lists it.
    // Disable the consumer so its (now malformed) operation never executes —
    // we are exercising the propagation sites, not the op.
    {
        let consumer = graph.nodes.get_mut(&consumer_id).unwrap();
        consumer.inputs.truncate(1);
        consumer.is_enabled = false;
    }

    // Normal run path: source executes and propagates — must skip, not panic.
    graph.set_input(source_id.clone(), 0, Value::Decimal(7.0));
    graph.run().await;

    // Skip-branch path: re-run the source with unchanged inputs so the cached
    // input hash matches and the "still propagate existing outputs" branch
    // runs — must also skip, not panic.
    graph.nodes.get_mut(&source_id).unwrap().is_dirty = true;
    graph.run().await;

    // The remaining in-range input is untouched by the stale connection.
    assert_eq!(graph.nodes.get(&consumer_id).unwrap().inputs.len(), 1);
}

#[tokio::test]
async fn test_run_disabled_passthrough_with_stale_connection_does_not_panic() {
    let mut graph = create_test_graph();
    let source_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let consumer_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(consumer_id.clone(), 1, source_id.clone(), 0).await;

    // Disable the SOURCE so its passthrough branch does the propagation, and
    // shrink the consumer so the stale index 1 is out of range.
    {
        let consumer = graph.nodes.get_mut(&consumer_id).unwrap();
        consumer.inputs.truncate(1);
        consumer.is_enabled = false;
    }
    {
        let source = graph.nodes.get_mut(&source_id).unwrap();
        source.is_enabled = false;
    }

    graph.set_input(source_id.clone(), 0, Value::Decimal(3.0));
    graph.run().await; // must not panic (disabled-passthrough propagation site)
}

// ── set_subgraph_path prunes stale upstream connections ───────────────────

/// Helper: write a child graph file containing one add node with the first
/// `exposed_inputs` inputs exposed and the output exposed. Returns the path.
#[cfg(test)]
async fn write_child_with_exposed_inputs(exposed_inputs: usize, label: &str) -> std::path::PathBuf {
    use std::fs;
    use crate::GraphSaveData;

    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut child = Graph::new(get_id(), tx_nc, tx_gc, true).unwrap();
    let node_id = child
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    {
        let n = child.nodes.get_mut(&node_id).unwrap();
        for i in 0..exposed_inputs {
            n.inputs[i].is_exposed = true;
        }
        n.outputs[0].is_exposed = true;
    }

    let path = std::env::temp_dir()
        .join(format!("mangler_prune_child_{}_{}.mangler.json", label, get_id()));
    let save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: child.id.clone(),
        name: child.name.clone(),
        nodes: child.nodes.clone(),
    };
    fs::write(&path, serde_json::to_string(&save).unwrap()).unwrap();
    path
}

#[tokio::test]
async fn test_subgraph_reload_prunes_out_of_range_upstream_connections() {
    use std::fs;

    // Child v1 exposes two inputs; child v2 exposes only one.
    let two_input_path = write_child_with_exposed_inputs(2, "two").await;
    let one_input_path = write_child_with_exposed_inputs(1, "one").await;

    let mut parent = create_test_graph();
    let subgraph_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;
    parent.set_subgraph_path(subgraph_id.clone(), two_input_path.clone());
    assert_eq!(parent.nodes.get(&subgraph_id).unwrap().inputs.len(), 2);

    // Feed both exposed inputs from one decimal source.
    let source_id = parent
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    parent.add_connection(subgraph_id.clone(), 0, source_id.clone(), 0).await;
    parent.add_connection(subgraph_id.clone(), 1, source_id.clone(), 0).await;
    {
        let conns = parent.nodes.get(&source_id).unwrap().outputs[0].connection.as_ref().unwrap();
        assert!(conns.contains(&(subgraph_id.clone(), 0)));
        assert!(conns.contains(&(subgraph_id.clone(), 1)));
    }

    // Reload the subgraph node from a child exposing only ONE input.
    parent.set_subgraph_path(subgraph_id.clone(), one_input_path.clone());
    assert_eq!(parent.nodes.get(&subgraph_id).unwrap().inputs.len(), 1);

    // The upstream entry for the vanished input index 1 must be pruned; the
    // in-range entry for index 0 must survive.
    {
        let conns = parent.nodes.get(&source_id).unwrap().outputs[0].connection.as_ref().unwrap();
        assert!(
            !conns.contains(&(subgraph_id.clone(), 1)),
            "stale connection to removed input index 1 must be pruned"
        );
        assert!(
            conns.contains(&(subgraph_id.clone(), 0)),
            "in-range connection to input index 0 must be kept"
        );
    }

    // The engine must survive subsequent runs (this used to panic).
    parent.set_input(source_id.clone(), 0, Value::Decimal(4.0));
    parent.run().await;
    parent.nodes.get_mut(&source_id).unwrap().is_dirty = true;
    parent.run().await;

    let _ = fs::remove_file(&two_input_path);
    let _ = fs::remove_file(&one_input_path);
}

// ── save_to_file error handling ───────────────────────────────────────────

#[tokio::test]
async fn test_save_to_file_write_failure_does_not_panic() {
    let mut graph = create_test_graph();
    graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // Point the save path into a directory that does not exist: fs::write
    // fails. Must return an error, not panic or corrupt state.
    graph.set_save_path(std::path::PathBuf::from(
        "/nonexistent_mangler_dir_for_test/graph.mangler.json",
    ));
    assert!(graph.save_to_file().is_err());

    assert_eq!(graph.nodes.len(), 1);
}

// Saving to a path whose *parent* is a regular file (not a directory) is
// another way the write can fail — `fs::write` returns an OS error instead
// of creating anything. Distinct from the nonexistent-directory case above:
// this one is a real, existing filesystem entry that just isn't a directory.
#[tokio::test]
async fn test_save_to_file_parent_is_regular_file_returns_err() {
    let mut graph = create_test_graph();

    let tmp_dir = std::env::temp_dir().join(format!("mangler_save_err_{}", get_id()));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let file_masquerading_as_dir = tmp_dir.join("not_a_directory");
    std::fs::write(&file_masquerading_as_dir, b"not a directory").unwrap();

    let save_path = file_masquerading_as_dir.join("graph.mangler.json");
    graph.set_save_path(save_path);

    assert!(graph.save_to_file().is_err(), "writing under a regular file as if it were a directory should fail");

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

// ── trigger firing vs input-hash cache ────────────────────────────────────
//
// Value::Trigger hashes as a constant (see Value::fingerprint), so the
// input-hash skip in Graph::run cannot tell a fresh firing apart from
// "inputs unchanged". Graph::run compensates with its forced_nodes set.
// Node::run records `time` on every execution, so resetting `time` to None
// before a run gives a deterministic ran/skipped marker per node.

/// Build random_decimal -> add -> add and return (graph, ids).
async fn create_trigger_chain() -> (Graph, String, String, String) {
    let mut graph = create_test_graph();
    let random_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberRandomDecimal), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let add1_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0), true, None, Vec::new())
        .await;
    let add2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(400.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(add1_id.clone(), 0, random_id.clone(), 0).await;
    graph.add_connection(add2_id.clone(), 0, add1_id.clone(), 0).await;

    (graph, random_id, add1_id, add2_id)
}

fn clear_ran_markers(graph: &mut Graph, ids: &[&String]) {
    for id in ids {
        graph.nodes.get_mut(*id).unwrap().time = None;
    }
}

#[tokio::test]
async fn test_trigger_fire_reruns_two_hop_downstream_chain() {
    let (mut graph, random_id, add1_id, add2_id) = create_trigger_chain().await;

    // Initial run settles the chain and populates the input-hash cache.
    graph.run().await;

    clear_ran_markers(&mut graph, &[&random_id, &add1_id, &add2_id]);

    // Fire the trigger the way connection propagation / subgraph input
    // forwarding does: Node::set_input_value marks the node dirty but does
    // NOT clear cached_input_hash, and the Trigger value hashes identically
    // to the previous firing.
    graph
        .nodes
        .get_mut(&random_id)
        .unwrap()
        .set_input_value(0, Value::Trigger);
    graph.run().await;

    assert!(
        graph.nodes.get(&random_id).unwrap().time.is_some(),
        "trigger-fired node must re-run despite its constant input hash"
    );
    assert!(
        graph.nodes.get(&add1_id).unwrap().time.is_some(),
        "1-hop downstream node must re-run after a trigger firing"
    );
    assert!(
        graph.nodes.get(&add2_id).unwrap().time.is_some(),
        "2-hop downstream node must re-run after a trigger firing"
    );

    // Exactly once per firing: a follow-up tick with no new firing must not
    // re-run anything.
    clear_ran_markers(&mut graph, &[&random_id, &add1_id, &add2_id]);
    graph.run().await;

    assert!(graph.nodes.get(&random_id).unwrap().time.is_none());
    assert!(graph.nodes.get(&add1_id).unwrap().time.is_none());
    assert!(graph.nodes.get(&add2_id).unwrap().time.is_none());
}

#[tokio::test]
async fn test_no_trigger_fire_downstream_nodes_stay_cached() {
    let (mut graph, random_id, add1_id, add2_id) = create_trigger_chain().await;

    // Second input of add1 driven by a decimal node so we can dirty part of
    // the graph without firing the trigger.
    let dec_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::new(0.0, 200.0), true, None, Vec::new())
        .await;
    graph.add_connection(add1_id.clone(), 1, dec_id.clone(), 0).await;

    graph.set_input(dec_id.clone(), 0, Value::Decimal(5.0));
    graph.run().await;

    clear_ran_markers(&mut graph, &[&random_id, &add1_id, &add2_id, &dec_id]);

    // Re-set the same value: the decimal node re-runs (set_input clears its
    // hash cache) but produces an identical output, and no trigger fired —
    // so the nodes downstream of the trigger must all be skipped (cached).
    graph.set_input(dec_id.clone(), 0, Value::Decimal(5.0));
    graph.run().await;

    assert!(
        graph.nodes.get(&dec_id).unwrap().time.is_some(),
        "explicitly edited node runs (its hash cache was invalidated)"
    );
    assert!(
        graph.nodes.get(&random_id).unwrap().time.is_none(),
        "trigger node must not re-run when the trigger did not fire"
    );
    assert!(
        graph.nodes.get(&add1_id).unwrap().time.is_none(),
        "downstream node must stay cached when nothing changed and no trigger fired"
    );
    assert!(
        graph.nodes.get(&add2_id).unwrap().time.is_none(),
        "2-hop downstream node must stay cached when nothing changed and no trigger fired"
    );
}

// ── tolerant load / unknown-node placeholders (saved-graph version compat) ─

/// Build the raw JSON for a normal add node, then corrupt its operation
/// string to something this build cannot recognize — simulating a node
/// saved by a newer NodeMangler.
fn add_node_json_with_unknown_operation(position: glam::Vec2) -> serde_json::Value {
    let template = crate::node::Node::new(
        get_id(),
        AddNodeType::Operation(Operation::OpNumberMathAdd),
        position,
    );
    let mut raw = serde_json::to_value(&template).unwrap();
    raw["node_type"]["Operation"]["operation"] = serde_json::json!("OpFromTheFuture");
    raw
}

#[test]
fn test_tolerant_load_preserves_other_nodes_and_emits_load_warnings_first() {
    use std::fs;
    use crate::GraphSaveData;
    use crate::node_type::NodeType;

    // A normal decimal node plus a hand-crafted unknown-op node.
    let node_a = crate::node::Node::new(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO);
    let node_a_id = node_a.id.clone();
    let mut nodes = std::collections::HashMap::new();
    nodes.insert(node_a_id.clone(), node_a);

    let unknown_id = get_id();
    let unknown_raw = add_node_json_with_unknown_operation(glam::Vec2::new(50.0, 60.0));

    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: "tolerant-load-graph".to_string(),
        name: "tolerant load test".to_string(),
        nodes,
    };
    let mut json = serde_json::to_value(&save_data).unwrap();
    json["nodes"][unknown_id.as_str()] = unknown_raw;

    let tmp_path = std::env::temp_dir().join(format!("mangler_tolerant_load_{}.mangler.json", get_id()));
    fs::write(&tmp_path, serde_json::to_string(&json).unwrap()).unwrap();

    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let (tx_gc, mut rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let loaded = Graph::load(tmp_path.clone(), Some(tx_nc), Some(tx_gc), false)
        .expect("tolerant load should succeed despite the unknown node");

    // Both nodes present; the good one loaded normally, the bad one is a
    // placeholder.
    assert_eq!(loaded.nodes.len(), 2);
    let good = loaded.nodes.get(&node_a_id).expect("known node should load normally");
    assert!(matches!(good.node_type, NodeType::Operation { .. }));
    let bad = loaded.nodes.get(&unknown_id).expect("unknown node should still be present as a placeholder");
    assert!(matches!(bad.node_type, NodeType::Unknown { .. }));
    assert!(bad.is_error);

    // LoadWarnings must arrive before any LoadedNode.
    let first = rx_gc.try_recv().expect("a message should have been sent");
    assert!(matches!(first, GraphChangedMessage::LoadWarnings { .. }), "expected LoadWarnings first, got {:?}", first);

    let mut loaded_node_count = 0;
    while let Ok(msg) = rx_gc.try_recv() {
        if matches!(msg, GraphChangedMessage::LoadedNode { .. }) {
            loaded_node_count += 1;
        }
    }
    assert_eq!(loaded_node_count, 2, "both nodes should still be echoed to the UI");

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_save_to_file_round_trips_unknown_node_verbatim() {
    use std::fs;
    use crate::GraphSaveData;
    use crate::node_type::NodeType;

    let known = crate::node::Node::new(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(1.0, 2.0));
    let known_id = known.id.clone();
    let mut nodes = std::collections::HashMap::new();
    nodes.insert(known_id.clone(), known);

    let unknown_id = get_id();
    let mut unknown_raw = add_node_json_with_unknown_operation(glam::Vec2::new(3.0, 4.0));
    unknown_raw["future_only_field"] = serde_json::json!(42);

    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "round trip test".to_string(),
        nodes,
    };
    let mut json = serde_json::to_value(&save_data).unwrap();
    json["nodes"][unknown_id.as_str()] = unknown_raw.clone();

    let tmp_path = std::env::temp_dir().join(format!("mangler_unknown_roundtrip_{}.mangler.json", get_id()));
    fs::write(&tmp_path, serde_json::to_string(&json).unwrap()).unwrap();

    // Load (tolerant) and immediately save back out via save_to_file (the
    // GraphSaveRef mirror path — this is what guards the mirror staying in
    // sync with GraphSaveData's tolerant `#[serde(with)]` attribute).
    let mut loaded = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    assert!(matches!(loaded.nodes.get(&unknown_id).unwrap().node_type, NodeType::Unknown { .. }));
    loaded.save_to_file().unwrap();

    let raw_after = fs::read_to_string(&tmp_path).unwrap();
    let after: serde_json::Value = serde_json::from_str(&raw_after).unwrap();
    let node_after = &after["nodes"][unknown_id.as_str()];

    // A field this build never parsed must survive untouched.
    assert_eq!(node_after["future_only_field"], serde_json::json!(42));
    // Position (untouched by any edit) matches the original.
    assert_eq!(node_after["position"], unknown_raw["position"]);

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_placeholder_node_move_persists_through_save_and_load() {
    use std::fs;
    use crate::GraphSaveData;
    use crate::node_type::NodeType;

    let unknown_id = get_id();
    let unknown_raw = add_node_json_with_unknown_operation(glam::Vec2::ZERO);

    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "move test".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    let mut json = serde_json::to_value(&save_data).unwrap();
    json["nodes"][unknown_id.as_str()] = unknown_raw;

    let tmp_path = std::env::temp_dir().join(format!("mangler_placeholder_move_{}.mangler.json", get_id()));
    fs::write(&tmp_path, serde_json::to_string(&json).unwrap()).unwrap();

    let mut graph = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    graph.set_node_position(unknown_id.clone(), glam::Vec2::new(321.0, 654.0));
    graph.save_to_file().unwrap();

    let reloaded = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    let node = reloaded.nodes.get(&unknown_id).unwrap();
    assert!(matches!(node.node_type, NodeType::Unknown { .. }));
    assert_eq!(node.position, glam::Vec2::new(321.0, 654.0));

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_connection_into_placeholder_persists_in_raw() {
    use std::fs;
    use crate::GraphSaveData;
    use crate::node_type::NodeType;

    let unknown_id = get_id();
    let unknown_raw = add_node_json_with_unknown_operation(glam::Vec2::ZERO);

    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "connection persist test".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    let mut json = serde_json::to_value(&save_data).unwrap();
    json["nodes"][unknown_id.as_str()] = unknown_raw;

    let tmp_path = std::env::temp_dir().join(format!("mangler_placeholder_conn_{}.mangler.json", get_id()));
    fs::write(&tmp_path, serde_json::to_string(&json).unwrap()).unwrap();

    let mut graph = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    assert!(matches!(graph.nodes.get(&unknown_id).unwrap().node_type, NodeType::Unknown { .. }));
    assert_eq!(graph.nodes.get(&unknown_id).unwrap().inputs.len(), 2, "add's sockets should have parsed");

    let source_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::new(-100.0, 0.0), true, None, Vec::new())
        .await;

    graph.add_connection(unknown_id.clone(), 0, source_id.clone(), 0).await;
    assert!(
        graph.nodes.get(&unknown_id).unwrap().inputs[0].connection.is_some(),
        "a connection into a placeholder's parsed socket should be accepted like any other node"
    );

    graph.save_to_file().unwrap();
    let reloaded = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    let node = reloaded.nodes.get(&unknown_id).unwrap();
    assert!(matches!(node.node_type, NodeType::Unknown { .. }));
    let (conn_id, conn_idx) = node.inputs[0].connection.as_ref()
        .expect("connection into a placeholder must survive save + reload");
    assert_eq!(conn_id, &source_id);
    assert_eq!(*conn_idx, 0);

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_run_with_placeholder_node_emits_error_and_does_not_panic() {
    use crate::node_type::NodeType;

    let (tx_nc, mut rx_nc) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

    let raw = add_node_json_with_unknown_operation(glam::Vec2::ZERO);
    let placeholder_id = get_id();
    let placeholder = crate::node::Node::placeholder_from_raw(placeholder_id.clone(), raw);
    graph.nodes.insert(placeholder_id.clone(), placeholder);
    graph.nodes.get_mut(&placeholder_id).unwrap().is_dirty = true;

    // Must not panic.
    graph.run().await;

    let mut saw_error = false;
    while let Ok(msg) = rx_nc.try_recv() {
        if let NodeChangedMessage::Error { node_id, is_error: true, .. } = msg {
            if node_id == placeholder_id {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "running a placeholder node should emit a persistent Error message");
    assert!(matches!(graph.nodes.get(&placeholder_id).unwrap().node_type, NodeType::Unknown { .. }));
}

#[tokio::test]
async fn test_load_report_detects_newer_version() {
    use std::fs;
    use crate::GraphSaveData;

    let save_data = GraphSaveData {
        version: "999.0.0".to_string(),
        id: get_id(),
        name: "newer version test".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    let tmp_path = std::env::temp_dir().join(format!("mangler_newer_version_{}.mangler.json", get_id()));
    fs::write(&tmp_path, serde_json::to_string(&save_data).unwrap()).unwrap();

    let loaded = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    let report = loaded.load_report.expect("load_report should be populated by a real load");
    assert!(report.is_newer_than_app);
    assert_eq!(report.file_version, "999.0.0");

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_load_report_missing_version_is_not_newer() {
    use std::fs;

    // Pre-versioning save: no "version" field at all.
    let tmp_path = std::env::temp_dir().join(format!("mangler_missing_version_{}.mangler.json", get_id()));
    let json = serde_json::json!({
        "id": "old-graph",
        "name": "pre-versioning graph",
        "nodes": {}
    });
    fs::write(&tmp_path, serde_json::to_string(&json).unwrap()).unwrap();

    let loaded = Graph::load(tmp_path.clone(), None, None, false).unwrap();
    let report = loaded.load_report.expect("load_report should always be populated by a real load");
    assert!(!report.is_newer_than_app);
    assert_eq!(report.file_version, "");

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_save_to_file_updates_last_synced_mtime() {
    let mut graph = create_test_graph();
    let tmp_path = std::env::temp_dir().join(format!("mangler_mtime_save_{}.mangler.json", get_id()));
    graph.set_save_path(tmp_path.clone());

    assert!(graph.last_synced_mtime.is_none(), "no sync has happened yet");
    graph.save_to_file().unwrap();
    assert!(graph.last_synced_mtime.is_some(), "save_to_file should record the file's new mtime");

    let _ = std::fs::remove_file(tmp_path);
}

#[tokio::test]
async fn test_disk_is_newer_detects_external_rewrite() {
    let mut graph = create_test_graph();
    let tmp_path = std::env::temp_dir().join(format!("mangler_disk_newer_{}.mangler.json", get_id()));
    graph.set_save_path(tmp_path.clone());
    graph.save_to_file().unwrap();

    assert!(!graph.disk_is_newer(), "no external edit has happened yet");

    // mtime granularity can be 1s on some filesystems; sleep across it so the
    // external rewrite is guaranteed to produce a strictly later mtime.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&tmp_path, "{}").unwrap();

    assert!(graph.disk_is_newer(), "an external rewrite after our last sync should be detected");

    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_disk_is_newer_false_when_no_save_path() {
    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();
    assert!(!graph.disk_is_newer());
}

#[tokio::test]
async fn test_subgraph_with_unknown_child_node_surfaces_error_on_parent() {
    use std::fs;
    use crate::GraphSaveData;

    // Child graph with one normal node plus a hand-crafted unknown-op node.
    let normal = crate::node::Node::new(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO);
    let normal_id = normal.id.clone();
    let mut child_nodes = std::collections::HashMap::new();
    child_nodes.insert(normal_id.clone(), normal);

    let unknown_id = get_id();
    let unknown_raw = add_node_json_with_unknown_operation(glam::Vec2::ZERO);

    let child_save = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "child with unknown node".to_string(),
        nodes: child_nodes,
    };
    let mut child_json = serde_json::to_value(&child_save).unwrap();
    child_json["nodes"][unknown_id.as_str()] = unknown_raw;

    let child_path = std::env::temp_dir()
        .join(format!("mangler_child_unknown_{}.mangler.json", get_id()));
    fs::write(&child_path, serde_json::to_string(&child_json).unwrap()).unwrap();

    // Parent with a subgraph node, kept on a channel we can inspect.
    let (tx_nc, mut rx_nc) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut parent = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    // Drain the AddedNode-adjacent chatter from add_node before checking for
    // the subgraph-load error.
    while rx_nc.try_recv().is_ok() {}

    parent.set_subgraph_path(subgraph_node_id.clone(), child_path.clone());

    let mut saw_unknown_error = false;
    while let Ok(msg) = rx_nc.try_recv() {
        if let NodeChangedMessage::Error { node_id, is_error: true, message: Some(text) } = msg {
            if node_id == subgraph_node_id && text.contains("unknown") {
                saw_unknown_error = true;
            }
        }
    }
    assert!(saw_unknown_error, "parent subgraph node should report the child's unknown node");

    let _ = fs::remove_file(&child_path);
}

#[tokio::test]
async fn test_subgraph_bogus_path_emits_error_message_not_just_println() {
    let (tx_nc, mut rx_nc) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let mut parent = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();
    let subgraph_node_id = parent
        .add_node(get_id(), AddNodeType::Subgraph, glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    while rx_nc.try_recv().is_ok() {}

    parent.set_subgraph_path(
        subgraph_node_id.clone(),
        std::path::PathBuf::from("/definitely/does/not/exist.mangler.json"),
    );

    let mut saw_error = false;
    while let Ok(msg) = rx_nc.try_recv() {
        if let NodeChangedMessage::Error { node_id, is_error: true, message: Some(text) } = msg {
            if node_id == subgraph_node_id && text.contains("failed to load subgraph") {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "a bogus subgraph path should surface an Error message on the node, not just a println");
}

// === rename_file ===

// Happy path: the file is physically moved, save_path and name follow the new
// stem, and last_synced_mtime is re-stat'd from the new path so a follow-up
// disk_is_newer() check does NOT see a spurious conflict.
#[test]
fn test_rename_file_moves_file_and_updates_state() {
    use std::fs;

    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

    let old_path = std::env::temp_dir().join(format!("mangler_rename_old_{}.mangler.json", get_id()));
    graph.set_save_path(old_path.clone());
    graph.save_to_file().unwrap();
    assert!(old_path.exists());

    let new_path = graph.rename_file("renamed graph").expect("rename should succeed");

    // Old file gone, new file present.
    assert!(!old_path.exists(), "old file should have been moved");
    assert!(new_path.exists(), "new file should exist");
    assert_eq!(
        new_path.file_name().unwrap().to_str().unwrap(),
        "renamed graph.mangler.json"
    );

    // save_path and name follow the new stem.
    assert_eq!(graph.save_path, Some(new_path.clone()));
    assert_eq!(graph.name, "renamed graph");

    // Baseline was re-stat'd from the new path, so no spurious conflict.
    assert!(!graph.disk_is_newer(), "disk_is_newer must be false right after rename");

    let _ = fs::remove_file(&new_path);
}

// A rename that would collide with an existing file must error and leave the
// original file untouched.
#[test]
fn test_rename_file_collision_errors_and_preserves_original() {
    use std::fs;

    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

    let unique = get_id();
    let old_path = std::env::temp_dir().join(format!("mangler_rename_src_{}.mangler.json", unique));
    graph.set_save_path(old_path.clone());
    graph.save_to_file().unwrap();

    // Pre-create the file the rename would target.
    let target_stem = format!("mangler_rename_dst_{}", unique);
    let target_path = std::env::temp_dir().join(format!("{}.mangler.json", target_stem));
    fs::write(&target_path, "existing file").unwrap();

    let result = graph.rename_file(&target_stem);
    assert!(result.is_err(), "rename onto an existing file must error");

    // Original file and state untouched; target file not clobbered.
    assert!(old_path.exists(), "original file must survive a failed rename");
    assert_eq!(graph.save_path, Some(old_path.clone()));
    assert_eq!(fs::read_to_string(&target_path).unwrap(), "existing file");

    let _ = fs::remove_file(&old_path);
    let _ = fs::remove_file(&target_path);
}

// Renaming a graph that has no save path yet is an error.
#[test]
fn test_rename_file_without_save_path_errors() {
    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

    assert!(graph.save_path.is_none());
    assert!(graph.rename_file("whatever").is_err());
}

// The embedded `name` field is a write-only mirror: load ignores it and
// derives the display name from the file stem instead.
#[test]
fn test_load_ignores_embedded_name_uses_file_stem() {
    use std::fs;
    use crate::GraphSaveData;

    let tmp_path = std::env::temp_dir().join(format!("mangler_stem_name_{}.mangler.json", get_id()));
    let save_data = GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        // Deliberately disagrees with the file stem — must be ignored.
        name: "embedded bar name".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    fs::write(&tmp_path, serde_json::to_string(&save_data).unwrap()).unwrap();

    let loaded = Graph::load(tmp_path.clone(), None, None, false).expect("should load");
    let expected_stem = tmp_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .strip_suffix(".mangler.json")
        .unwrap();
    assert_eq!(loaded.name, expected_stem);
    assert_ne!(loaded.name, "embedded bar name");

    let _ = fs::remove_file(&tmp_path);
}

#[tokio::test]
async fn test_to_file_node_prefills_folder_with_graph_dir() {
    use std::path::PathBuf;
    const FOLDER: usize = 1;

    let mut graph = create_test_graph();
    let dir = PathBuf::from("/some/where/MyGraphs");
    graph.set_save_path(dir.join("mygraph.mangler.json"));

    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputFile), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    let folder = &node.inputs[FOLDER].value;
    assert!(matches!(folder, Value::Path(p) if p == &dir));

    // File name defaults to `{graph name}_{N}`, starting at 1.
    const FILE_NAME: usize = 2;
    assert!(matches!(&node.inputs[FILE_NAME].value, Value::Text(t) if t == "mygraph_1"));
}

#[tokio::test]
async fn test_to_file_nodes_get_incrementing_unique_names() {
    use std::path::PathBuf;
    const FILE_NAME: usize = 2;

    let mut graph = create_test_graph();
    // Point at a nonexistent dir so only in-graph names drive uniqueness.
    graph.set_save_path(PathBuf::from("/no/such/dir/mygraph.mangler.json"));

    let a = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputFile), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let b = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputFile), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let c = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputFile), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    let name = |id: &str| match &graph.nodes.get(id).unwrap().inputs[FILE_NAME].value {
        Value::Text(t) => t.clone(),
        _ => panic!("expected text"),
    };
    assert_eq!(name(&a), "mygraph_1");
    assert_eq!(name(&b), "mygraph_2");
    assert_eq!(name(&c), "mygraph_3");
}

#[tokio::test]
async fn test_material_node_prefills_folder_and_name() {
    use std::path::PathBuf;
    const FOLDER: usize = 9;
    const FILE_NAME: usize = 10;

    let mut graph = create_test_graph();
    let dir = PathBuf::from("/some/where/MyGraphs");
    graph.set_save_path(dir.join("mygraph.mangler.json"));

    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputMaterial), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    let node = graph.nodes.get(&node_id).unwrap();
    assert!(matches!(&node.inputs[FOLDER].value, Value::Path(p) if p == &dir));
    assert!(matches!(&node.inputs[FILE_NAME].value, Value::Text(t) if t == "mygraph_1"));
}

#[tokio::test]
async fn test_output_nodes_share_unique_name_counter_across_types() {
    use std::path::PathBuf;

    // A "to file" and a "material" node in the same graph must not pick the
    // same {name}_{N} stem (they'd collide on disk).
    let mut graph = create_test_graph();
    graph.set_save_path(PathBuf::from("/no/such/dir/mygraph.mangler.json"));

    let file_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputFile), glam::Vec2::ZERO, true, None, Vec::new())
        .await;
    let mat_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputMaterial), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    let file_name = match &graph.nodes.get(&file_id).unwrap().inputs[2].value {
        Value::Text(t) => t.clone(),
        _ => panic!("expected text"),
    };
    let mat_name = match &graph.nodes.get(&mat_id).unwrap().inputs[10].value {
        Value::Text(t) => t.clone(),
        _ => panic!("expected text"),
    };
    assert_eq!(file_name, "mygraph_1");
    assert_eq!(mat_name, "mygraph_2");
}

#[tokio::test]
async fn test_to_file_node_folder_empty_without_save_path() {
    const FOLDER: usize = 1;

    // No save_path set: folder stays empty (empty resolves to graph dir at save).
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpImageOutputFile), glam::Vec2::ZERO, true, None, Vec::new())
        .await;

    let folder = &graph.nodes.get(&node_id).unwrap().inputs[FOLDER].value;
    assert!(matches!(folder, Value::Path(p) if p.as_os_str().is_empty()));
}

#[tokio::test]
async fn test_to_file_node_explicit_folder_not_clobbered() {
    use std::path::PathBuf;
    const FOLDER: usize = 1;

    // A recreated node carrying an explicit folder keeps it, even though the
    // graph has its own save location.
    let mut graph = create_test_graph();
    graph.set_save_path(PathBuf::from("/some/where/MyGraphs/mygraph.mangler.json"));

    let explicit = PathBuf::from("/elsewhere/renders");
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpImageOutputFile),
            glam::Vec2::ZERO, true, None,
            vec![(FOLDER, Value::Path(explicit.clone()))],
        )
        .await;

    let folder = &graph.nodes.get(&node_id).unwrap().inputs[FOLDER].value;
    assert!(matches!(folder, Value::Path(p) if p == &explicit));
}

/// Regression: a graph saved by a build whose op had *fewer* inputs must load
/// with a full-length input slice (padded from the current schema), so `run()`
/// can't index out of bounds and panic the engine thread. Saved slots keep
/// their value; new trailing slots get schema defaults.
#[tokio::test]
async fn load_pads_short_input_vector_to_current_schema() {
    let mut graph = create_test_graph();
    // `levels` currently has 6 inputs.
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpImageAdjustmentLevels),
            glam::Vec2::ZERO,
            true,
            None,
            Vec::new(),
        )
        .await;

    let full_len = graph.nodes.get(&node_id).unwrap().inputs.len();
    assert!(full_len >= 4, "test op needs several inputs, got {full_len}");

    // Simulate an older save that only stored the first 3 inputs, marking the
    // last surviving one so we can prove its value is preserved.
    {
        let node = graph.nodes.get_mut(&node_id).unwrap();
        node.inputs.truncate(3);
        node.inputs[2].value = Value::Decimal(0.4242);
    }

    // Round-trip through the real save/load path.
    let path = std::env::temp_dir().join(format!("mangler_pad_test_{}.mangler.json", get_id()));
    graph.save_path = Some(path.clone());
    graph.save_to_file().expect("save");

    let loaded = Graph::load(path.clone(), None, None, false).expect("load");
    let _ = std::fs::remove_file(&path);

    let node = loaded.nodes.get(&node_id).expect("node survives load");
    assert_eq!(
        node.inputs.len(),
        full_len,
        "short saved input vector should be padded back to the current schema length"
    );
    match &node.inputs[2].value {
        Value::Decimal(v) => assert!((v - 0.4242).abs() < 1e-9, "preserved value changed: {v}"),
        other => panic!("preserved slot should keep its saved Decimal value, got {other:?}"),
    }
}
