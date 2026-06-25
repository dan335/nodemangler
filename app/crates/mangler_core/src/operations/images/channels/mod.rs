//! RGBA channel manipulation operations.
//!
//! Provides nodes for decomposing an image into individual channels,
//! recombining channels into a single image, and remapping (shuffling)
//! which source channel feeds each output channel.

/// Splits an RGBA image into four separate grayscale channel images.
pub mod split;
/// Merges four grayscale channel images into a single RGBA image.
pub mod merge;
/// Remaps image channels by selecting which source channel (R/G/B/A) feeds each output channel.
pub mod shuffle;
/// Extracts a single source channel as a 1-channel grayscale image.
pub mod select;
/// Per-output-channel linear combinations of the input R/G/B channels plus bias.
pub mod mixer;
