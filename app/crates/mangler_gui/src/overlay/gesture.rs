//! Gesture bookkeeping shared by every overlay editor.
//!
//! All the interactive editors drawn over an image or inside a settings box
//! (the 2D preview's curve overlay, the settings panel's tone-curve box, the
//! spatial gizmos) follow one protocol: mutate a local working copy every frame
//! for instant feedback, and push to the engine only when a gesture *completes*.
//! [`Gesture`] is the two-bit result of one frame of that protocol.

/// One frame of an overlay edit.
///
/// `changed` means the working value was mutated this frame; `commit` means the
/// gesture completed and the value should reach the engine.
///
/// ## The asymmetry, stated once
/// On a drag's *release* frame the pointer has not moved, so egui reports
/// `dragged() == false` and `drag_stopped() == true`. That frame therefore
/// yields `{ changed: false, commit: true }`. Callers must push their
/// **accumulated local value**, never the frame's `changed` payload — reading
/// the payload on the release frame would push nothing at all.
///
/// `commit` is deliberately never set mid-drag, so heavy downstream nodes
/// re-run once per gesture rather than once per frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Gesture {
    /// The working value was mutated this frame.
    pub changed: bool,
    /// The gesture completed; push the accumulated value to the engine.
    pub commit: bool,
}

impl Gesture {
    /// Nothing happened this frame.
    pub const IDLE: Gesture = Gesture { changed: false, commit: false };

    /// A mid-drag mutation: the value moved but the gesture is still running.
    pub const fn dragging() -> Gesture {
        Gesture { changed: true, commit: false }
    }

    /// A completed edit that both mutated the value and ended in the same frame
    /// — an insert, a delete, or a discrete control toggle.
    pub const fn edited() -> Gesture {
        Gesture { changed: true, commit: true }
    }

    /// A gesture that ended without moving anything this frame (the drag-release
    /// case described on the type).
    pub const fn released() -> Gesture {
        Gesture { changed: false, commit: true }
    }

    /// Fold another frame's result in. Both fields are sticky, so the order of
    /// merges never matters — an editor can accumulate across many handles.
    pub fn merge(&mut self, other: Gesture) {
        self.changed |= other.changed;
        self.commit |= other.commit;
    }
}

#[cfg(test)]
#[path = "gesture_tests.rs"]
mod tests;
