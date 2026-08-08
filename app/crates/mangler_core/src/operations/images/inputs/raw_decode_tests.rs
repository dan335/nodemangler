use super::*;

#[test]
fn test_raw_options_default_is_camera_faithful() {
    let opts = RawOptions::default();
    assert_eq!(opts.white_balance, RawWhiteBalance::AsShot);
    assert!(opts.demosaic);
    assert!(!opts.linear_output, "default output must match the pipeline's sRGB convention");
    assert_eq!(opts.max_dimension, None, "the dispatch path must stay lossless");
    assert!(opts.apply_orientation);
    assert_eq!(opts.exposure_stops, 0.0);
}

#[test]
fn test_white_balance_from_label() {
    assert_eq!(RawWhiteBalance::from_label("as shot"), RawWhiteBalance::AsShot);
    assert_eq!(RawWhiteBalance::from_label("camera neutral"), RawWhiteBalance::CameraNeutral);
    assert_eq!(RawWhiteBalance::from_label("none"), RawWhiteBalance::None);
    // Unknown labels fall back to the default rather than erroring.
    assert_eq!(RawWhiteBalance::from_label("nonsense"), RawWhiteBalance::AsShot);
    assert_eq!(RawWhiteBalance::from_label(""), RawWhiteBalance::AsShot);
}

#[cfg(feature = "raw")]
mod with_rawler {
    use super::*;

    /// The stock options must reproduce rawler's own default pipeline exactly,
    /// so `from file` behaves like every other RAW developer's defaults.
    #[test]
    fn test_default_options_match_rawler_default_pipeline() {
        assert_eq!(steps_for(&RawOptions::default()), RawDevelop::default().steps);
    }

    #[test]
    fn test_linear_output_drops_only_the_srgb_step() {
        let opts = RawOptions { linear_output: true, ..Default::default() };
        let steps = steps_for(&opts);
        assert!(!steps.contains(&ProcessingStep::SRgb));
        assert!(steps.contains(&ProcessingStep::Demosaic));
        assert!(steps.contains(&ProcessingStep::WhiteBalance));
        assert!(steps.contains(&ProcessingStep::Calibrate));
    }

    #[test]
    fn test_white_balance_none_drops_only_the_wb_step() {
        let opts = RawOptions { white_balance: RawWhiteBalance::None, ..Default::default() };
        let steps = steps_for(&opts);
        assert!(!steps.contains(&ProcessingStep::WhiteBalance));
        assert!(steps.contains(&ProcessingStep::Calibrate));
        assert!(steps.contains(&ProcessingStep::SRgb));
    }

    #[test]
    fn test_demosaic_off_drops_demosaic_and_calibrate() {
        let opts = RawOptions { demosaic: false, ..Default::default() };
        let steps = steps_for(&opts);
        assert!(!steps.contains(&ProcessingStep::Demosaic));
        assert!(!steps.contains(&ProcessingStep::Calibrate));
        // The crops and rescale are never optional.
        assert!(steps.contains(&ProcessingStep::Rescale));
        assert!(steps.contains(&ProcessingStep::CropActiveArea));
        assert!(steps.contains(&ProcessingStep::CropDefault));
    }

    /// Whatever the option combination, the vector must stay in rawler's
    /// canonical order — that is what makes the result correct whether rawler
    /// iterates the vector or merely tests membership.
    #[test]
    fn test_steps_always_in_canonical_order() {
        const CANONICAL: [ProcessingStep; 7] = [
            ProcessingStep::Rescale,
            ProcessingStep::Demosaic,
            ProcessingStep::CropActiveArea,
            ProcessingStep::WhiteBalance,
            ProcessingStep::Calibrate,
            ProcessingStep::CropDefault,
            ProcessingStep::SRgb,
        ];
        let rank = |s: &ProcessingStep| CANONICAL.iter().position(|c| c == s).unwrap();

        for demosaic in [true, false] {
            for linear in [true, false] {
                for wb in [
                    RawWhiteBalance::AsShot,
                    RawWhiteBalance::CameraNeutral,
                    RawWhiteBalance::None,
                ] {
                    let steps = steps_for(&RawOptions {
                        demosaic,
                        linear_output: linear,
                        white_balance: wb,
                        ..Default::default()
                    });
                    let ranks: Vec<_> = steps.iter().map(rank).collect();
                    assert!(
                        ranks.windows(2).all(|w| w[0] < w[1]),
                        "steps out of canonical order for demosaic={demosaic} linear={linear} wb={wb:?}: {steps:?}"
                    );
                }
            }
        }
    }

