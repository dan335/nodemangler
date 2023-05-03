use crate::value::{Value, ValueType};

#[derive(Debug)]
pub struct Input {
    pub name: String,
    pub value: Value,
    pub connection: Option<String>,
    pub valid_types: Vec<ValueType>,
}

impl Input {
    pub fn new(name: String, value: Value) -> Input {
        Input { name, value, connection: None, valid_types: Vec::new() }
    }
}


pub struct InputSettings {
    pub name: String,
    pub default_value: Value,
    pub valid_types: Vec<ValueType>,
}


// #[macro_export] macro_rules! create_inputs {
//     ($input_settings:expr) => {
//         $input_settings.iter().map(|settings: &InputSettings| Input {
//             name: settings.name.to_owned(),
//             value: settings.default_value.clone(),
//             connection: None,
//             valid_types: settings.valid_types.to_vec(),
//         }).collect::<Vec<Input>>()
//     };
// }

#[macro_export]
macro_rules! create_inputs {
    ($input_settings:expr) => {
        $input_settings.iter().map(|settings: &InputSettings| Input {
            name: settings.name.to_owned(),
            value: settings.default_value.clone(),
            connection: None,
            valid_types: settings.valid_types.to_vec(),
        }).collect::<Vec<Input>>()
    };
}