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

/// Renders `request`'s dialog in `theme`, writes it to `target/ui-snapshots`,
/// and hands the pixels back so a contact sheet can be assembled.
fn shoot_dialog(name: &str, theme: Theme, request: FileDialogRequest) -> Option<image::RgbaImage> {
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
                dialog.update(ui.ctx(), &theme);
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
            Some(image)
        }
        Err(err) => {
            // No GPU adapter (CI, a headless box) — say so rather than fail.
            println!("skipped {name}: could not render ({err})");
            None
        }
    }
}

/// Tiles the per-theme renders into one image, so all four can be compared in
/// a single look instead of four. Cropped to the dialog itself — the harness
/// canvas is mostly empty backdrop.
fn write_contact_sheet(name: &str, shots: &[image::RgbaImage]) {
    const CROP_W: u32 = 920;
    const CROP_H: u32 = 600;
    const GAP: u32 = 8;

    let Some(first) = shots.first() else { return };
    let cell_w = CROP_W.min(first.width());
    let cell_h = CROP_H.min(first.height());

    let cols = 2u32;
    let rows = shots.len().div_ceil(cols as usize) as u32;
    let mut sheet = image::RgbaImage::from_pixel(
        cols * cell_w + (cols - 1) * GAP,
        rows * cell_h + (rows - 1) * GAP,
        image::Rgba([24, 24, 24, 255]),
    );

    for (i, shot) in shots.iter().enumerate() {
        let ox = (i as u32 % cols) * (cell_w + GAP);
        let oy = (i as u32 / cols) * (cell_h + GAP);
        let cell = image::imageops::crop_imm(
            shot,
            0,
            0,
            cell_w.min(shot.width()),
            cell_h.min(shot.height()),
        )
        .to_image();
        image::imageops::replace(&mut sheet, &cell, i64::from(ox), i64::from(oy));
    }

    let path = output_dir().join(format!("sheet_{name}.png"));
    sheet
        .save(&path)
        .unwrap_or_else(|e| panic!("failed writing {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

/// Renders `request` in **every** theme. Chrome that derives a color badly
/// usually looks fine in the one theme it was tuned against and wrong in the
/// other three, so these always go through the whole set.
fn shoot_all_themes(name: &str, request: &FileDialogRequest) {
    let shots: Vec<image::RgbaImage> = Theme::list()
        .into_iter()
        .filter_map(|theme| {
            shoot_dialog(
                &format!("{name}_{}", theme.config_name()),
                theme,
                request.clone(),
            )
        })
        .collect();

    write_contact_sheet(name, &shots);
}

#[test]
#[ignore = "renders with a GPU; run explicitly to look at the UI"]
fn shoot_open_graph_dialog_all_themes() {
    shoot_all_themes("open_graph", &FileDialogRequest::OpenGraph);
}

#[test]
#[ignore = "renders with a GPU; run explicitly to look at the UI"]
fn shoot_save_graph_dialog_all_themes() {
    shoot_all_themes(
        "save_graph",
        &FileDialogRequest::SaveGraph {
            default_dir: Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
            default_stem: "my graph".to_owned(),
        },
    );
}

#[test]
#[ignore = "renders with a GPU; run explicitly to look at the UI"]
fn shoot_pick_folder_dialog_all_themes() {
    shoot_all_themes("pick_folder", &FileDialogRequest::AddLibrary);
}
