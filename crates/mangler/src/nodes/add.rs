use std::time::Instant;

use crate::graph::Graph;
use crate::input::InputSettings;
use crate::node_attributes::NodeAttributes;
use crate::output::OutputSettings;
use crate::node::Node;
use crate::value::{Value, ValueType};
use crate::get_id;

lazy_static! {
    pub static ref INPUT_SETTINGS: Vec<InputSettings> = vec![
        InputSettings {
            name: "a".to_string(),
            default_value: Value::Decimal { value: 0.0 },
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
        },
        InputSettings {
            name: "b".to_string(),
            default_value: Value::Decimal { value: 0.0 },
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
        },
    ];

    pub static ref OUTPUT_SETTINGS: Vec<OutputSettings> = vec![
        OutputSettings {
            name: "result".to_string(),
            default_value: Value::Decimal { value: 0.0 },
        },
    ];
}


#[derive(Debug)]
pub struct Add {
    pub attr: NodeAttributes,
}


impl Add {
    pub fn new(graph: &mut Graph) -> String {
        let id = get_id();
        let attr = NodeAttributes::new(id.clone(), &INPUT_SETTINGS, &OUTPUT_SETTINGS);
        graph.add_node(id.clone(), Box::new(Add { attr }));
        id
    }
}


impl Node for Add {
    fn run(&mut self) {
        let start_time = Instant::now();

        self.attr.outputs[0].value = match (&self.attr.inputs[0].value, &self.attr.inputs[1].value) {
            (
                Value::Integer { value: a },
                Value::Decimal { value: b }
            ) => {
                Value::Decimal { value: *a as f32 + *b }
            },

            (
                Value::Integer { value: a },
                Value::Integer { value: b }
            ) => {
                Value::Integer { value: *a + *b }
            },

            (
                Value::Integer { value: a },
                Value::String { value: b }
            ) => {
                Value::String { value: format!("{} {}", a.to_string(), *b) }
            },

            (
                Value::Decimal { value: a },
                Value::Decimal { value: b }
            ) => {
                Value::Decimal { value: *a + *b }
            },

            (
                Value::Decimal { value: a },
                Value::Integer { value: b }
            ) => {
                Value::Decimal { value: *a + *b as f32 }
            },

            (
                Value::Decimal { value: a },
                Value::String { value: b }
            ) => {
                Value::String { value: format!("{} {}", a.to_string(), *b) }
            },

            (
                Value::String { value: a },
                Value::Integer { value: b }
            ) => {
                Value::String { value: format!("{} {}", *a, b.to_string()) }
            },

            (
                Value::String { value: a },
                Value::Decimal { value: b }
            ) => {
                Value::String { value: format!("{} {}", *a, b.to_string()) }
            },

            (
                Value::String { value: a },
                Value::String { value: b }
            ) => {
                Value::String { value: format!("{} {}", *a, *b) }
            },
        };


        self.attr.time = Some(Instant::now().duration_since(start_time));
    }

    fn set_intput_value(&mut self, index: usize, value: Value) {
        self.attr.set_intput_value(index, value);
    }

    fn print_output(&self) -> String {
        self.attr.print_output()
    }
}