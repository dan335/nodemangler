//! Pattern generation operations.
//!
//! This module provides operations that generate repeating tiled patterns as
//! grayscale images. Available patterns include brick, hexagonal, weave, and
//! a tile sampler that scatters an input pattern across a grid with randomization.

pub mod brick;
pub mod hexagonal;
pub mod weave;
pub mod tile_sampler;
