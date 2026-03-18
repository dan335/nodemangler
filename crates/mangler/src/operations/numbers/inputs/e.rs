//! Euler's number constant node for the node graph.
//!
//! Outputs Euler's number e (2.71828...).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that outputs Euler's number e.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputE {}

impl OpNumberInputE {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "e".to_string(),
            description: "Outputs Euler's number e (2.71828...).".to_string(),
        }
    }

    /// Creates the default input list: no inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(std::f32::consts::E), None)
        ]
    }

    /// Executes the node: returns e.
    pub async fn run(_inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(std::f32::consts::E),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_e_settings() {
        let s = OpNumberInputE::settings();
        assert_eq!(s.name, "e");
        assert_eq!(OpNumberInputE::create_inputs().len(), 0);
        assert_eq!(OpNumberInputE::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_e_value() {
        let mut inputs = vec![];
        let result = OpNumberInputE::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::E).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
