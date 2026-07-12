//! Curve-space simulation operations: nodes that evolve a curve by simulating
//! the physical process that shapes it, emitting the evolved `Value::Curve`
//! (plus raster byproducts). Follows the images/simulation category
//! conventions — seed-first input order, optional guidance maps, iteration
//! counts as the main driver — but lives under curves because the primary
//! output is a curve.

/// Curvature-driven river meandering (Howard & Knutson): bends grow, migrate
/// downstream, and cut off into oxbow lakes.
pub mod meander;
