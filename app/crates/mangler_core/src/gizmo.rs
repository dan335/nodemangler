//! Declarative spatial-gizmo table: which of an operation's inputs are
//! positions, and what their numbers mean.
//!
//! Some operations take parameters that are inherently spatial but are edited
//! as bare numbers — `crop`'s four fractions, `sample pixel`'s x/y. The GUI can
//! draw those directly on the image and let the user drag them, but only if it
//! knows *which* inputs pair up and *what convention* they use. This module is
//! that knowledge, as data.
//!
//! It lives in `mangler_core`, next to the operations, because the conventions
//! are defined by each op's `run()` — `crop.rs` is what makes `width` a size
//! rather than a far edge, and `sample_pixel.rs` is what makes its fraction
//! resolve against pixel *centres* rather than the pixel extent. Keeping the
//! table in the same crate lets `gizmo_tests.rs` cross-check every declared
//! index against `create_inputs()`, so inserting an input into an operation
//! fails a test instead of silently repointing a gizmo at the wrong slider.
//!
//! [`gizmos`] is one central `match` with a `_ => &[]` catch-all — the same
//! shape as `graph::output_node_path_inputs`. The overwhelming majority of
//! operations have no gizmo and cost nothing.
//!
//! ## What is deliberately *not* here
//! Value types and ranges. The GUI reads those from the [`Input`] itself:
//! whether to write back a `Decimal` or an `Integer` comes from the input's
//! current value, and clamping comes from its `InputSettings::Slider` /
//! `DragValue` range. Duplicating either here would be a second source of truth
//! that drifts — this way, widening a slider automatically widens its gizmo.
//!
//! [`Input`]: crate::input::Input

use crate::operations::Operation;

/// One draggable overlay an operation offers.
#[derive(Debug, Clone, Copy)]
pub struct GizmoSpec {
    /// Short name shown in the overlay's readout, e.g. `"crop"`.
    pub label: &'static str,
    pub kind: Gizmo,
}

/// The shape of a gizmo and the inputs it drives.
#[derive(Debug, Clone, Copy)]
pub enum Gizmo {
    /// A single draggable point, drawn as a crosshair.
    ///
    /// `diameter` is an optional **display-only** input index: source-pixel
    /// sample size (as on `sample pixel`). It is not drag-driven and does not
    /// participate in the editable/read-only rule — connecting diameter must
    /// not freeze the crosshair. The overlay paints a circle of that diameter
    /// so changing the slider updates the 2D view live.
    Point {
        x: usize,
        y: usize,
        diameter: Option<usize>,
        space: SpatialSpace,
    },
    /// A draggable, resizable box.
    Rect { x: usize, y: usize, w: usize, h: usize, space: SpatialSpace, extent: RectExtent },
}

/// How a gizmo's numbers relate to positions on the image.
///
/// More variants arrive as more operations are wired up (signed offsets from
/// the centre, pixels at the 1024 reference, absolute destination pixels); they
/// are **not** interchangeable, which is exactly why the convention is declared
/// rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialSpace {
    /// A 0-1 fraction of the image, top-left origin, y-down.
    Norm01 { basis: PixelBasis },
}

/// How an operation resolves a 0-1 fraction against pixels.
///
/// This is a real half-pixel discrepancy between existing ops, not pedantry:
/// at `x = 1.0` an [`Extent`](PixelBasis::Extent) gizmo sits on the image's
/// right edge while a [`Centres`](PixelBasis::Centres) one sits on the last
/// pixel's centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelBasis {
    /// `v * w` — the fraction spans the pixel *extent*. Used by `crop`.
    Extent,
    /// `v * (w - 1)` — the fraction spans pixel *centres*, so `1.0` addresses
    /// the last pixel rather than one past it. Used by `sample pixel`.
    Centres,
}

/// Whether a [`Gizmo::Rect`]'s `w`/`h` inputs are a size or the far edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectExtent {
    /// `w`/`h` are a *size* measured from `x`/`y`. Used by `crop`.
    OriginSize,
    /// `w`/`h` are the far edge's absolute coordinates.
    TwoCorner,
}

/// `crop`: an origin/size box in plain image fractions.
const CROP: &[GizmoSpec] = &[GizmoSpec {
    label: "crop",
    kind: Gizmo::Rect {
        x: 1,
        y: 2,
        w: 3,
        h: 4,
        space: SpatialSpace::Norm01 { basis: PixelBasis::Extent },
        extent: RectExtent::OriginSize,
    },
}];

