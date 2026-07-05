//! Image difference (error) metrics between two images.
//!
//! Compares two images pixel-for-pixel over their RGB channels and emits the
//! classic error metrics: mean squared error, root mean squared error, mean
//! absolute error, and peak signal-to-noise ratio. When the two images differ
//! in size, image b is resized to match image a before comparison.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::pixel_rgba;

/// Operation that computes MSE/RMSE/MAE/PSNR between two images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageDifference {}

impl OpNumberImageDifference {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image difference".to_string(),
            description: "Measures per-pixel error (MSE, RMSE, MAE, PSNR) between two images.".to_string(),
            help: "Compares image a against image b over their red, green, and blue channels and reports four error metrics: mean squared error (MSE), root mean squared error (RMSE), mean absolute error (MAE), and peak signal-to-noise ratio (PSNR). If image b's dimensions differ from image a's, image b is resized to match before comparison.\n\nPSNR assumes a maximum intensity of 1.0 and is expressed in decibels; identical images report a capped 100 dB (and zero error). Higher PSNR and lower MSE/RMSE/MAE mean the images are more alike. Alpha is ignored — only RGB is compared.".to_string(),
        }
    }

    /// Creates the input ports: the two images to compare.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image a".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Reference image."),
            Input::new("image b".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image compared against image a; resized to match if its size differs."),
        ]
    }

    /// Creates the output ports: the four error metrics.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("mse".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean squared error across the RGB channels."),
            Output::new("rmse".to_string(), Value::Decimal(0.0), None)
                .with_description("Root mean squared error (square root of MSE)."),
            Output::new("mae".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean absolute error across the RGB channels."),
            Output::new("psnr".to_string(), Value::Decimal(100.0), None)
                .with_description("Peak signal-to-noise ratio in dB (max intensity 1.0; capped at 100)."),
        ]
    }

    /// Executes the image-difference computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: a, change_id: _ } = a_converted.unwrap() else { unreachable!() };
        let Value::Image { data: b, change_id: _ } = b_converted.unwrap() else { unreachable!() };

        let (wa, ha) = a.dimensions();
        let b_resized = if b.dimensions() != (wa, ha) { b.resize(wa, ha) } else { (*b).clone() };

        let mut sumsq = 0.0f64;
        let mut sumabs = 0.0f64;
        let mut nch = 0.0f64;
        for (pa, pb) in a.pixels().zip(b_resized.pixels()) {
            let (ar, ag, ab, _) = pixel_rgba(pa);
            let (br, bg, bb, _) = pixel_rgba(pb);
            let dr = (ar - br) as f64;
            let dg = (ag - bg) as f64;
            let db = (ab - bb) as f64;
            sumsq += dr * dr + dg * dg + db * db;
            sumabs += dr.abs() + dg.abs() + db.abs();
            nch += 3.0;
        }

        let (mse, rmse, mae, psnr) = if nch == 0.0 {
            (0.0f64, 0.0f64, 0.0f64, 100.0f64)
        } else {
            let mse = sumsq / nch;
            let mae = sumabs / nch;
            let rmse = mse.sqrt();
            let psnr = if mse <= 1e-12 { 100.0 } else { 10.0 * (1.0 / mse).log10() };
            (mse, rmse, mae, psnr)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(mse as f32) },
                OutputResponse { value: Value::Decimal(rmse as f32) },
                OutputResponse { value: Value::Decimal(mae as f32) },
                OutputResponse { value: Value::Decimal(psnr as f32) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "image_difference_tests.rs"]
mod tests;
