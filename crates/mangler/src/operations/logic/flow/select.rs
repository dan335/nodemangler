//! Select (multiplexer) operation.
//!
//! Chooses between two values based on a boolean condition. When the condition
//! is `true`, the "if true" input is forwarded; otherwise, the "if false" input
//! is forwarded. The output value retains its original type (no coercion is
//! applied to the selected branch).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Select (mux) node that picks between two values based on a boolean condition.
///
/// Acts as a ternary operator: `condition ? if_true : if_false`. The condition
/// input is coerced to boolean, but the two branch inputs are passed through
/// without type conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicFlowSelect {}

impl OpLogicFlowSelect {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "select".to_string(),
            description: "Selects between two values based on a condition.".to_string(),
        }
    }

    /// Creates the default inputs: a boolean condition, and two decimal branch values ("if true" and "if false").
    pub fn create_inputs() -> Vec<Input> {
        let mut if_true = Input::new("if true".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None);
        if_true.accepts_any_type = true;

        let mut if_false = Input::new("if false".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None);
        if_false.accepts_any_type = true;

        vec![
            Input::new("condition".to_string(), Value::Bool(false), None, None),
            if_true,
            if_false,
        ]
    }

    /// Creates the default output: a single decimal output defaulting to 0.0.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Evaluates the condition and forwards the appropriate branch value.
    ///
    /// Only the condition input is coerced (to boolean). The selected branch
    /// value is cloned and output as-is, preserving its original type.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let condition_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(condition) = condition_converted.unwrap() else { unreachable!() };

        // Forward the selected branch value without any type coercion
        let value = if condition {
            inputs[1].value.clone() // "if true" branch
        } else {
            inputs[2].value.clone() // "if false" branch
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn make_inputs(condition: Value, if_true: Value, if_false: Value) -> Vec<Input> {
        vec![
            Input::new("condition".to_string(), condition, None, None),
            Input::new("if true".to_string(), if_true, None, None),
            Input::new("if false".to_string(), if_false, None, None),
        ]
    }

    #[tokio::test]
    async fn test_select_true() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(10.0), Value::Decimal(20.0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6),
            other => panic!("Expected Decimal(10.0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_false() {
        let mut inputs = make_inputs(Value::Bool(false), Value::Decimal(10.0), Value::Decimal(20.0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 20.0).abs() < 1e-6),
            other => panic!("Expected Decimal(20.0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_integers() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Integer(42), Value::Integer(0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer(42), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_strings() {
        let mut inputs = make_inputs(
            Value::Bool(false),
            Value::String("yes".to_string()),
            Value::String("no".to_string()),
        );
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::String(v) => assert_eq!(v, "no"),
            other => panic!("Expected String(\"no\"), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_bools() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true), Value::Bool(false));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_select_condition_from_integer() {
        let mut inputs = make_inputs(Value::Integer(1), Value::Decimal(10.0), Value::Decimal(20.0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6),
            other => panic!("Expected Decimal(10.0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_settings() {
        let s = OpLogicFlowSelect::settings();
        assert_eq!(s.name, "select");
        assert_eq!(OpLogicFlowSelect::create_inputs().len(), 3);
        assert_eq!(OpLogicFlowSelect::create_outputs().len(), 1);
    }

    #[test]
    fn test_select_branch_inputs_accept_any_type() {
        let inputs = OpLogicFlowSelect::create_inputs();
        assert!(!inputs[0].accepts_any_type, "condition input should not accept any type");
        assert!(inputs[1].accepts_any_type, "if true input should accept any type");
        assert!(inputs[2].accepts_any_type, "if false input should accept any type");
    }

    #[tokio::test]
    async fn test_select_with_images() {
        use std::sync::Arc;
        use image::{DynamicImage, RgbaImage};
        use crate::get_id;

        let img_true = Value::DynamicImage {
            data: Arc::new(DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255])))),
            change_id: get_id(),
        };
        let img_false = Value::DynamicImage {
            data: Arc::new(DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, image::Rgba([0, 255, 0, 255])))),
            change_id: get_id(),
        };

        let mut inputs = make_inputs(Value::Bool(true), img_true.clone(), img_false.clone());
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::DynamicImage { .. }));

        let mut inputs = make_inputs(Value::Bool(false), img_true, img_false);
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::DynamicImage { .. }));
    }
}