/// `sample pixel`: the sampled point, addressed by pixel centres.
/// `diameter` (input 3) is display-only — see [`Gizmo::Point`].
const SAMPLE_PIXEL: &[GizmoSpec] = &[GizmoSpec {
    label: "sample",
    kind: Gizmo::Point {
        x: 1,
        y: 2,
        diameter: Some(3),
        space: SpatialSpace::Norm01 { basis: PixelBasis::Centres },
    },
}];

/// The spatial gizmos this operation exposes on the 2D preview, in draw order
/// (later entries hit-test on top). Empty for the vast majority of operations.
///
/// Keep in sync with each op's `create_inputs()`. `gizmo_tests.rs` asserts that
/// every referenced index exists, is numeric, and carries the expected input
/// name, so a reordering or insertion is caught by the test suite.
pub fn gizmos(op: &Operation) -> &'static [GizmoSpec] {
    match op {
        Operation::OpImageTransformCrop => CROP,
        Operation::OpColorSampleSamplePixel => SAMPLE_PIXEL,
        _ => &[],
    }
}

impl Gizmo {
    /// The input indices this gizmo **drives** (drag / commit), in a fixed order.
    ///
    /// Used to decide whether the gizmo is editable (every input must be
    /// unconnected) and to report which inputs a completed gesture touched.
    /// Display-only indices such as [`Gizmo::Point`]'s `diameter` are **not**
    /// included — see [`Self::referenced_inputs`].
    pub fn inputs(&self) -> Vec<usize> {
        match *self {
            Gizmo::Point { x, y, .. } => vec![x, y],
            Gizmo::Rect { x, y, w, h, .. } => vec![x, y, w, h],
        }
    }

    /// Every input index this gizmo reads, including display-only ones.
    ///
    /// Use for bounds-checking the declaration against `create_inputs()`. Prefer
    /// [`Self::inputs`] when deciding editability or commit targets.
    pub fn referenced_inputs(&self) -> Vec<usize> {
        match *self {
            Gizmo::Point { x, y, diameter, .. } => {
                let mut v = vec![x, y];
                if let Some(d) = diameter {
                    v.push(d);
                }
                v
            }
            Gizmo::Rect { x, y, w, h, .. } => vec![x, y, w, h],
        }
    }

    /// The coordinate convention this gizmo's numbers use.
    pub fn space(&self) -> SpatialSpace {
        match *self {
            Gizmo::Point { space, .. } | Gizmo::Rect { space, .. } => space,
        }
    }
}

impl SpatialSpace {
    /// Convert a value pair in this space to a normalized `[0,1]²` image
    /// position. `dims` is the backdrop's pixel size, when known.
    pub fn to_unit(self, dims: Option<(u32, u32)>, v: [f32; 2]) -> [f32; 2] {
        match self {
            SpatialSpace::Norm01 { basis } => match (basis, dims) {
                (PixelBasis::Centres, Some((w, h))) => {
                    [centres_to_unit(v[0], w), centres_to_unit(v[1], h)]
                }
                // Without pixel dimensions the half-pixel correction is
                // unknowable, so degrade to the plain fraction rather than
                // guessing — the two differ by well under a screen pixel on any
                // real image.
                _ => v,
            },
        }
    }

    /// Exact inverse of [`to_unit`](Self::to_unit).
    pub fn from_unit(self, dims: Option<(u32, u32)>, u: [f32; 2]) -> [f32; 2] {
        match self {
            SpatialSpace::Norm01 { basis } => match (basis, dims) {
                (PixelBasis::Centres, Some((w, h))) => {
                    [unit_to_centres(u[0], w), unit_to_centres(u[1], h)]
                }
                _ => u,
            },
        }
    }
}

/// Map a pixel-centres fraction to a unit position: the value addresses pixel
/// `v * (n - 1)`, whose centre sits at `(index + 0.5) / n` across the image.
fn centres_to_unit(v: f32, n: u32) -> f32 {
    if n == 0 {
        return v;
    }
    (v * (n.saturating_sub(1)) as f32 + 0.5) / n as f32
}

/// Inverse of [`centres_to_unit`].
fn unit_to_centres(u: f32, n: u32) -> f32 {
    if n <= 1 {
        return u;
    }
    (u * n as f32 - 0.5) / (n - 1) as f32
}

#[cfg(test)]
#[path = "gizmo_tests.rs"]
mod tests;
