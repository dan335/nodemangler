//! Text-from-clipboard operation (currently disabled/commented out).
//!
//! This module was intended to read text content from the system clipboard and
//! expose it as a string output. The implementation is commented out pending
//! integration with the current operation framework.

// use crate::input::Input;
// use crate::node_settings::NodeSettings;
// use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType, OutputResponse};
// use crate::value::{Value, ValueType};
// use std::time::Instant;
// use arboard::Clipboard;

// lazy_static! {
//     pub static ref SETTINGS: NodeSettings = NodeSettings::new("Text from Clipboard".to_string());
//     pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
//         ConnectionSettings {
//             name: "Copy from Clipboard".to_string(),
//             default_value: Value::UiButton(true),
//             valid_types: vec![ValueType::Bool],
//             ui_type: Some(UiType::UiButton),
//         },
//     ];
//     pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
//         name: "text".to_string(),
//         default_value: Value::String("".to_string()),
//         valid_types: vec![ValueType::Image],
//         ui_type: None,
//     },];
// }

// pub async fn text_from_clipboard(_inputs: &[Input]) -> Result<OperationResponse, OperationError> {
//     let start_time = Instant::now();
//         let mut input_errors: Vec<(usize, String)> = vec![];
//
//         // convert inputs
//         // gather errors
//
//         // return if error
//         if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }
//
//         // get values
//         // run node

//     if let Ok(mut clipboard) = Clipboard::new() {
//         if let Ok(text) = clipboard.get_text() {
//             let node_output_message = OperationResponse {
//                 time: Instant::now().duration_since(start_time),
//                 outputs: vec![OutputResponse {
//                     value: Value::String(text),
//                 }]
//             };
        
//             return Ok(node_output_message);
//         }
//     }
    
//     Err(OperationError { message: "Error grabbing text from clipboard.".to_string() })
// }