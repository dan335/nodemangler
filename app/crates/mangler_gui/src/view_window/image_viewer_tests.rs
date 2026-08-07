//! Unit tests for the viewer's image↔screen rect, now shared by `draw_image`
//! and both preview overlays. A divergence here would draw handles that don't
//! line up with the image under pan or zoom, so the formula is pinned.

use super::*;

fn view() -> Rect {
    Rect::from_min_size(Pos2::new(0.0, 0.0), epaint::Vec2::new(800.0, 600.0))
}

#[test]
fn at_zoom_one_the_image_sits_at_its_natural_size_and_offset() {
    let viewer = ImageViewer::new();
    let r = viewer.displayed_image_rect(view(), 400.0, 300.0);
    assert!((r.width() - 400.0).abs() < 1e-3, "{r:?}");
    assert!((r.height() - 300.0).abs() < 1e-3, "{r:?}");
    // Default position is zero, so the image centres on view.top_left + size/2.
    assert!((r.center().x - 200.0).abs() < 1e-3, "{r:?}");
    assert!((r.center().y - 150.0).abs() < 1e-3, "{r:?}");
}

#[test]
fn zoom_scales_the_rect_inversely() {
    // Larger zoom = smaller on screen (`graph_to_view_space` divides by zoom).
    let mut viewer = ImageViewer::new();
    viewer.zoom = 2.0;
    let r = viewer.displayed_image_rect(view(), 400.0, 300.0);
    assert!((r.width() - 200.0).abs() < 1e-3, "{r:?}");
    assert!((r.height() - 150.0).abs() < 1e-3, "{r:?}");
}

#[test]
fn position_offsets_the_rect_in_graph_space() {
    // `position` is in graph space, so its screen effect is divided by zoom too.
    let mut viewer = ImageViewer::new();
    viewer.zoom = 2.0;
    viewer.position = Pos2::new(100.0, 40.0);
    let r = viewer.displayed_image_rect(view(), 400.0, 300.0);
    assert!((r.center().x - (200.0 / 2.0 + 100.0 / 2.0)).abs() < 1e-3, "{r:?}");
    assert!((r.center().y - (150.0 / 2.0 + 40.0 / 2.0)).abs() < 1e-3, "{r:?}");
}

#[test]
fn it_matches_the_raw_get_rect_call_it_replaced() {
    // Guards against the two ever drifting apart again: `draw_image` used to
    // spell this expression out, and so did the overlay call site.
    let mut viewer = ImageViewer::new();
    viewer.zoom = 1.7;
    viewer.position = Pos2::new(-30.0, 55.0);
    let (v, w, h) = (view(), 640.0, 480.0);
    let expected = viewer.get_rect(
        Pos2::new(v.left() + w * 0.5, v.top() + h * 0.5),
        viewer.zoom,
        w,
        h,
    );
    assert_eq!(viewer.displayed_image_rect(v, w, h), expected);
}

#[test]
fn fit_on_source_change_frames_a_new_source_once() {
    let mut viewer = ImageViewer::new();
    viewer.zoom = 4.0;
    viewer.fit_on_source_change("a", 0, view(), 400.0, 300.0);
    let framed = (viewer.zoom, viewer.position);
    assert!(framed.0 < 4.0, "should have re-fit, zoom {}", framed.0);

    // A second call for the same source must not fight the user's pan/zoom.
    viewer.zoom = 4.0;
    viewer.position = Pos2::new(12.0, 34.0);
    viewer.fit_on_source_change("a", 0, view(), 400.0, 300.0);
    assert_eq!(viewer.zoom, 4.0);
    assert_eq!(viewer.position, Pos2::new(12.0, 34.0));
}

#[test]
fn fit_on_source_change_refits_when_the_source_changes() {
    let mut viewer = ImageViewer::new();
    viewer.fit_on_source_change("a", 0, view(), 400.0, 300.0);
    viewer.zoom = 4.0;

    // A different output index on the same node counts as a different source.
    viewer.fit_on_source_change("a", 1, view(), 400.0, 300.0);
    assert!(viewer.zoom < 4.0, "zoom {}", viewer.zoom);

    viewer.zoom = 4.0;
    viewer.fit_on_source_change("b", 1, view(), 400.0, 300.0);
    assert!(viewer.zoom < 4.0, "zoom {}", viewer.zoom);
}
