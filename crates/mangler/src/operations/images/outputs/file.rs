use image::{RgbaImage, ImageFormat};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
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
            Input::new("file name".to_string(), Value::String("image01".to_string()), InputSettings::None, None),
            Input::new("folder".to_string(), Value::Path(PathBuf::new()), InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: None,
                file_dialog_type: crate::input::FileDialogType::PickFolder,
            }, None),
            Input::new("image format".to_string(), Value::ImageType(ImageFormat::Jpeg), InputSettings::None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("file path".to_string(), Value::Path(PathBuf::new()), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        
        let Value::DynamicImage{data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };
        let Value::String(file_name) = &inputs[1].value else { return Err(OperationError { message: "Unable to convert to path.".to_string() })};
        let Value::Path(mut folder_path) = inputs[2].value.clone() else { return Err(OperationError { message: "Unable to convert to path.".to_string() })};
        let Value::ImageType(image_type) = inputs[3].value else { return Err(OperationError { message: "Unable to convert to path.".to_string() })};

        if folder_path.exists() {
            if !folder_path.ends_with("/") {
                // not sure why this is needed
                // if path does not end in / then last dir is removed
                // when setting filename
                folder_path.push("asdf");
            }
            
            folder_path.set_file_name(file_name);
            folder_path.set_extension(image_type.extensions_str()[0]);

            if let Ok(_) = data.save_with_format(folder_path.clone(), image_type) {
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![OutputResponse {
                        value: Value::Path(folder_path),
                    }],
                })
            } else {
                Err(OperationError { message: "Unable to convert to path.".to_string() })
            }
        } else {
            Err(OperationError { message: "Unable to convert to path.".to_string() })
        }

        
    }
}
