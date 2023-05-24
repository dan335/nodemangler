use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType};
use crate::value::{Value, ValueType};
use std::time::Instant;
use arboard::Clipboard;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Text from Clipboard".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "Copy from Clipboard".to_string(),
            default_value: Value::UiButton(true),
            valid_types: vec![ValueType::Bool],
            ui_type: Some(UiType::UiButton),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "text".to_string(),
        default_value: Value::String("".to_string()),
        valid_types: vec![ValueType::DynamicImage],
        ui_type: None,
    },];
}

pub async fn text_from_clipboard(_inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
    let start_time = Instant::now();

    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            let node_output_message = OperationResponse {
                index: 0,
                value: Value::String(text),
                time: Instant::now().duration_since(start_time),
            };
        
            return Ok(vec![node_output_message]);
        }
    }
    
    Err(OperationError { message: "Error grabbing text from clipboard.".to_string() })
}