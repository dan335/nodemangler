use image::RgbaImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse};
use crate::output::Output;
use crate::value::{Value};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageOutputFile {}

impl OperationImageOutputFile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image to file".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id() }, InputSettings::None, None),
            Input::new("path".to_string(), Value::Path(PathBuf::new()), InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: Some("image".to_string()),
                file_dialog_type: crate::input::FileDialogType::SaveFile,
            }, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        
        let Value::Path(path) = &inputs[1].value else { return Err(OperationError { message: "Unable to convert to path.".to_string() })};
        let Value::DynamicImage{data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![],
        })
    }
}
