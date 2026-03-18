//! Algebraic operations for the node graph.
//!
//! Provides mathematical functions including roots, powers, factorials,
//! number theory (GCD/LCM), and numeric decomposition (frac/trunc).

/// Absolute value of a number.
pub mod abs;
/// Square root of a number (errors on negative input).
pub mod sqrt;
/// Cube root of a number (handles negative inputs).
pub mod cbrt;
/// Nth root of a number (`a^(1/n)`).
pub mod nth_root;
/// Raises a base to an exponent (`base^exponent`).
pub mod pow;
/// Factorial of an integer (clamped to 0..12 to fit in i32).
pub mod factorial;
/// Greatest common divisor of two integers (Euclidean algorithm).
pub mod gcd;
/// Least common multiple of two integers.
pub mod lcm;
/// Fractional part of a decimal (`fract()`).
pub mod frac;
/// Truncation: removes the fractional part of a decimal (`trunc()`).
pub mod trunc;
