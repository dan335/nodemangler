/// AI image editing operation.
///
/// Sends an image and a text prompt to the OpenAI image edits API
/// and returns the edited image along with its dimensions.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::shared;

/// OpenAI image edits API endpoint.
const OPENAI_EDITS_URL: &str = "https://api.openai.com/v1/images/edits";

/// Operation that edits an image using a text prompt via the OpenAI API.
///
/// Inputs: image, prompt, model, size, api key.
/// Outputs: edited image, width, height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpAiEdit {}

impl OpAiEdit {
    /// Returns node metadata.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ai edit".to_string(),
            description: "Edits an image using a text prompt via OpenAI.".to_string(),
        }
    }

    /// Creates the input definitions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("prompt".to_string(), Value::Text(String::new()), Some(InputSettings::MultiLineText), None),
            Input::new("model".to_string(), Value::Text("dall-e-2".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("size".to_string(), Value::Text("1024x1024".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("api key".to_string(), Value::Text(String::new()), Some(InputSettings::SingleLineText), None),
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

    /// Builds the multipart form for the OpenAI image edits endpoint.
    pub fn build_multipart_form(
        png_bytes: Vec<u8>,
        prompt: &str,
        model: &str,
        size: &str,
    ) -> reqwest::multipart::Form {
        let image_part = reqwest::multipart::Part::bytes(png_bytes)
            .file_name("image.png")
            .mime_str("image/png")
            .unwrap();

        reqwest::multipart::Form::new()
            .part("image", image_part)
            .text("prompt", prompt.to_string())
            .text("model", model.to_string())
            .text("size", size.to_string())
            .text("response_format", "b64_json".to_string())
    }

    /// Executes the operation: sends image + prompt to OpenAI edits endpoint.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let prompt_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let model_converted = convert_input(inputs, 2, ValueType::Text, &mut input_errors);
        let size_converted = convert_input(inputs, 3, ValueType::Text, &mut input_errors);
        let api_key_converted = convert_input(inputs, 4, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Image { data: image_data, .. } = image_converted.unwrap() else { unreachable!() };
        let Value::Text(prompt) = prompt_converted.unwrap() else { unreachable!() };
        let Value::Text(model) = model_converted.unwrap() else { unreachable!() };
        let Value::Text(size) = size_converted.unwrap() else { unreachable!() };
        let Value::Text(api_key_input) = api_key_converted.unwrap() else { unreachable!() };

        // Validate prompt is not empty.
        if prompt.trim().is_empty() {
            return Err(OperationError {
                input_errors: vec![(1, "Prompt cannot be empty.".to_string())],
                node_error: None,
            });
        }

        // Resolve API key.
        let api_key = match shared::resolve_api_key(&api_key_input, "OPENAI_API_KEY") {
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
        let form = Self::build_multipart_form(png_bytes, &prompt, &model, &size);
        let json = match shared::make_ai_multipart_request(OPENAI_EDITS_URL, &api_key, form).await {
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
#[path = "edit_tests.rs"]
mod tests;
