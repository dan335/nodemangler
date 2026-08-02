//! Effect operations: drop shadow, outer glow, inner glow, bloom.
//!
//! Drop shadow / outer glow / inner glow are mask-driven — each takes a
//! grayscale-ish mask input and emits an RGBA image of the effect alone
//! (shadow / glow layer); composite is the caller's job, pairing with `blit`
//! or `blend` to lay the effect over the source. Bloom is different: it takes
//! a full image, keys on its own luminance rather than a mask shape, and
//! outputs the source already composited with the bloom halo.

pub mod bloom;
pub mod drop_shadow;
pub mod inner_glow;
pub mod outer_glow;
