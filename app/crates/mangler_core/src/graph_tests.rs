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
    assert!(!graph.is_dirty);
    assert!(!graph.is_subgraph);
}

#[tokio::test]
async fn test_add_node() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::ZERO,
        )
        .await;

    assert!(graph.nodes.contains_key(&node_id));
    assert!(graph.is_dirty);

    let node = graph.nodes.get(&node_id).unwrap();
    assert_eq!(node.inputs.len(), 2); // a, b
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.settings.name, "add");
}

#[tokio::test]
async fn test_add_decimal_input_node() {
    let mut graph = create_test_graph();
    let node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::ZERO,
        )
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
            glam::Vec2::ZERO,
        )
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
            glam::Vec2::ZERO,
        )
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
            glam::Vec2::new(0.0, 0.0),
        )
        .await;

    let add_node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(200.0, 0.0),
        )
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
            glam::Vec2::ZERO,
        )
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
            glam::Vec2::new(0.0, 0.0),
        )
        .await;
    let input_b_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberInputDecimal),
            glam::Vec2::new(0.0, 100.0),
        )
        .await;

    // Create add node
    let add_node_id = graph
        .add_node(
            get_id(),
            AddNodeType::Operation(Operation::OpNumberMathAdd),
            glam::Vec2::new(200.0, 0.0),
        )
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
            glam::Vec2::ZERO,
        )
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
            glam::Vec2::ZERO,
        )
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
    graph.set_save_path(std::path::PathBuf::from("/tmp/test.mangle.json"));
    assert_eq!(graph.save_path, Some(std::path::PathBuf::from("/tmp/test.mangle.json")));
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add1_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
        .await;
    let add2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(400.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add1_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
        .await;
    let add2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 100.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(50.0, 75.0))
        .await;
    graph.set_input(node_id.clone(), 0, Value::Decimal(42.0));

    let tmp_path = std::env::temp_dir().join(format!("test_graph_{}.mangle.json", get_id()));
    graph.set_save_path(tmp_path.clone());
    graph.save_to_file();

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

#[tokio::test]
async fn test_save_to_file_subgraph_is_noop() {
    let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
    let mut graph = Graph::new(get_id(), tx_nc, tx_gc, true).unwrap();

    let tmp_path = std::env::temp_dir().join(format!("test_subgraph_{}.mangle.json", get_id()));
    graph.set_save_path(tmp_path.clone());
    graph.save_to_file();

    // File should NOT be created for subgraphs
    assert!(!tmp_path.exists());
}

#[tokio::test]
async fn test_save_to_file_no_path_is_noop() {
    let graph = create_test_graph();
    assert!(graph.save_path.is_none());
    // Should be a no-op, not panic
    graph.save_to_file();
}

// === load() error cases ===

#[test]
fn test_load_nonexistent_file() {
    let result = Graph::load(
        std::path::PathBuf::from("/nonexistent/path/graph.mangle.json"),
        None, None, false,
    );
    assert!(result.is_err());
}

#[test]
fn test_load_invalid_json() {
    let tmp_path = std::env::temp_dir().join(format!("test_bad_json_{}.mangle.json", get_id()));
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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

    let id1 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO).await;
    let id2 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO).await;
    let id3 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO).await;

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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO)
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO)
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO)
        .await;
    let int2_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::new(0.0, 100.0))
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0))
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
async fn test_connect_to_condition_does_not_adapt_types() {
    // Connecting to the condition input (index 0) should NOT trigger type
    // adaptation since condition is not accepts_any_type.
    let mut graph = create_test_graph();

    let bool_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicInputBool), glam::Vec2::ZERO)
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicInputBool), glam::Vec2::ZERO)
        .await;
    let int_true_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::new(0.0, 100.0))
        .await;
    let int_false_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::new(0.0, 200.0))
        .await;
    let select_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpLogicFlowSelect), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let add_id = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let source_b = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let consumer = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let source_b = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
        .await;
    let consumer = graph
        .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
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
