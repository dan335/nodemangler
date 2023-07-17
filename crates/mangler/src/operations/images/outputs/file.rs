use image::{RgbaImage, ImageFormat};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputFile {}

impl OpImageOutputFile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image to file".to_string(),
            description: "Saves an image to a file.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("file name".to_string(), Value::String("image01".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("folder".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: None,
                file_dialog_type: crate::input::FileDialogType::PickFolder,
            }), None),
            Input::new("image format".to_string(), Value::ImageType(ImageFormat::Jpeg), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("file path".to_string(), Value::Path(PathBuf::new()), None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = inputs[0].value.try_convert_to(ValueType::DynamicImage);
        let file_name_converted = inputs[1].value.try_convert_to(ValueType::String);
        let folder_converted = inputs[2].value.try_convert_to(ValueType::Path);
        let image_type_converted = inputs[3].value.try_convert_to(ValueType::ImageType);

        // gather errors
        if image_converted.is_err() { input_errors.push((0, image_converted.as_ref().err().unwrap().message.clone())); }
        if file_name_converted.is_err() { input_errors.push((1, file_name_converted.as_ref().err().unwrap().message.clone())); }
        if folder_converted.is_err() { input_errors.push((2, folder_converted.as_ref().err().unwrap().message.clone())); }
        if image_type_converted.is_err() { input_errors.push((3, image_type_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::DynamicImage{data, change_id:_}) = image_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::String(file_name)) = file_name_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Path(mut folder_path)) = folder_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::ImageType(image_type)) = image_type_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
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
                Err(OperationError { input_errors: vec![], node_error: Some("Unable to convert to path.".to_string()) })
            }
        } else {
            Err(OperationError { input_errors: vec![], node_error: Some("Folder does not exist.".to_string()) })
        }

        
    }
}
