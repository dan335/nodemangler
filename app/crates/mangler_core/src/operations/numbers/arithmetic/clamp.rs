//! Clamp operation for the node graph.
//!
//! Restricts a value to lie within a specified `[min, max]` range.
//! The min and max bounds are converted to decimals for comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that clamps a number between a minimum and maximum.
///
/// Accepts integer or decimal input. The `min` and `max` bounds are converted
/// to decimal for the comparison. Integer inputs produce integer outputs
/// (the clamped value is rounded back to i32).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathClamp {}

impl OpNumberMathClamp {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clamp".to_string(),
            description: "Clamps a number between two values.".to_string(),
            help: "Restricts input a to the [min, max] range: values below min are raised to min, values above max are lowered to max. The min and max bounds are converted to decimal for the comparison.\n\nInteger inputs produce integer outputs, with the clamped value rounded back to i32. Useful for normalizing values to the 0-1 range or keeping parameters inside valid ranges. If min is greater than max, the two bounds are treated as swapped so the range stays valid instead of panicking.".to_string(),
        }
    }

    /// Creates the default input list: value `a`, `min` (0.0), and `max` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Value to restrict inside the min and max range."),
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Lower bound; inputs below this are raised to min."),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Upper bound; inputs above this are lowered to max."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Input a constrained to the [min, max] range.")
        ]
    }

    /// Executes the clamp: restricts input `a` to the `[min, max]` range.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(min)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert 'min' to Decimal.".to_string())], node_error: None })};
        let Ok(Value::Decimal(max)) = inputs[2].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(2, "Unable to convert 'max' to Decimal.".to_string())], node_error: None })};

        // `f32::clamp` panics if `min > max` (its internal assert requires
        // `min <= max`), which is easy to hit with a wired-up min/max pair.
        // Normalize the bounds here instead of calling `.clamp()` directly:
        // `lo`/`hi` are the actual low/high bounds regardless of the order
        // the caller supplied them in, so this is panic-free even when
        // `min > max` (bounds treated as swapped) or one of them is NaN
        // (`f32::min`/`f32::max` prefer the non-NaN operand).
        let lo = min.min(max);
        let hi = min.max(max);

        let value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer((*a as f32).max(lo).min(hi).round() as i32),
            Value::Decimal(a) => Value::Decimal((*a).max(lo).min(hi)),

            _ => {return Err(OperationError {
                input_errors: vec![], node_error: Some("Error converting.".to_string()),
            });}
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "clamp_tests.rs"]
mod tests;
