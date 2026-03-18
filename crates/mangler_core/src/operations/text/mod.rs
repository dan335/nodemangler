//! Text operations for the node graph.
//!
//! Provides operations for producing and manipulating `Text` values,
//! plus a clipboard source node (currently WIP).

/// Text input (source) nodes.
pub mod inputs;
/// Text manipulation nodes (append, length, case, cast).
pub mod manipulation;
/// Reads text content from the system clipboard (WIP/disabled).
pub mod text_from_clipboard;
