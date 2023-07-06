use serde::{Serialize, Deserialize};

pub mod blend;
pub mod color_spaces;

// stored as srgba floats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32, 
    pub a: f32,
}