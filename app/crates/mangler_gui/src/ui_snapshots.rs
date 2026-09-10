//! Renders real UI chrome headlessly to PNG files, so it can be *looked at*
//! without launching the app and taking a screenshot.
//!
//! These are not assertions — they always pass. They exist because visual
//! defects (an unthemed widget, a cramped row, clipped text) are invisible to
//! ordinary tests, and iterating on them by launching the GUI is slow and
//! needs a human at the screen.
//!
//! Run with:
//!
//! ```text
//! cargo test -p mangler_gui --features ui-snapshots -- --ignored --nocapture
//! ```
//!
//! and open the paths it prints (they land in `target/ui-snapshots/`).
//! Behind the off-by-default `ui-snapshots` feature (wgpu is a long compile)
//! *and* `#[ignore]`d, so the normal test run stays fast and headless-safe.

use eframe::egui;
use egui_kittest::Harness;

use crate::file_dialog::{AppFileDialog, FileDialogRequest};
use crate::themes::theme::{set_theme, Theme};

/// Where the PNGs land. Under `target/` so they are never committed.
fn output_dir() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/ui-snapshots");
    std::fs::create_dir_all(&dir).expect("should be able to create the snapshot dir");
    dir
}

/// Applies the real app's fonts and theme, so a snapshot shows what the user
/// actually sees rather than egui's defaults.
fn dress_context(ctx: &egui::Context, theme: Theme) {
    crate::app::setup_fonts(ctx);
    set_theme(ctx, theme);
}

/// Renders `request`'s dialog in `theme` and writes it to `target/ui-snapshots`.
fn shoot_dialog(name: &str, theme: Theme, request: FileDialogRequest) {
    let mut dialog = AppFileDialog::new();
    dialog.open(request, Some("snapshot".to_owned()));

    let mut dressed = false;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(
            move |ui, dialog: &mut AppFileDialog| {
                if !dressed {
                    dress_context(ui.ctx(), theme.clone());
                    dressed = true;
                }
                // The dialog paints its own window on the context; the panel
                // this closure runs in is just the backdrop.
                dialog.update(ui.ctx());
            },
            dialog,
        );

    // Let the fonts land and the window settle before capturing.
    harness.run();

    match harness.render() {
        Ok(image) => {
            let path = output_dir().join(format!("{name}.png"));
            image
                .save(&path)
                .unwrap_or_else(|e| panic!("failed writing {}: {e}", path.display()));
            println!("wrote {}", path.display());
        }
        Err(err) => {
            // No GPU adapter (CI, a headless box) — say so rather than fail.
            println!("skipped {name}: could not render ({err})");
        }
    }
}

#[test]
#[ignore = "renders with a GPU; run explicitly to look at the UI"]
fn shoot_open_graph_dialog() {
    shoot_dialog(
        "open_graph_dark_green",
        Theme::DarkGreen,
        FileDialogRequest::OpenGraph,
    );
}

#[test]
#[ignore = "renders with a GPU; run explicitly to look at the UI"]
fn shoot_save_graph_dialog() {
    shoot_dialog(
        "save_graph_dark_green",
        Theme::DarkGreen,
        FileDialogRequest::SaveGraph {
            default_dir: Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
            default_stem: "my graph".to_owned(),
        },
    );
}

#[test]
#[ignore = "renders with a GPU; run explicitly to look at the UI"]
fn shoot_open_graph_dialog_light() {
    shoot_dialog("open_graph_light", Theme::Light, FileDialogRequest::OpenGraph);
}
