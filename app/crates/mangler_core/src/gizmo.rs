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
//! are defined by each op's `run()`. Keeping the table in the same crate lets
//! `gizmo_tests.rs` cross-check every declared index against `create_inputs()`.
//!
//! [`gizmos`] is one central `match` with a `_ => &[]` catch-all.
//!
//! ## What is deliberately *not* here
//! Value types and ranges. The GUI reads those from the [`Input`] itself.
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
    /// A draggable point (crosshair). Optional `diameter` is display-only
    /// (source-pixel disk). Optional `radius` is a **draggable** rim written
    /// in [`RadiusSpace`] units.
    Point {
        x: usize,
        y: usize,
        diameter: Option<usize>,
        radius: Option<(usize, RadiusSpace)>,
        space: SpatialSpace,
    },
    /// A draggable, resizable box.
    Rect {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        space: SpatialSpace,
        extent: RectExtent,
    },
    /// Graduated-filter line: `angle` in degrees, `position` 0–1 along the
    /// gradient axis (mid-transition).
    Line {
        angle: usize,
        position: usize,
        space: SpatialSpace,
    },
    /// Vertical and/or horizontal axis lines at 0–1 positions.
    Axes {
        /// Vertical line at this x (0–1).
        x: Option<usize>,
        /// Horizontal line at this y (0–1).
        y: Option<usize>,
        space: SpatialSpace,
    },
    /// Four corners as offsets from the image corners (perspective).
    /// Input order: TL x/y, TR x/y, BR x/y, BL x/y — each offset is a
    /// fraction of width/height added to the fixed corner base.
    QuadCorners {
        corners: [usize; 8],
    },
    /// Affine transform: fractional offsets, rotation in degrees about centre.
    Transform {
        offset_x: usize,
        offset_y: usize,
        rotation: usize,
        scale_x: Option<usize>,
        scale_y: Option<usize>,
    },
    /// Offset in reference pixels (px@1024), drawn as a vector from image centre.
    OffsetPx {
        x: usize,
        y: usize,
    },
    /// Radius ring fixed at image centre (vignette / swirl / spherize).
    CenterRadius {
        radius: usize,
        space: RadiusSpace,
    },
}

/// How a gizmo's numbers relate to positions on the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialSpace {
    /// A 0-1 fraction of the image, top-left origin, y-down.
    Norm01 { basis: PixelBasis },
    /// Offset from image centre: 0 = middle, ±1 = edges (`circle` centre_x/y).
    OffsetCenter,
}

/// How an operation resolves a 0-1 fraction against pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelBasis {
    /// `v * w` — pixel *extent*. Used by `crop`.
    Extent,
    /// `v * (w - 1)` — pixel *centres*. Used by `sample pixel`.
    Centres,
}

/// Units for a radius/rim handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadiusSpace {
    /// Fraction of half the shorter image dimension (circle, radial mask).
    HalfMinExtent,
    /// Fraction of half the image diagonal (swirl).
    HalfDiagonal,
    /// Fraction of half-extent where 1 reaches the corners (vignette-style unit square).
    CornerNorm,
}

/// Whether a [`Gizmo::Rect`]'s `w`/`h` inputs are a size or the far edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectExtent {
    /// `w`/`h` are a *size* measured from `x`/`y`.
    OriginSize,
    /// `w`/`h` are the far edge's absolute coordinates.
    TwoCorner,
}

// ── Declarations ──────────────────────────────────────────────────────────

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

const SAMPLE_PIXEL: &[GizmoSpec] = &[GizmoSpec {
    label: "sample",
    kind: Gizmo::Point {
        x: 1,
        y: 2,
        diameter: Some(3),
        radius: None,
        space: SpatialSpace::Norm01 { basis: PixelBasis::Centres },
    },
}];

/// radial gradient mask: centre 0–1 + radius as half-min-extent fraction.
const RADIAL_MASK: &[GizmoSpec] = &[GizmoSpec {
    label: "radial",
    kind: Gizmo::Point {
        x: 2,
        y: 3,
        diameter: None,
        radius: Some((4, RadiusSpace::HalfMinExtent)),
        space: SpatialSpace::Norm01 { basis: PixelBasis::Extent },
    },
}];

/// linear gradient mask: angle + position along axis.
const LINEAR_MASK: &[GizmoSpec] = &[GizmoSpec {
    label: "linear",
    kind: Gizmo::Line {
        angle: 2,
        position: 3,
        space: SpatialSpace::Norm01 { basis: PixelBasis::Extent },
    },
}];

