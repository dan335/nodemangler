//! Logic operations for the node graph.
//!
//! This module organizes all logic-related operations into subcategories:
//! - `inputs` — boolean input nodes for feeding values into the graph
//! - `comparison` — relational operators (equal, not equal, less than, etc.)
//! - `boolean` — boolean algebra gates (and, or, not, xor, nand, nor)
//! - `flow` — control flow nodes (select/mux)

/// Boolean input nodes.
pub mod inputs;
/// Relational comparison operators that produce boolean outputs.
pub mod comparison;
/// Boolean algebra gate operations.
pub mod boolean;
/// Control flow operations (conditional selection).
pub mod flow;
