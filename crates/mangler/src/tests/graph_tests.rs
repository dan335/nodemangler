// #[cfg(test)]

// mod graph_tests {
//     use tokio::sync::mpsc;

//     use crate::{
//         get_id, graph::Graph, operation::Operation, value::Value, AddNodeType, GraphChangedMessage,
//         NodeChangedMessage,
//     };

//     #[tokio::test]
//     async fn test_add() {
//         let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(32);
//         let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(32);

//         let graph_option = Graph::new(get_id(), tx_node_changed, tx_graph_changed, false);
//         assert!(graph_option.is_ok());
//         let mut graph = graph_option.unwrap();
//         let add_node_id = graph
//             .add_node(
//                 get_id(),
//                 AddNodeType::Operation(Operation::NumberMathAdd),
//                 glam::Vec2::ZERO,
//             )
//             .await;

//         let add_node_option = graph.nodes.get_mut(&add_node_id);
//         assert!(add_node_option.is_some());
//         let add_node = add_node_option.unwrap();

//         assert_eq!(add_node.inputs.len(), 2);
//         assert_eq!(add_node.outputs.len(), 1);
//         add_node.inputs[0].value = Value::Decimal(5.0);
//         add_node.inputs[1].value = Value::Decimal(10.0);
//         assert_eq!(add_node.outputs[0].value, Value::Decimal(0.0));

//         graph.run().await;

//         let add_node_option = graph.nodes.get_mut(&add_node_id);
//         assert!(add_node_option.is_some());
//         let add_node = add_node_option.unwrap();
//         assert_eq!(add_node.outputs[0].value, Value::Decimal(15.0));
//     }
// }
