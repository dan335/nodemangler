use image::ImageFormat;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
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
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let file_name_converted = convert_input(inputs, 1, ValueType::String, &mut input_errors);
        let folder_converted = convert_input(inputs, 2, ValueType::Path, &mut input_errors);
        let image_type_converted = convert_input(inputs, 3, ValueType::ImageType, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::String(file_name) = file_name_converted.unwrap() else { unreachable!() };
        let Value::Path(mut folder_path) = folder_converted.unwrap() else { unreachable!() };
        let Value::ImageType(image_type) = image_type_converted.unwrap() else { unreachable!() };

        // run node
        if folder_path.exists() {
            folder_path.push(file_name);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_output_settings() {
        let s = OpImageOutputFile::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageOutputFile::create_inputs().is_empty());
        assert!(!OpImageOutputFile::create_outputs().is_empty());
    }

    #[tokio::test]
    async fn test_file_output_exact_settings() {
        let s = OpImageOutputFile::settings();
        assert_eq!(s.name, "image to file");
        assert_eq!(OpImageOutputFile::create_inputs().len(), 4);
        assert_eq!(OpImageOutputFile::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_file_output_nonexistent_folder_returns_error() {
        use image::DynamicImage;
        use std::sync::Arc;
        use crate::get_id;

        let imgbuf = image::RgbaImage::new(4, 4);
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("file name".to_string(), Value::String("test_output".to_string()), None, None),
            Input::new("folder".to_string(), Value::Path(std::path::PathBuf::from("/this/path/does/not/exist/at/all")), None, None),
            Input::new("image format".to_string(), Value::ImageType(image::ImageFormat::Png), None, None),
        ];
        let result = OpImageOutputFile::run(&mut inputs).await;
        assert!(result.is_err(), "saving to nonexistent folder should fail");
    }
}
