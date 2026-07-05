//! Random number generation nodes.
//!
//! Provides trigger-based random number generators for both decimal and integer types.

/// Generates a random decimal in `[0, 1)` on each trigger.
pub mod random_decimal;
/// Generates a random integer in a configurable `[min, max)` range on each trigger.
pub mod random_integer;
/// Generates a normally-distributed random decimal (Box–Muller) on each trigger.
pub mod random_gaussian;