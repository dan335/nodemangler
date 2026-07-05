//! Image → text operations: describe or encode an image as a `Text` value.
//!
//! These nodes take a `Value::Image` and emit `Text`, so they live under the
//! `text` category (categorized by output type) even though their input is an
//! image — the text counterpart to `numbers/image/`. Luminance/channel
//! reductions reuse the shared `pixel_luma`/`pixel_rgba` helpers from
//! `numbers/image/`.

/// Renders an image to multi-line ASCII art text.
pub mod ascii_art;
/// Encodes an image as a `data:image/...;base64,...` URI string.
pub mod data_uri;
/// Formats an image's dimensions/channels/aspect into a summary string.
pub mod image_info;
/// Lists an image's most common colors as newline-separated hex codes.
pub mod palette_hex;
/// Computes a perceptual average-hash of an image as a hex string.
pub mod image_hash;