    // A 3x2 single-channel source:  row0 = 0 1 2   row1 = 3 4 5
    const W: usize = 3;
    const H: usize = 2;
    const SRC: [f32; 6] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];

    fn oriented(o: Orientation) -> FloatImage {
        orient_into(&SRC, W, H, 1, 1, o).expect("orientation must produce a valid image")
    }

    #[test]
    fn test_orientation_normal_is_identity() {
        let img = oriented(Orientation::Normal);
        assert_eq!((img.width(), img.height()), (3, 2));
        assert_eq!(img.as_slice(), &SRC);
    }

    #[test]
    fn test_orientation_unknown_does_not_rotate() {
        // The camera didn't say; guessing would be worse than doing nothing.
        let img = oriented(Orientation::Unknown);
        assert_eq!((img.width(), img.height()), (3, 2));
        assert_eq!(img.as_slice(), &SRC);
    }

    /// The non-transposing flips have an unambiguous expected result regardless
    /// of rawler's transpose convention.
    #[test]
    fn test_orientation_flips_are_exact() {
        assert_eq!(
            oriented(Orientation::HorizontalFlip).as_slice(),
            &[2.0, 1.0, 0.0, 5.0, 4.0, 3.0]
        );
        assert_eq!(
            oriented(Orientation::VerticalFlip).as_slice(),
            &[3.0, 4.0, 5.0, 0.0, 1.0, 2.0]
        );
        assert_eq!(
            oriented(Orientation::Rotate180).as_slice(),
            &[5.0, 4.0, 3.0, 2.0, 1.0, 0.0]
        );
    }

    /// Every orientation must preserve the pixel set and the total count, and
    /// must swap the dimensions exactly when it transposes.
    #[test]
    fn test_all_orientations_permute_without_loss() {
        let all = [
            Orientation::Normal,
            Orientation::HorizontalFlip,
            Orientation::Rotate180,
            Orientation::VerticalFlip,
            Orientation::Transpose,
            Orientation::Rotate90,
            Orientation::Transverse,
            Orientation::Rotate270,
            Orientation::Unknown,
        ];
        for o in all {
            let img = oriented(o);
            assert_eq!(img.as_slice().len(), SRC.len(), "{o:?} changed the pixel count");

            let (transpose, _, _) = if matches!(o, Orientation::Unknown) {
                (false, false, false)
            } else {
                o.to_flips()
            };
            let expected = if transpose { (2, 3) } else { (3, 2) };
            assert_eq!((img.width(), img.height()), expected, "{o:?} has wrong dimensions");

            let mut got: Vec<f32> = img.as_slice().to_vec();
            got.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert_eq!(got, SRC.to_vec(), "{o:?} lost or duplicated pixels");
        }
    }

    /// The quarter turns must be genuine opposites — this is the classic
    /// Rotate90/Rotate270 transposition bug, invisible on landscape images.
    #[test]
    fn test_quarter_turns_are_opposites() {
        let cw = oriented(Orientation::Rotate90);
        let ccw = oriented(Orientation::Rotate270);
        assert_ne!(cw.as_slice(), ccw.as_slice());

        // Applying one then the other must return the original.
        let back = orient_into(
            cw.as_slice(),
            cw.width() as usize,
            cw.height() as usize,
            1,
            1,
            Orientation::Rotate270,
        )
        .unwrap();
        assert_eq!((back.width(), back.height()), (3, 2));
        assert_eq!(back.as_slice(), &SRC, "Rotate90 then Rotate270 must be identity");
    }

    /// A 4-colour CFA's fourth layer must be dropped, not smuggled through as
    /// alpha — the whole pipeline would read a 4-channel image as RGBA.
    #[test]
    fn test_four_channel_source_is_truncated_to_rgb() {
        let src = [1.0, 2.0, 3.0, 99.0, 4.0, 5.0, 6.0, 98.0];
        let img = orient_into(&src, 2, 1, 4, 3, Orientation::Normal).unwrap();
        assert_eq!(img.channels(), 3);
        assert_eq!(img.as_slice(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_exposure_in_linear_light_is_a_plain_scale() {
        let mut img = FloatImage::from_raw(2, 1, 3, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.0]).unwrap();
        apply_exposure(&mut img, 1.0, true);
        let got = img.as_slice();
        for (got, want) in got.iter().zip([0.2, 0.4, 0.6, 0.8, 1.0, 0.0]) {
            assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
        }
    }

    /// Highlights must be allowed past 1.0 — clamping here would throw away the
    /// headroom the recovery nodes downstream exist to use.
    #[test]
    fn test_exposure_does_not_clamp_highlights() {
        let mut img = FloatImage::from_raw(1, 1, 3, vec![0.8, 0.9, 1.0]).unwrap();
        apply_exposure(&mut img, 2.0, true);
        assert!(img.as_slice().iter().all(|v| *v > 1.0));
    }

    #[test]
    fn test_exposure_on_srgb_data_brightens_through_linear_light() {
        let mut img = FloatImage::from_raw(1, 1, 3, vec![0.5, 0.5, 0.5]).unwrap();
        apply_exposure(&mut img, 1.0, false);
        // Doubling in linear light, not a naive 0.5 -> 1.0 in encoded space.
        for value in img.as_slice() {
            assert!(*value > 0.65 && *value < 0.75, "unexpected encoded value {value}");
        }
    }

    #[test]
    fn test_exposure_leaves_alpha_alone() {
        let mut img = FloatImage::from_raw(1, 1, 4, vec![0.1, 0.2, 0.3, 0.5]).unwrap();
        apply_exposure(&mut img, 1.0, true);
        assert_eq!(img.as_slice()[3], 0.5, "alpha must not be scaled by exposure");
    }

    #[test]
    fn test_decode_rejects_a_non_raw_file() {
        // A PNG masquerading as a CR3 must error, and must not panic — rawler
        // catches decoder panics internally.
        let path = std::env::temp_dir().join("mangler_raw_decode_bogus.cr3");
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([1u8, 2, 3]));
        img.save_with_format(&path, image::ImageFormat::Png).unwrap();

        let result = decode_raw(&path, &RawOptions::default());
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err(), "a PNG named .cr3 must not decode as RAW");
    }

    #[test]
    fn test_decode_reports_a_missing_file() {
        let result = decode_raw(std::path::Path::new("/nonexistent/nope.cr3"), &RawOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_preview_rejects_a_non_raw_file() {
        let path = std::env::temp_dir().join("mangler_raw_preview_bogus.cr3");
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([1u8, 2, 3]));
        img.save_with_format(&path, image::ImageFormat::Png).unwrap();

        let result = decode_raw_preview_rgba8(&path, 192);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err(), "a PNG named .cr3 must not yield a RAW preview");
    }

    #[test]
    fn test_preview_reports_a_missing_file() {
        let result =
            decode_raw_preview_rgba8(std::path::Path::new("/nonexistent/nope.cr3"), 192);
        assert!(result.is_err());
    }

    /// Real-file preview path. Same fixture env as `test_decode_real_raw_fixture`.
    #[test]
    fn test_preview_real_raw_fixture() {
        let Ok(path) = std::env::var("NODEMANGLER_RAW_FIXTURE") else {
            return;
        };
        let path = std::path::PathBuf::from(path);

        let (pixels, w, h) = decode_raw_preview_rgba8(&path, 192)
            .unwrap_or_else(|e| panic!("preview failed for {}: {e}", path.display()));
        assert!(w.max(h) <= 192);
        assert!(w >= 16 && h >= 16, "implausibly tiny preview {w}x{h}");
        assert_eq!(pixels.len(), (w * h * 4) as usize);
    }

    /// End-to-end decode of a real camera file. Skipped unless
    /// `NODEMANGLER_RAW_FIXTURE` points at one, so no multi-megabyte blob has
    /// to live in git.
    #[test]
    fn test_decode_real_raw_fixture() {
        let Ok(path) = std::env::var("NODEMANGLER_RAW_FIXTURE") else { return };
        let path = std::path::PathBuf::from(path);

        let img = decode_raw(&path, &RawOptions::default())
            .unwrap_or_else(|e| panic!("failed to decode {}: {e}", path.display()));

        assert_eq!(img.channels(), 3, "a developed colour raw must be RGB");
        assert!(img.width() > 1000 && img.height() > 1000, "implausible dimensions");

        // Accumulate in f64: an f32 accumulator saturates well before the end of
        // a 25-megapixel buffer and silently reports a far-too-low mean.
        let mut sums = [0f64; 3];
        for pixel in img.as_slice().chunks_exact(3) {
            for (sum, component) in sums.iter_mut().zip(pixel) {
                *sum += *component as f64;
            }
        }
        let count = (img.as_slice().len() / 3) as f64;
        let [r, g, b] = sums.map(|s| s / count);

        // A real photograph should be neither black nor blown out.
        let mean = (r + g + b) / 3.0;
        assert!(mean > 0.01 && mean < 0.99, "implausible mean brightness {mean}");

        // A developed raw is white-balanced and matrixed, so no channel should
        // dominate. Skipping white balance or the colour matrix shows up here as
        // a strong green cast, the classic raw-pipeline failure.
        let (lo, hi) = (r.min(g).min(b), r.max(g).max(b));
        assert!(hi / lo < 2.0, "strong colour cast: R={r:.4} G={g:.4} B={b:.4}");

        // Capping the long edge must actually cap it.
        let small = decode_raw(
            &path,
            &RawOptions { max_dimension: Some(512), ..Default::default() },
        )
        .unwrap();
        assert!(small.width().max(small.height()) <= 512);
    }
}
