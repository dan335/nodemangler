//! Mask-driven effect operations: drop shadow, outer glow, inner glow.
//!
//! Each op takes a grayscale-ish mask input and emits an RGBA image of the
//! effect alone (shadow / glow layer). Composite is the caller's job — pair
//! with `blit` or `blend` to lay the effect over the source.

pub mod drop_shadow;
pub mod inner_glow;
pub mod outer_glow;
