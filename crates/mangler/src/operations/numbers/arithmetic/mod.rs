//! Basic arithmetic operations for the node graph.
//!
//! Includes standard math operations on integers and decimals, with automatic
//! type promotion (e.g., integer + decimal yields decimal).

/// Addition of two values (numbers, strings, colors, images).
pub mod add;
/// Subtraction of two numbers.
pub mod subtract;
/// Multiplication of two numbers.
pub mod multiply;
/// Division of two numbers with division-by-zero protection.
pub mod divide;
/// Decrements a number by 1.
pub mod decrement;
/// Increments a number by 1.
pub mod increment;
/// Returns the larger of two numbers.
pub mod max;
/// Returns the smaller of two numbers.
pub mod min;
/// Clamps a number between a minimum and maximum.
pub mod clamp;
/// Remainder (modulus) of dividing two numbers.
pub mod modulus;
/// Rounds a decimal to the nearest whole number.
pub mod round;
/// Returns the sign (-1, 0, or 1) of a number.
pub mod sign;
/// Negates a number (flips sign).
pub mod negate;
/// Computes 1/x (reciprocal).
pub mod reciprocal;
/// Computes the average (mean) of two numbers.
pub mod average;
/// Ceiling: rounds up to the nearest integer.
pub mod ceil;
/// Floor: rounds down to the nearest integer.
pub mod floor;
/// Truncation: removes the fractional part of a decimal (`trunc()`).
pub mod trunc;
/// Fractional part of a decimal (`fract()`).
pub mod frac;
