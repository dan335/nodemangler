//! Physical-process simulation generators.
//!
//! Nodes in this category generate content by simulating the real-world
//! process that creates a material's look (crack propagation, hydraulic
//! erosion, diffusion-limited aggregation, percolation, ...) rather than by
//! layering random noise. The caustics noise node's refraction simulation is
//! the reference for the approach.
//!
//! Category convention: guidance-map image inputs (weakness, fuel, moisture,
//! height, ...) are OPTIONAL — when unconnected, the node generates an
//! internal fallback map from its seed, so every simulation node also works
//! standalone like a noise generator. Connecting a map makes the simulation
//! context-aware (e.g. cracks concentrate where the supplied weakness map is
//! dark).
//!
//! No nodes yet; planned nodes are listed in the repository's `plan.md`.
