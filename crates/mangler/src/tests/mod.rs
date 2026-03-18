//! Integration and performance tests for the mangler crate.
//!
//! Unit tests live in each source file's `#[cfg(test)] mod tests` block.
//! This directory holds cross-cutting tests that exercise multiple subsystems
//! together (serialization round-trips, performance benchmarks, etc.).

mod serialization_tests;
mod perf_tests;
