//! Subgraph operation: embeds an entire graph inside a single node.
//!
//! This operation loads a graph from a `.mangle.json` file and executes it as a
//! nested subgraph within the parent graph. This is experimental/WIP -- the
//! subgraph is lazily loaded on first run and executed on a dedicated thread
//! with its own tokio runtime.

use crate::graph::Graph;
use crate::node::{Data, SubgraphData};
use crate::{Value, AddNodeMessage, RemoveNodeMessage, AddConnectionMessage, RemoveConnectionMessage, SetNodeInputMessage, NodeInputChangedMessage, NodeOutputChangedMessage, AddedNodeMessage, RemovedNodeMessage, LoadedNodeMessage, AddedConnectionMessage, RemovedConnectionMessage, GraphMessage, NodePosition};
use crate::output::Output;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse};
use crate::value::ValueType;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use std::thread;

use std::path::PathBuf;
use tokio::{
    time::{Duration, Instant},
};


/// Operation that wraps an entire node graph as a single node.
///
/// On first execution, the subgraph is loaded from the file path specified
/// in input 0. Subsequent executions reuse the loaded graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSubgraph {}

impl OperationSubgraph {
    /// Returns the display settings for the subgraph node.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "subgraph".to_string(),
        }
    }

    /// Creates the default inputs: a single file path to the `.mangle.json` graph file.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input {
                name: "file path".to_string(),
                value: Value::Text("C:\\temp\\New_Graph.mangle.json".to_string()),
                connection: None,
                valid_types: vec![],
            }
        ]
    }

    /// Creates outputs. Currently empty -- subgraph outputs are not yet wired through.
    pub fn create_outputs() -> Vec<Output> {
        vec![]
    }

    /// Creates the initial data slots, including a `None` subgraph placeholder
    /// that will be populated on first run.
    pub fn create_data() -> Vec<Data> {
        vec![
            Data::Subgraph(None)
        ]
    }

    /// Executes the subgraph operation.
    ///
    /// On first invocation, loads the graph from the file path in input 0 and
    /// stores it in the data slot. On subsequent calls, runs the already-loaded
    /// graph. The subgraph execution happens on a spawned thread with its own
    /// tokio runtime to avoid blocking the parent runtime.
    pub async fn run(inputs: &Vec<Input>, data: &mut Vec<Data>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        if let Some(mut data) = data.get_mut(0) {
            // Temporary holder: we build the subgraph data outside the borrow,
            // then swap it in afterwards to satisfy the borrow checker.
            let mut data_subgraph: Option<Data> = None;

            if let Data::Subgraph(subgraph) = data {
                // Lazy initialization: only load the graph file on first run
                if subgraph.is_none() {
                    let Ok(Value::Text(path_string)) = inputs[0].value.try_convert_to(ValueType::Text) else { return Err(OperationError { message: "Unable to get path string.".to_string() })};
                    let path = PathBuf::from(path_string);

                    let graph_result = Graph::load(
                        path,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    );
        
                    match graph_result {
                        Ok(graph) => {
                            // let id = graph.id.clone();
                            // let name = graph.name.clone();
                            // let save_path = graph.save_path.clone();
                            
                            let subgraph_data = SubgraphData {
                                graph
                            };
        
                            data_subgraph = Some(Data::Subgraph(Some(subgraph_data)));
                        },
                        Err(_) =>  {
                            return Err(OperationError{ message: "Error creating graph.".to_string() });
                        }
                    }
                }
            }

            // If we just loaded a new subgraph, extract and run it
            if let Some(asdf) = data_subgraph {
                let Data::Subgraph(subgraph_data) = asdf;

                if let Some(mut subgraph) = subgraph_data {

                    // let rt = Runtime::new().unwrap();

                    // rt.block_on(async {
                    //     subgraph.graph.run().await;
                    //     println!("Async function completed synchronously.");
                    // });

                    // Spawn a dedicated thread with its own tokio runtime to run the
                    // subgraph, avoiding nesting async runtimes on the parent executor.
                    thread::spawn(move || {
                        let rt = Runtime::new().unwrap();

                        rt.block_on(async {
                            subgraph.graph.run().await;
                            println!("Async function completed synchronously.");
                        });
                    }).join().expect("Thread panicked");

                    return Ok(OperationResponse { ai_cost_usd: None,
                        time: Instant::now().duration_since(start_time),
                        responses: vec![],
                    });
                }
            }
            
        }

        // if possible_data.is_none() {
        //     // create graph

        //     // get file path to graph
        //     let Ok(Value::String(path_string)) = inputs[0].value.try_convert_to(ValueType::String) else { return Err(OperationError { message: "Unable to get path string.".to_string() })};
        //     let path = PathBuf::from(path_string);

        //     // // create channels
        //     // let (tx_add_node, mut rx_add_node) = mpsc::channel::<AddNodeMessage>(32);
        //     // let (tx_remove_node, mut rx_remove_node) = mpsc::channel::<RemoveNodeMessage>(32);
        //     // let (tx_add_connection, mut rx_add_connection) = mpsc::channel::<AddConnectionMessage>(32);
        //     // let (tx_remove_connection, mut rx_remove_connection) =
        //     //     mpsc::channel::<RemoveConnectionMessage>(32);
        //     // let (tx_set_input, mut rx_set_input) = mpsc::channel::<SetNodeInputMessage>(32);
        //     // let (tx_input_changed, rx_input_changed) = mpsc::channel::<NodeInputChangedMessage>(32);
        //     // let (tx_output_changed, rx_output_changed) = mpsc::channel::<NodeOutputChangedMessage>(32);
        //     // let (tx_added_node, rx_added_node) = mpsc::channel::<AddedNodeMessage>(32);
        //     // let (tx_removed_node, rx_removed_node) = mpsc::channel::<RemovedNodeMessage>(32);
        //     // let (tx_loaded_node, rx_loaded_node) = mpsc::channel::<LoadedNodeMessage>(32);
        //     // let (tx_added_connection, rx_added_connection) =
        //     //     mpsc::channel::<AddedConnectionMessage>(32);
        //     // let (tx_removed_connection, rx_removed_connection) =
        //     //     mpsc::channel::<RemovedConnectionMessage>(32);
        //     // let (tx_graph_setting, mut rx_graph_setting) = mpsc::channel::<GraphMessage>(32);
        //     // let (tx_node_position, mut rx_node_position) = mpsc::channel::<NodePosition>(32);

        //     // create path from input
            
        // }

        // if let Some(data) = possible_data {
        //     if let Data::Subgraph(subgraph_data) = data {
        //         if let Some(subgraph) = subgraph_data {

        //             let rt = Runtime::new().unwrap();

        //             rt.block_on(async {
        //                 subgraph.graph.run().await;
        //                 println!("Async function completed synchronously.");
        //             });

        //             return Ok(OperationResponse { ai_cost_usd: None,
        //                 time: Instant::now().duration_since(start_time),
        //                 responses: vec![],
        //             });
        //         }
        //     }
        // }

        Err(OperationError{ message: "Error in subgraph.".to_string() })
    }
}