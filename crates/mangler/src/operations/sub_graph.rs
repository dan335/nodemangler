
use crate::Value;
use crate::output::Output;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse};

use std::time::Instant;


pub struct OperationSubgraph {}

impl OperationSubgraph {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "subgraph".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input {
                name: "file path".to_string(),
                value: Value::String("C:\\temp\\New_Graph.mangle".to_string()),
                connection: None,
                valid_types: vec![],
            }
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![]
    }

    pub async fn run(_inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![],
        })
    }
}