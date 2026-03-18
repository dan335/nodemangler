//! Control flow operations.
//!
//! Provides nodes that direct data flow based on conditions, enabling
//! conditional logic within the node graph.

/// Multiplexer node that selects between two values based on a boolean condition.
pub mod select;
