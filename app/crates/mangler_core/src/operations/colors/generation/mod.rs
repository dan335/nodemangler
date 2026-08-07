//! Color generation operations (from hex, to hex, random color, skin tone).

/// Parse a hex string into a color.
pub mod from_hex;
/// Convert a color to a hex string.
pub mod to_hex;
/// Generate a random color with constrained saturation and lightness.
pub mod random_color;
/// Generate a plausible human skin tone via the Inclusive Color Space.
pub mod skin_tone;
