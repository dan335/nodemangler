use serde::{Serialize, Deserialize};

pub mod color_space;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32, 
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color {
            r,
            g,
            b,
            a,
        }
    }

    pub fn from_srgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::new(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    pub fn to_srgba_u8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    pub fn to_srgb_u8(&self) -> (u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::new(0.0, 0.0, 0.0, 1.0)
    }
}