use image::{ImageBuffer, Rgba};


#[derive(Debug, Clone, PartialEq)]
pub enum Thumbnail {
    Image(ImageBuffer<Rgba<u8>, Vec<u8>>),
    Text(String),
}