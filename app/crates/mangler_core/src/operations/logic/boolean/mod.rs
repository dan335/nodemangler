//! Boolean algebra gate operations.
//!
//! Each operation converts its inputs to booleans (non-zero numeric values are
//! truthy, zero is falsy) and applies the corresponding logical gate. All gates
//! produce a single boolean output.

/// Logical AND: true only when both inputs are true.
pub mod and;
/// Logical OR: true when at least one input is true.
pub mod or;
/// Logical NOT: inverts a single boolean input.
pub mod not;
/// Logical XOR: true when exactly one input is true.
pub mod xor;
/// Logical NAND: true unless both inputs are true (negated AND).
pub mod nand;
/// Logical NOR: true only when both inputs are false (negated OR).
pub mod nor;
