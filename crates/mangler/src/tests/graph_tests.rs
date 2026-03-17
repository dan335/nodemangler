#[cfg(test)]
mod graph_tests {
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
        assert_eq!(node.inputs.len(), 3); // a, b, mask
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
}
