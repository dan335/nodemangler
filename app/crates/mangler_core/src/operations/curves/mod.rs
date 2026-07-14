//! Curve operations: user-drawn 2D paths and closed shapes as first-class
//! values (see [`crate::curve::Curve`]). Categorized by output type — every op
//! here emits a `Value::Curve`.

pub mod combine;
pub(crate) mod common;
pub mod generators;
pub mod inputs;
pub mod modify;
pub mod simulation;