/// circle: centre_x/y as offset-from-centre, radius half-min-extent.
const CIRCLE: &[GizmoSpec] = &[GizmoSpec {
    label: "circle",
    kind: Gizmo::Point {
        x: 3,
        y: 4,
        diameter: None,
        radius: Some((2, RadiusSpace::HalfMinExtent)),
        space: SpatialSpace::OffsetCenter,
    },
}];

/// from text: x_position / y_position anchors (0–1).
const TEXT: &[GizmoSpec] = &[GizmoSpec {
    label: "text",
    kind: Gizmo::Point {
        x: 4,
        y: 5,
        diameter: None,
        radius: None,
        space: SpatialSpace::Norm01 { basis: PixelBasis::Extent },
    },
}];

/// mirror: optional vertical / horizontal split lines.
const MIRROR: &[GizmoSpec] = &[GizmoSpec {
    label: "mirror",
    kind: Gizmo::Axes {
        x: Some(3),
        y: Some(4),
        space: SpatialSpace::Norm01 { basis: PixelBasis::Extent },
    },
}];

/// perspective: eight corner offsets.
const PERSPECTIVE: &[GizmoSpec] = &[GizmoSpec {
    label: "perspective",
    kind: Gizmo::QuadCorners {
        corners: [1, 2, 3, 4, 5, 6, 7, 8],
    },
}];

/// transform (affine): offset + rotation (+ optional uniform-ish scale handles).
const TRANSFORM: &[GizmoSpec] = &[GizmoSpec {
    label: "transform",
    kind: Gizmo::Transform {
        offset_x: 1,
        offset_y: 2,
        rotation: 3,
        scale_x: Some(4),
        scale_y: Some(5),
    },
}];

/// drop shadow: offset in px@1024.
const DROP_SHADOW: &[GizmoSpec] = &[GizmoSpec {
    label: "shadow",
    kind: Gizmo::OffsetPx { x: 1, y: 2 },
}];

/// vignette: centre-fixed radius (corner-normalized unit square).
const VIGNETTE: &[GizmoSpec] = &[GizmoSpec {
    label: "vignette",
    kind: Gizmo::CenterRadius {
        radius: 2,
        space: RadiusSpace::CornerNorm,
    },
}];

/// swirl: centre-fixed radius (half-diagonal).
const SWIRL: &[GizmoSpec] = &[GizmoSpec {
    label: "swirl",
    kind: Gizmo::CenterRadius {
        radius: 2,
        space: RadiusSpace::HalfDiagonal,
    },
}];

/// spherize: centre-fixed radius (half-min-extent).
const SPHERIZE: &[GizmoSpec] = &[GizmoSpec {
    label: "spherize",
    kind: Gizmo::CenterRadius {
        radius: 2,
        space: RadiusSpace::HalfMinExtent,
    },
}];

/// The spatial gizmos this operation exposes on the 2D preview, in draw order
/// (later entries hit-test on top). Empty for the vast majority of operations.
pub fn gizmos(op: &Operation) -> &'static [GizmoSpec] {
    match op {
        Operation::OpImageTransformCrop => CROP,
        Operation::OpColorSampleSamplePixel => SAMPLE_PIXEL,
        Operation::OpImageMaskRadialGradient => RADIAL_MASK,
        Operation::OpImageMaskLinearGradient => LINEAR_MASK,
        Operation::OpImageShapesCircle => CIRCLE,
        Operation::OpImageInputText => TEXT,
        Operation::OpImageTransformMirror => MIRROR,
        Operation::OpImageTransformPerspective => PERSPECTIVE,
        Operation::OpImageTransformAffine => TRANSFORM,
        Operation::OpImageFxDropShadow => DROP_SHADOW,
        Operation::OpImageAdjustmentVignette => VIGNETTE,
        Operation::OpImageTransformSwirl => SWIRL,
        Operation::OpImageTransformSpherize => SPHERIZE,
        _ => &[],
    }
}

impl Gizmo {
    /// Input indices this gizmo **drives** (drag / commit).
    pub fn inputs(&self) -> Vec<usize> {
        match *self {
            Gizmo::Point { x, y, radius, .. } => {
                let mut v = vec![x, y];
                if let Some((r, _)) = radius {
                    v.push(r);
                }
                v
            }
            Gizmo::Rect { x, y, w, h, .. } => vec![x, y, w, h],
            Gizmo::Line { angle, position, .. } => vec![angle, position],
            Gizmo::Axes { x, y, .. } => {
                let mut v = Vec::new();
                if let Some(i) = x {
                    v.push(i);
                }
                if let Some(i) = y {
                    v.push(i);
                }
                v
            }
            Gizmo::QuadCorners { corners } => corners.to_vec(),
            Gizmo::Transform {
                offset_x,
                offset_y,
                rotation,
                scale_x,
                scale_y,
            } => {
                let mut v = vec![offset_x, offset_y, rotation];
                if let Some(s) = scale_x {
                    v.push(s);
                }
                if let Some(s) = scale_y {
                    v.push(s);
                }
                v
            }
            Gizmo::OffsetPx { x, y } => vec![x, y],
            Gizmo::CenterRadius { radius, .. } => vec![radius],
        }
    }

