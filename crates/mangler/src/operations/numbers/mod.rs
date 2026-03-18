//! Number operations for the node graph.
//!
//! Provides numeric input nodes, arithmetic, algebra, casting, logarithmic,
//! and random number generation operations.

/// Numeric input nodes (integer and decimal constants).
pub mod inputs;
/// Basic arithmetic operations (add, subtract, multiply, divide, etc.).
pub mod arithmetic;
/// Type casting between integer and decimal.
pub mod cast;
/// Random number generation nodes.
pub mod random;
/// Algebraic functions (abs, sqrt, pow, factorial, gcd, lcm, etc.).
pub mod algebra;
/// Logarithmic and exponential functions (log, ln, exp).
pub mod logarithmic;
