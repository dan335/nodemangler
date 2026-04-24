//! Division operation for the node graph.
//!
//! Computes `a / b` for integer and decimal types. Returns an error if `b` is zero.
//! Mixed integer/decimal types promote to decimal.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that divides `a` by `b`.
///
/// Returns an error when `b` is zero. Supports integer and decimal types;
/// mixed types promote to decimal. Integer division truncates toward zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathDivide {}

impl OpNumberMathDivide {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "divide".to_string(),
            description: "Divides two numbers.".to_string(),
            help: "Computes a / b for integer or decimal inputs. Mixed integer/decimal types promote to decimal, while two integers stay in integer arithmetic with truncation toward zero.\n\nSetting b to zero raises a division-by-zero error rather than producing infinity or NaN, so the node surfaces the problem clearly instead of silently propagating a bad value downstream.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (a and b), defaulting to 1.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Dividend (numerator) of the division."),
            Input::new("b".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Divisor (denominator); produces an error when set to zero.")
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Quotient a / b; integer inputs truncate toward zero.")
        ]
    }

    /// Executes the division: computes `a / b`, returning an error if `b` is zero.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        // Check for division by zero
        let is_zero = match &inputs[1].value {
            Value::Integer(b) => *b == 0,
            Value::Decimal(b) => *b == 0.0,
            _ => false,
        };
        if is_zero {
            return Err(OperationError {
                input_errors: vec![(1, "Division by zero.".to_string())], node_error: None,
            });
        }

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 / *b),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a / *b),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(*a / *b),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(*a / *b as f32),

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
#[path = "divide_tests.rs"]
mod tests;
