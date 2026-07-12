//! Curve input node operation.
//!
//! Provides a single user-drawn 2D curve (open path or closed shape) to the
//! graph. The curve is edited by selecting this node and drawing in the 2D
//! preview panel; there is no inline editor in the settings panel.

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;

/// Node operation that emits a user-drawn [`Curve`] value onto the graph.
///
/// Passes its single curve input through to the output. The curve is authored
/// in the 2D preview overlay, not in the node settings panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveInputCurve {}

impl OpCurveInputCurve {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curve".to_string(),
            description: "A drawable 2D curve input.".to_string(),
            help: "Emits a user-drawn 2D curve — an open path or a closed shape — onto the graph. Points are stored in normalized 0-1 coordinates (y-down, image convention), so the same curve maps onto any image size.\n\nEditing: select this node, then draw in the 2D preview panel. Click empty space to add points, drag a point to move it, double- or right-click a point to delete it. Toggle open/closed and the interpolation from the overlay strip. 'Smooth' fits a centripetal Catmull-Rom spline through the points; 'Bezier' adds a mirrored tangent handle to each point — drag either knob to shape the curvature (the twin follows, so the curve stays smooth).\n\nFeed the output into a 'rasterize curve' node to bake it into an image mask.".to_string(),
        }
    }

    /// Creates the default input: a single curve value (edited in the 2D preview).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            // `settings: None` — the widget is chosen by the value variant, like
            // the enum input types. The curve is drawn in the 2D preview panel.
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The 2D curve to emit; drawn in the 2D preview panel.")
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The curve from the input, passed through.")
        ]
    }

    /// Executes the node: passes the curve input through to the output.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Curve(curve) = input_converted.unwrap() else { unreachable!() };

        // run node
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Curve(curve),
            }],
        })
    }
}
