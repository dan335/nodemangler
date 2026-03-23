/// AI image generation operation.
///
/// Sends a text prompt to the OpenAI image generation API (DALL-E 3 / gpt-image-1)
/// and returns the generated image along with its dimensions and the revised prompt.

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

/// OpenAI image generation API endpoint.
const OPENAI_IMAGES_URL: &str = "https://api.openai.com/v1/images/generations";

/// Operation that generates an image from a text prompt via the OpenAI API.
///
/// Inputs: prompt, model, size, quality, api key.
/// Outputs: generated image, width, height, revised prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpAiGenerate {}

impl OpAiGenerate {
    /// Returns node metadata.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ai generate".to_string(),
            description: "Generates an image from a text prompt using OpenAI (DALL-E).".to_string(),
        }
    }

    /// Creates the input definitions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("prompt".to_string(), Value::Text(String::new()), Some(InputSettings::MultiLineText), None),
            Input::new("model".to_string(), Value::Text("dall-e-3".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("size".to_string(), Value::Text("1024x1024".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("quality".to_string(), Value::Text("standard".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("api key".to_string(), Value::Text(String::new()), Some(InputSettings::SingleLineText), None),
        ]
    }

    /// Creates the output definitions.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
            Output::new("width".to_string(), Value::Integer(0), None),
            Output::new("height".to_string(), Value::Integer(0), None),
            Output::new("revised prompt".to_string(), Value::Text(String::new()), None),
        ]
    }

    /// Builds the JSON request body for the OpenAI images/generations endpoint.
    pub fn build_request_body(prompt: &str, model: &str, size: &str, quality: &str) -> serde_json::Value {
        serde_json::json!({
            "model": model,
            "prompt": prompt,
            "n": 1,
            "size": size,
            "quality": quality,
            "response_format": "b64_json"
        })
    }

    /// Executes the operation: sends prompt to OpenAI and decodes the returned image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let prompt_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let model_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let size_converted = convert_input(inputs, 2, ValueType::Text, &mut input_errors);
        let quality_converted = convert_input(inputs, 3, ValueType::Text, &mut input_errors);
        let api_key_converted = convert_input(inputs, 4, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Text(prompt) = prompt_converted.unwrap() else { unreachable!() };
        let Value::Text(model) = model_converted.unwrap() else { unreachable!() };
        let Value::Text(size) = size_converted.unwrap() else { unreachable!() };
        let Value::Text(quality) = quality_converted.unwrap() else { unreachable!() };
        let Value::Text(api_key_input) = api_key_converted.unwrap() else { unreachable!() };

        // Validate prompt is not empty.
        if prompt.trim().is_empty() {
            return Err(OperationError {
                input_errors: vec![(0, "Prompt cannot be empty.".to_string())],
                node_error: None,
            });
        }

        // Resolve API key.
        let api_key = match shared::resolve_api_key(&api_key_input, "OPENAI_API_KEY") {
            Ok(key) => key,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Build request body and send.
        let body = Self::build_request_body(&prompt, &model, &size, &quality);
        let json = match shared::make_ai_request(OPENAI_IMAGES_URL, &api_key, body).await {
            Ok(json) => json,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Parse the response.
        let (image, width, height, revised_prompt) = match shared::parse_openai_image_response(&json) {
            Ok(result) => result,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
                OutputResponse { value: Value::Text(revised_prompt.unwrap_or_default()) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
