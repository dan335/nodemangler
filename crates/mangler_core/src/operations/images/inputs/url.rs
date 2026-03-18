//! Image-from-URL input operation.
//!
//! Fetches an image from a remote URL via an async HTTP GET request and
//! outputs the decoded image, its dimensions, and the resolved URL string.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that downloads and decodes an image from a URL.
///
/// Uses `reqwest` to perform an async HTTP GET, then decodes the response
/// bytes into a `DynamicImage`. Outputs the image, its width and height,
/// and the URL string that was fetched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputUrl {}

impl OpImageInputUrl {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from url".to_string(),
            description: "Grabs an image from a url.".to_string(),
        }
    }

    /// Creates the input definitions: a single URL string with multi-line text editing.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("url".to_string(), Value::Text("https://i.imgur.com/3aDSTiBl.jpg".to_string()), Some(InputSettings::MultiLineText), None),
        ]
    }

    /// Creates the output definitions: the decoded image, width, height, and the fetched URL.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(i32::default()), None),
            Output::new("height".to_string(), Value::Integer(i32::default()), None),
            Output::new("url".to_string(), Value::Text("".to_string()), None),
        ]
    }

    /// Executes the operation: fetches the URL, downloads the image bytes, and decodes them.
    ///
    /// Returns an error if the HTTP request fails, the response cannot be read as bytes,
    /// or the image format is unsupported.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let url_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Text(url) = url_converted.unwrap() else { unreachable!() };

        // run node
        if let Ok(image_response) =  reqwest::get(url.clone()).await {
            if let Ok(image_bytes) = image_response.bytes().await {
                if let Ok(image) = image::load_from_memory(&image_bytes) {
                    let width = image.width() as i32;
                    let height = image.height() as i32;

                    Ok(OperationResponse {
                        time: Instant::now().duration_since(start_time), 
                        responses: vec![
                            OutputResponse { value: Value::DynamicImage { data: Arc::new(image), change_id: get_id() } },
                            OutputResponse { value: Value::Integer(width) },
                            OutputResponse { value: Value::Integer(height) },
                            OutputResponse { value: Value::Text(url) },
                        ],
                    })
                } else {
                    Err(OperationError{ input_errors, node_error: Some("Format not supported.".to_string())  })
                }
            } else {
                Err(OperationError{ input_errors, node_error: Some("Could not parse into bytes.".to_string())  })
            }
        } else {
            Err(OperationError{ input_errors, node_error: Some("Error getting url.".to_string())  })
        }

        
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_input_settings() {
        let s = OpImageInputUrl::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageInputUrl::create_inputs().is_empty());
        assert!(!OpImageInputUrl::create_outputs().is_empty());
    }

    #[tokio::test]
    async fn test_url_input_exact_settings() {
        let s = OpImageInputUrl::settings();
        assert_eq!(s.name, "from url");
        assert_eq!(OpImageInputUrl::create_inputs().len(), 1);
        assert_eq!(OpImageInputUrl::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_url_input_invalid_url_returns_error() {
        use crate::input::Input;
        let mut inputs = vec![
            Input::new("url".to_string(), Value::Text("not_a_valid_url".to_string()), None, None),
        ];
        let result = OpImageInputUrl::run(&mut inputs).await;
        assert!(result.is_err(), "invalid url should return error");
    }
}
