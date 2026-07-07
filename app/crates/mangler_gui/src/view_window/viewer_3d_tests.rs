//! Tests for the pure staging-decision logic in `viewer_3d.rs`.
//!
//! `decide_staging` is the GL-free core of `stage_material_uploads`: it maps
//! (channel bound?, content changed?, GPU texture present?) to the action to
//! queue for the paint callback. The GL upload/clear calls themselves need a
//! live context and are verified manually.

use super::{decide_staging, StagingDecision};

/// Channel bound and content changed (or renderer not created yet): upload.
#[test]
fn bound_and_changed_uploads() {
    // renderer_has_texture is irrelevant here — both values must upload.
    assert_eq!(decide_staging(true, true, false), StagingDecision::Upload);
    assert_eq!(decide_staging(true, true, true), StagingDecision::Upload);
}

/// Channel bound but the GPU already has this change_id: nothing to do.
#[test]
fn bound_and_up_to_date_does_nothing() {
    assert_eq!(decide_staging(true, false, false), StagingDecision::None);
    assert_eq!(decide_staging(true, false, true), StagingDecision::None);
}

/// Channel unbound while a stale texture is still on the GPU: clear it.
/// This is the stale-texture bug fix — previously this case was a no-op and
/// the old texture kept rendering after the user set the channel to "None".
#[test]
fn unbound_with_stale_texture_clears() {
    // needs_upload is a don't-care when there is no data; enumerate both.
    assert_eq!(decide_staging(false, false, true), StagingDecision::Clear);
    assert_eq!(decide_staging(false, true, true), StagingDecision::Clear);
}

/// Channel unbound and nothing on the GPU: no-op (Clear must not be queued
/// every frame for never-bound channels — clear_texture is idempotent, but
/// queueing endlessly would still be wasted work).
#[test]
fn unbound_without_texture_does_nothing() {
    assert_eq!(decide_staging(false, false, false), StagingDecision::None);
    assert_eq!(decide_staging(false, true, false), StagingDecision::None);
}
