//! Shared building blocks for the app's interactive overlay editors.
//!
//! Three editors sit on top of this: the 2D preview's spatial curve overlay
//! (`view_window::curve_overlay`), the settings panel's tone-curve box
//! (`settings::tone_curve_widget`), and the 2D preview's spatial gizmos
//! (`view_window::spatial_overlay`). They all follow one protocol:
//!
//! 1. Clone the value into a working copy.
//! 2. Register a click catcher, then handles, and let egui resolve the drags.
//! 3. Return a [`Gesture`] saying whether the value moved and whether the
//!    gesture *finished*.
//! 4. The caller mirrors the mutated value into its local `GraphNode` every
//!    frame for instant feedback, and sends `ChangeNodeMessage::SetInput` only
//!    on `commit` — so a heavy downstream graph re-runs once per gesture.
//!
//! The pieces that used to be restated (and drift) per editor now live here:
//! the egui hit-test contract that lets handle drags and canvas panning coexist
//! ([`handle`]), the normalized ↔ screen mapping ([`mapping`]), the
//! release-frame commit asymmetry ([`gesture`]), the floating control strip
//! ([`strip`]), and the whole control-point interaction ([`point_editor`]).

pub mod gesture;
pub mod handle;
pub mod mapping;
pub mod point_editor;
pub mod strip;

pub use gesture::Gesture;
