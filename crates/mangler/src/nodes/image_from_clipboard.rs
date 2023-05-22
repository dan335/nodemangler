extern crate clipboard;

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Image from URL".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "Check".to_string(),
            default_value: Value::bool(true),
            valid_types: vec![ValueType::Bool],
            ui_type: Some(UiType::UiButton),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "image".to_string(),
        default_value: Value::ImageRgba8(RgbaImage::new(32, 32)),
        valid_types: vec![ValueType::ImageRgba8],
        ui_type: None,
    },];
}

pub async fn image_from_clipboard(node_id: &String, inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
    let start_time = Instant::now();

}