use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType, OutputResponse};
use crate::value::{Value, ValueType};
use std::time::Instant;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Add".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "a".to_string(),
            default_value: Value::Decimal(0.0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
            ui_type: Some(UiType::DragValue),
        },
        ConnectionSettings {
            name: "b".to_string(),
            default_value: Value::Decimal(0.0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
            ui_type: Some(UiType::DragValue),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "result".to_string(),
        default_value: Value::Decimal(0.0),
        valid_types: vec![ValueType::Decimal],
        ui_type: None,
    },];
}

// NodeOutputChangedMessage is the message to send to main thread
// value is separate because it will not be sent
pub async fn add(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
    let start_time = Instant::now();

    let value = match (&inputs[0].get_value(), &inputs[1].get_value()) {
        (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 + *b),

        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a + *b),

        (Value::Integer(a), Value::String(b)) => Value::String(format!("{} {}", a, *b)),

        (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(*a + *b),

        (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(*a + *b as f32),

        (Value::Decimal(a), Value::String(b)) => Value::String(format!("{} {}", a, *b)),

        (Value::String(a), Value::Integer(b)) => Value::String(format!("{} {}", *a, b)),

        (Value::String(a), Value::Decimal(b)) => Value::String(format!("{} {}", *a, b)),

        (Value::String(a), Value::String(b)) => Value::String(format!("{} {}", *a, *b)),

        _ => {return Err(OperationError{message:"Unable to add formats.".to_string()});}
    };

    Ok(OperationResponse {
        time: Instant::now().duration_since(start_time),
        outputs: vec![OutputResponse {
            value,
        }],
    })
}
