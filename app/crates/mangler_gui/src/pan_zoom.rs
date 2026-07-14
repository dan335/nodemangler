//! Shared drag-to-pan / scroll-to-zoom input handling for canvas-style panels.
//!
//! The graph editor and the 2D image preview pan a graph-space `position` and
//! zoom about the mouse cursor with identical math; this module is the single
//! implementation of both (they used to be copy-pasted). Callers keep owning
//! their `position`/`zoom` fields — other code reads those directly — and
//! embed a [`PanZoomController`] for the transient drag state.
//!
//! All input is read from the calling `Ui`'s own context. egui scopes input to
//! the viewport currently being rendered, so panels hosted in secondary OS
//! windows see that window's pointer — never the main window's stale one.

use eframe::egui;
use epaint::Pos2;

use crate::view_to_graph_space_pos2;

/// Scroll-wheel-to-zoom speed.
pub const ZOOM_MULTIPLIER: f32 = 0.001;
/// Min/max zoom for the graph editor. Larger zoom = smaller on screen
/// (`view = graph / zoom`).
pub const ZOOM_BOUNDS: [f32; 2] = [0.15, 5.0];
/// Min/max zoom for the 2D image preview. Images can be far larger than a
/// panel (a 4K render in a small split), so the zoom-out ceiling is much
/// higher than the graph editor's, and the floor allows pixel-peeping.
pub const IMAGE_ZOOM_BOUNDS: [f32; 2] = [0.02, 100.0];

/// The pointer position in the viewport this `ui` is rendering into, or a
/// far-offscreen point when the pointer is over another OS window (egui sends
/// `PointerGone` when the cursor leaves a viewport). The offscreen fallback
/// keeps every `rect.contains(cursor)` test false without threading `Option`
/// through all the hit-test code.
pub fn viewport_cursor(ui: &egui::Ui) -> Pos2 {
    ui.ctx()
        .pointer_latest_pos()
        .unwrap_or(Pos2::new(-10_000.0, -10_000.0))
}

/// Scroll-to-zoom anchored on the cursor: the graph point under the cursor
/// stays put. The screen mapping is `view = (graph + position) / zoom`, so
/// keeping `(g + p) / z` fixed at cursor `c` across a zoom change requires
/// `p_new = p_old + c * (z_new - z_old)`.
///
/// The caller gates this on "cursor inside my rect" (and any popup-open
/// checks) — this function only reads the scroll delta and applies the zoom.
/// `bounds` is the caller's min/max zoom ([`ZOOM_BOUNDS`] for the graph
/// editor, [`IMAGE_ZOOM_BOUNDS`] for the image preview).
pub fn zoom_about_cursor(
    ui: &egui::Ui,
    position: &mut Pos2,
    zoom: &mut f32,
    cursor: Pos2,
    bounds: [f32; 2],
) {
    ui.ctx().input(|input_state| {
        let new_zoom = (*zoom * (1.0 + input_state.smooth_scroll_delta.y * ZOOM_MULTIPLIER))
            .clamp(bounds[0], bounds[1]);
        *position += cursor.to_vec2() * (new_zoom - *zoom);
        *zoom = new_zoom;
    });
}

/// Primary-button edge events detected by [`PanZoomController::update`],
/// for callers that need them (the graph editor uses `primary_went_down`
/// for ctrl-click connection deletion).
pub struct PointerEvents {
    pub primary_went_down: bool,
    pub primary_went_up: bool,
}

/// Drag-to-pan state machine shared by the graph editor and the 2D image
/// preview.
///
/// The caller decides when a drag *starts* (its background widget's
/// `drag_started_by`, possibly gated on "not over a node") via
/// [`start_dragging`](Self::start_dragging); `update` then moves `position`
/// by the cursor delta each frame and stops the drag on button release or
/// when the cursor leaves the panel.
pub struct PanZoomController {
    is_dragging: bool,
    /// Cursor position on the previous dragged frame; pan applies the delta.
    last_drag_position: Option<Pos2>,
    /// Primary-button state last frame, for edge detection.
    previous_cursor_primary_down: Option<bool>,
}

impl PanZoomController {
    pub fn new() -> PanZoomController {
        PanZoomController {
            is_dragging: false,
            last_drag_position: None,
            previous_cursor_primary_down: None,
        }
    }

    /// Advance one frame: detect button edges, stop the drag on release or
    /// when the cursor leaves the panel, and while dragging move `position`
    /// by the cursor delta (converted to graph space).
    pub fn update(
        &mut self,
        position: &mut Pos2,
        zoom: f32,
        cursor_position: Pos2,
        cursor_inside: bool,
        cursor_primary_down: bool,
    ) -> PointerEvents {
        let mut events = PointerEvents {
            primary_went_down: false,
            primary_went_up: false,
        };
        if let Some(previous) = self.previous_cursor_primary_down {
            events.primary_went_up = previous && !cursor_primary_down;
            events.primary_went_down = !previous && cursor_primary_down;
        }

        if events.primary_went_up {
            self.stop_dragging();
        }

        if self.is_dragging && !cursor_inside {
            self.stop_dragging();
        }

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                *position += view_to_graph_space_pos2(
                    zoom,
                    cursor_position - last_drag_position.to_vec2(),
                )
                .to_vec2();
            }
            self.last_drag_position = Some(cursor_position);
        }

        self.previous_cursor_primary_down = Some(cursor_primary_down);
        events
    }

    pub fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    pub fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.last_drag_position = None;
    }
}
