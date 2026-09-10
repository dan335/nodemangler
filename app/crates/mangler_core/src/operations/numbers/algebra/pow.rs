//! Power (exponentiation) operation for the node graph.
//!
//! Computes `base^exponent` using f64 intermediate precision to reduce rounding
//! errors, then casts the result back to f32.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that raises a base to an exponent.
///
/// Both inputs are converted to decimal. The computation uses f64 precision
/// internally (`(base as f64).powf(exponent as f64)`) before casting to f32.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathPow {}

impl OpNumberMathPow {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "power".to_string(),
            description: "Raises base to an exponent.".to_string(),
            help: "Computes base^exponent. The computation is performed in f64 internally to preserve precision, then cast back to f32 for the output.\n\nSupports fractional and negative exponents. Raising a negative base to a fractional exponent is mathematically undefined for real numbers and will return NaN. Use nth root for the complementary operation.".to_string(),
        }
    }

    /// Creates the default input list: `base` (2.0) and `exponent` (2.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("base".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Base value that is raised to the exponent."),
            Input::new("exponent".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Power to raise the base to; fractional exponents are supported."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Result of base raised to exponent, computed with f64 precision.")
        ]
    }

    /// Executes the power operation: computes `base^exponent`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Decimal(base) = 0,
            Decimal(exponent) = 1,
        }

        let result = (base as f64).powf(exponent as f64) as f32;

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "pow_tests.rs"]
mod tests;
