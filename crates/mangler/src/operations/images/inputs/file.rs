use image::RgbaImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use image::io::Reader as ImageReader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputFile {}

impl OpImageInputFile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from file".to_string(),
            description: "Grabs an image from a file.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("path".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path{
                extension_filter: ValueType::file_extensions(&ValueType::DynamicImage),
                set_directory: None,
                set_file_name: None,
                set_title: Some("image".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let path_converted = inputs[0].value.try_convert_to(ValueType::Path);

        // gather errors
        if path_converted.is_err() { input_errors.push((0, path_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Path(path)) = path_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        let mut width = 0;
        let mut height = 0;
        let mut img = None;

        if let Ok(open) = ImageReader::open(path) {
            if let Ok(dynamic_image) = open.decode() {
                width = dynamic_image.width();
                height = dynamic_image.height();
                img = Some(dynamic_image);
            }
        }

        if let Some(value) = img {
            Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::DynamicImage { data: value, change_id: get_id() } },
                    OutputResponse { value: Value::Integer(width as i32) },
                    OutputResponse { value: Value::Integer(height as i32) },
                ],
            })
        } else {
            Err(OperationError { input_errors, node_error: Some("Error opening image.".to_string()) })
        }
    }
}
