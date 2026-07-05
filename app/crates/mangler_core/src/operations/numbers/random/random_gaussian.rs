//! Random Gaussian generation operation for the node graph.
//!
//! Generates a normally distributed random decimal each time the node is
//! triggered, using the Box–Muller transform over `fastrand::f32()`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::time::Instant;

/// Node operation that generates a normally distributed random decimal.
///
/// Takes a trigger plus `mean` and `std dev` inputs and outputs a sample from
/// a Gaussian distribution via the Box–Muller transform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberRandomGaussian {}

impl OpNumberRandomGaussian {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random gaussian".to_string(),
            description: "Generates a normally distributed random decimal.".to_string(),
            help: "Draws a sample from a normal (Gaussian) distribution with the given mean and standard deviation each time the generate trigger fires. It uses the Box–Muller transform to turn two uniform fastrand values into one normally distributed value.\n\nAbout 68% of samples fall within one standard deviation of the mean and 95% within two. The fastrand PRNG is non-cryptographic and thread-local, so results are not reproducible between runs.".to_string(),
        }
    }

    /// Creates the default input list: a trigger, `mean` (0.0), and `std dev` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None)
                .with_description("Trigger that causes the node to draw a new sample."),
            Input::new("mean".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Center of the distribution."),
            Input::new("std dev".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Standard deviation; the spread of the distribution."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Normally distributed random decimal.")
        ]
    }

    /// Executes the node: draws a Gaussian sample via the Box–Muller transform.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let mean_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let std_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(mean) = mean_converted.unwrap() else { unreachable!() };
        let Value::Decimal(std) = std_converted.unwrap() else { unreachable!() };

        let u1 = fastrand::f32().max(1e-7);
        let u2 = fastrand::f32();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        let output = mean + std * z;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "random_gaussian_tests.rs"]
mod tests;
