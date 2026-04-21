/// AI image variation operation.
///
/// Sends an image to the OpenAI image variations API and returns a
/// variation of the image along with its dimensions.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use crate::input::InputSettings;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::shared;

/// OpenAI image variations API endpoint.
const OPENAI_VARIATIONS_URL: &str = "https://api.openai.com/v1/images/variations";

/// Operation that creates a variation of an input image via the OpenAI API.
///
/// Inputs: image, model, size.
/// Outputs: variation image, width, height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpAiVariation {}

impl OpAiVariation {
    /// Returns node metadata.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ai variation".to_string(),
            description: "Creates a variation of an image using OpenAI.".to_string(),
        }
    }

    /// Creates the input definitions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("model".to_string(), Value::Text("dall-e-2".to_string()), Some(InputSettings::Dropdown {
                options: vec!["dall-e-2".to_string()],
            }), None),
            Input::new("size".to_string(), Value::Text("1024x1024".to_string()), Some(InputSettings::Dropdown {
                options: vec!["1024x1024".to_string(), "512x512".to_string(), "256x256".to_string()],
            }), None),
        ]
    }

    /// Creates the output definitions.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
            Output::new("width".to_string(), Value::Integer(0), None),
            Output::new("height".to_string(), Value::Integer(0), None),
        ]
    }

    /// Builds the multipart form for the OpenAI image variations endpoint.
    pub fn build_multipart_form(
        png_bytes: Vec<u8>,
        model: &str,
        size: &str,
    ) -> reqwest::multipart::Form {
        let image_part = reqwest::multipart::Part::bytes(png_bytes)
            .file_name("image.png")
            .mime_str("image/png")
            .unwrap();

        reqwest::multipart::Form::new()
            .part("image", image_part)
            .text("model", model.to_string())
            .text("size", size.to_string())
            .text("response_format", "b64_json".to_string())
    }

    /// Executes the operation: sends image to OpenAI variations endpoint.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let model_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let size_converted = convert_input(inputs, 2, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Image { data: image_data, .. } = image_converted.unwrap() else { unreachable!() };
        let Value::Text(model) = model_converted.unwrap() else { unreachable!() };
        let Value::Text(size) = size_converted.unwrap() else { unreachable!() };

        // Resolve API key from environment.
        let api_key = match shared::resolve_api_key("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Convert input image to PNG bytes for the multipart form.
        let png_bytes = match shared::float_image_to_png_bytes(&image_data) {
            Ok(bytes) => bytes,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Check cost limit before making the API call.
        if let Err(msg) = shared::check_cost_limit() {
            return Err(OperationError { input_errors: vec![], node_error: Some(msg) });
        }

        // Build and send the request.
        let form = Self::build_multipart_form(png_bytes, &model, &size);
        let json = match shared::make_ai_multipart_request(OPENAI_VARIATIONS_URL, &api_key, form).await {
            Ok(json) => json,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Parse the response.
        let (image, width, height, _) = match shared::parse_openai_image_response(&json) {
            Ok(result) => result,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Estimate and record cost.
        let cost = shared::estimate_cost_from_response(&json, &model, &size, "standard");
        shared::add_session_cost(cost);

        Ok(OperationResponse {
            ai_cost_usd: Some(cost),
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "variation_tests.rs"]
mod tests;