    /// Every input index this gizmo reads, including display-only ones.
    pub fn referenced_inputs(&self) -> Vec<usize> {
        match *self {
            Gizmo::Point {
                x,
                y,
                diameter,
                radius,
                ..
            } => {
                let mut v = vec![x, y];
                if let Some(d) = diameter {
                    v.push(d);
                }
                if let Some((r, _)) = radius {
                    v.push(r);
                }
                v
            }
            other => other.inputs(),
        }
    }

    /// Primary coordinate convention when the gizmo has one.
    pub fn space(&self) -> Option<SpatialSpace> {
        match *self {
            Gizmo::Point { space, .. }
            | Gizmo::Rect { space, .. }
            | Gizmo::Line { space, .. }
            | Gizmo::Axes { space, .. } => Some(space),
            _ => None,
        }
    }
}

impl SpatialSpace {
    /// Convert a value pair in this space to a normalized `[0,1]²` image position.
    pub fn to_unit(self, dims: Option<(u32, u32)>, v: [f32; 2]) -> [f32; 2] {
        match self {
            SpatialSpace::Norm01 { basis } => match (basis, dims) {
                (PixelBasis::Centres, Some((w, h))) => {
                    [centres_to_unit(v[0], w), centres_to_unit(v[1], h)]
                }
                _ => v,
            },
            SpatialSpace::OffsetCenter => [0.5 * (1.0 + v[0]), 0.5 * (1.0 + v[1])],
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
            SpatialSpace::OffsetCenter => [2.0 * u[0] - 1.0, 2.0 * u[1] - 1.0],
        }
    }
}

impl RadiusSpace {
    /// Convert a radius value to a screen-pixel radius for drawing/hit-testing.
    pub fn to_screen_radius(self, value: f32, image_rect: [f32; 2], dims: (u32, u32)) -> f32 {
        let (iw, ih) = dims;
        if iw == 0 || ih == 0 {
            return 0.0;
        }
        let (sw, sh) = (image_rect[0], image_rect[1]);
        let px = match self {
            RadiusSpace::HalfMinExtent => value.max(0.0) * 0.5 * iw.min(ih) as f32,
            RadiusSpace::HalfDiagonal => {
                let half_diag = 0.5 * ((iw as f32).hypot(ih as f32));
                value.max(0.0) * half_diag
            }
            // Vignette: distance is in unit square (0 centre, ~1 corners).
            // Map radius to the shorter half-extent so the ring tracks the UI.
            RadiusSpace::CornerNorm => value.max(0.0) * 0.5 * iw.min(ih) as f32,
        };
        // Average axis scale so the ring stays circular on screen for square pixels.
        let scale = 0.5 * (sw / iw as f32 + sh / ih as f32);
        px * scale
    }

    /// Inverse: screen-pixel radius → stored radius value.
    pub fn from_screen_radius(self, screen_r: f32, image_rect: [f32; 2], dims: (u32, u32)) -> f32 {
        let (iw, ih) = dims;
        if iw == 0 || ih == 0 {
            return 0.0;
        }
        let (sw, sh) = (image_rect[0], image_rect[1]);
        let scale = 0.5 * (sw / iw as f32 + sh / ih as f32);
        if scale <= 1e-9 {
            return 0.0;
        }
        let px = screen_r / scale;
        match self {
            RadiusSpace::HalfMinExtent | RadiusSpace::CornerNorm => {
                let half = 0.5 * iw.min(ih) as f32;
                if half <= 1e-9 {
                    0.0
                } else {
                    px / half
                }
            }
            RadiusSpace::HalfDiagonal => {
                let half_diag = 0.5 * ((iw as f32).hypot(ih as f32));
                if half_diag <= 1e-9 {
                    0.0
                } else {
                    px / half_diag
                }
            }
        }
    }
}

fn centres_to_unit(v: f32, n: u32) -> f32 {
    if n == 0 {
        return v;
    }
    (v * (n.saturating_sub(1)) as f32 + 0.5) / n as f32
}

fn unit_to_centres(u: f32, n: u32) -> f32 {
    if n <= 1 {
        return u;
    }
    (u * n as f32 - 0.5) / (n - 1) as f32
}

#[cfg(test)]
#[path = "gizmo_tests.rs"]
mod tests;
