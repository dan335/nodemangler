//! Color relationship operations (complementary, triadic, analogous, tetradic, double split-complementary, monochromatic).

/// Complementary and split-complementary color relationships.
pub mod complementary;
/// Triadic color relationships.
pub mod triadic;
/// Analogous color relationships.
pub mod analogous;
/// Tetradic color relationships.
pub mod tetradic;
/// Double split-complementary color relationships.
pub mod double_split_complementary;
/// Monochromatic color relationships.
pub mod monochromatic;

/// Rotates a hue value (0–360) by a given number of degrees, wrapping correctly.
pub(crate) fn rotate_hue(h: f32, degrees: f32) -> f32 {
    ((h + degrees) % 360.0 + 360.0) % 360.0
}
