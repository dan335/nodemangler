use image::RgbaImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use image::io::Reader as ImageReader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageInputFile {}

impl OperationImageInputFile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image from file".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("path".to_string(), Value::Path(PathBuf::new()), InputSettings::Path{
                extension_filter: ValueType::file_extensions(&ValueType::DynamicImage),
                set_directory: None,
                set_file_name: None,
                set_title: Some("image".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(i32::default()), None),
            Output::new("height".to_string(), Value::Integer(i32::default()), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut width = 0;
        let mut height = 0;
        let mut img = None;

        //let Ok(Value::Path { name, path, file_extensions }) = inputs[0].value.try_convert_to(ValueType::Path) else { return Err(OperationError { message: "Unable to convert to path.".to_string() })};
        let Value::Path(path) = &inputs[0].value else { return Err(OperationError { message: "Unable to convert to path.".to_string() })};

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
            Err(OperationError { message: "Error grabbing image from clipboard.".to_string() })
        }
    }
}
