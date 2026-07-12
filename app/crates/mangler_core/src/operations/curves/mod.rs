//! Curve operations: user-drawn 2D paths and closed shapes as first-class
//! values (see [`crate::curve::Curve`]). Categorized by output type — every op
//! here emits a `Value::Curve`.

pub mod inputs;
pub mod simulation;
