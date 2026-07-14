//! Per-run graph context made available to side-effecting output operations.
//!
//! Operations run through the uniform `Operation::run(&self, inputs)` dispatch
//! (see the `operations!` macro), so they receive nothing but their own inputs.
//! A few output operations, however, need to know *about the graph they belong
//! to* rather than just their inputs:
//!
//! - `to file` resolves its `folder` input relative to where the graph is saved
//!   and defaults its `file name` to the graph's name.
//! - `to file`, `material`, and `to clipboard` honour a "force save" flag so a
//!   headless `graph.run()` (the CLI's `mangle run`) writes every output even
//!   though the interactive auto-save toggle defaults to off.
//! - During a batch run (one run per source image in a folder), `to file` and
//!   `material` derive per-item file names from `batch_item_stem` so
//!   successive iterations don't overwrite each other's output, while
//!   `to clipboard` uses its presence to opt out of the force-save write
//!   (nobody wants the clipboard rewritten hundreds of times in a row).
//!
//! Threading a context parameter through the macro-generated dispatch would
//! mean touching every operation's `run` signature. Instead the engine stashes
//! a [`RunContext`] in a thread-local for the duration of a single operation's
//! execution, and the handful of ops that care read it via [`current`]. The
//! context is set on the same (blocking-pool) thread the operation runs on and
//! cleared as soon as it returns — see [`Node::run`](crate::node::Node::run).
//!
//! Direct unit-test calls to an op's `run` (which bypass the engine) simply see
//! `None`; those ops fall back to sensible standalone behaviour.

use std::cell::RefCell;
use std::path::PathBuf;

/// Read-only information about the graph a currently-running operation belongs
/// to. Cheap to clone (a `PathBuf`, a `String`, and a `bool`).
#[derive(Clone, Debug, Default)]
pub struct RunContext {
    /// Directory the graph is saved in, if it has ever been saved. Output ops
    /// resolve relative path inputs against this.
    pub graph_dir: Option<PathBuf>,
    /// The graph's display name, used as the default output file name.
    pub graph_name: String,
    /// When true, side-effecting output ops write regardless of their own
    /// auto-save toggle. Set by headless CLI runs so `mangle run` still emits
    /// files even though auto-save is off by default in the GUI.
    pub force_save: bool,
    /// File stem of the batch item currently driving this run, when a batch
    /// run is iterating a from-folder node. Output ops use it to derive
    /// per-item file names, and `to clipboard` uses it to skip forced writes
    /// during a batch.
    pub batch_item_stem: Option<String>,
}

thread_local! {
    /// The context for the operation currently executing on this thread, if any.
    static CONTEXT: RefCell<Option<RunContext>> = const { RefCell::new(None) };
}

/// RAII guard returned by [`set`]. Restores the previous (typically `None`)
/// context for this thread when dropped, so a context never leaks past the
/// operation it was set for — even if that operation panics.
pub struct RunContextGuard {
    previous: Option<RunContext>,
}

impl Drop for RunContextGuard {
    fn drop(&mut self) {
        CONTEXT.with(|c| *c.borrow_mut() = self.previous.take());
    }
}

/// Install `ctx` as the current run context for this thread, returning a guard
/// that clears it on drop. Call this on the thread that will run the operation,
/// immediately before invoking it.
pub fn set(ctx: RunContext) -> RunContextGuard {
    let previous = CONTEXT.with(|c| c.borrow_mut().replace(ctx));
    RunContextGuard { previous }
}

/// The run context for the operation executing on this thread, or `None` when
/// called outside the engine (e.g. a direct unit-test call to an op's `run`).
pub fn current() -> Option<RunContext> {
    CONTEXT.with(|c| c.borrow().clone())
}
