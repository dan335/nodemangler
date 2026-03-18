//! PBR (Physically Based Rendering) texture generation operations.
//!
//! This module provides operations for deriving PBR material maps from
//! height maps and normal maps. Includes normal map generation, ambient
//! occlusion, curvature detection, and height-based material blending.

pub mod ao_from_height;
pub mod curvature;
pub mod height_blend;
pub mod normal_from_height;
