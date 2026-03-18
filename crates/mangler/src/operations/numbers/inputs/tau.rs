//! Tau constant node for the node graph.
//!
//! Outputs the mathematical constant tau (2*pi = 6.28318...).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that outputs the constant tau.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputTau {}

impl OpNumberInputTau {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tau".to_string(),
            description: "Outputs the constant tau (2*pi = 6.28318...).".to_string(),
        }
    }

    /// Creates the default input list: no inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(std::f32::consts::TAU), None)
        ]
    }

    /// Executes the node: returns tau.
    pub async fn run(_inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(std::f32::consts::TAU),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tau_settings() {
        let s = OpNumberInputTau::settings();
        assert_eq!(s.name, "tau");
        assert_eq!(OpNumberInputTau::create_inputs().len(), 0);
        assert_eq!(OpNumberInputTau::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_tau_value() {
        let mut inputs = vec![];
        let result = OpNumberInputTau::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::TAU).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
