use super::*;

use crate::input::Input;
use crate::value::Value;

/// A near-straight horizontal test centerline; relies on the initial wobble /
/// bank roughness to start meandering (a perfectly straight line has zero
/// curvature).
fn line_curve() -> Curve {
    Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

/// Builds the 16 default inputs. `curve` of `None` uses the default arc;
/// `erodibility` of `None` leaves the image input at its unconnected 1x1
/// placeholder. Individual tests mutate entries by index afterwards.
fn make_inputs(width: i32, height: i32, curve: Option<Curve>, erodibility: Option<FloatImage>) -> Vec<Input> {
    let erod_value = match erodibility {
        Some(img) => Value::Image { data: Arc::new(img), change_id: get_id() },
        None => Value::Image { data: default_image(), change_id: get_id() },
    };
    vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("curve".to_string(), Value::Curve(curve.unwrap_or_default()), None, None),
        Input::new("erodibility".to_string(), erod_value, None, None),
        Input::new("iterations".to_string(), Value::Integer(800), None, None),
        Input::new("migration rate".to_string(), Value::Decimal(0.4), None, None),
        Input::new("channel width".to_string(), Value::Decimal(10.0), None, None),
        Input::new("meander scale".to_string(), Value::Decimal(10.0), None, None),
        Input::new("upstream fraction".to_string(), Value::Decimal(0.35), None, None),
        Input::new("bend widening".to_string(), Value::Decimal(0.1), None, None),
        Input::new("width noise".to_string(), Value::Decimal(0.1), None, None),
        Input::new("bend wavelength".to_string(), Value::Decimal(12.5), None, None),
        Input::new("cutoff distance".to_string(), Value::Decimal(1.5), None, None),
        Input::new("initial wobble".to_string(), Value::Decimal(0.25), None, None),
        Input::new("bank roughness".to_string(), Value::Decimal(0.25), None, None),
    ]
}

/// Overwrites input `idx` with a new value, keeping the same name.
fn set(inputs: &mut [Input], idx: usize, value: Value) {
    inputs[idx] = Input::new(inputs[idx].name.clone(), value, None, None);
}

/// Extracts the pixel data of the response at `index`.
fn image_pixels(result: &OperationResponse, index: usize) -> Vec<Vec<f32>> {
    match &result.responses[index].value {
        Value::Image { data, .. } => data.pixels().map(|p| p.to_vec()).collect(),
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Extracts the curve of the response at `index`.
fn out_curve(result: &OperationResponse, index: usize) -> Curve {
    match &result.responses[index].value {
        Value::Curve(c) => c.clone(),
        other => panic!("Expected Curve, got {:?}", other),
    }
}

/// Arc length of a Curve's point polyline.
fn curve_length(c: &Curve) -> f32 {
    c.points
        .windows(2)
        .map(|s| {
            let dx = s[1][0] - s[0][0];
            let dy = s[1][1] - s[0][1];
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveSimulationMeander::settings();
    assert_eq!(s.name, "meander");
    assert_eq!(OpCurveSimulationMeander::create_inputs().len(), 16);
    assert_eq!(OpCurveSimulationMeander::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_output_shapes() {
    let mut inputs = make_inputs(64, 32, Some(line_curve()), None);
    set(&mut inputs, 5, Value::Integer(50));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    assert!(matches!(result.responses[0].value, Value::Curve(_)), "output 0 should be a Curve");
    for i in 1..4 {
        match &result.responses[i].value {
            Value::Image { data, .. } => {
                assert_eq!(data.width(), 64, "output {i} width");
                assert_eq!(data.height(), 32, "output {i} height");
                assert_eq!(data.channels(), 1, "output {i} channels");
            }
            other => panic!("Expected Image, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_deterministic() {
    let build = || {
        let mut inputs = make_inputs(64, 64, Some(line_curve()), None);
        set(&mut inputs, 5, Value::Integer(150));
        inputs
    };
    let r1 = OpCurveSimulationMeander::run(&mut build()).await.unwrap();
    let r2 = OpCurveSimulationMeander::run(&mut build()).await.unwrap();
    assert_eq!(out_curve(&r1, 0).points, out_curve(&r2, 0).points, "curve output not deterministic");
    for i in 1..4 {
        assert_eq!(image_pixels(&r1, i), image_pixels(&r2, i), "image output {i} not deterministic");
    }
}

#[tokio::test]
async fn test_zero_iterations_passthrough() {
    // The default curve is the 3-point Smooth arc; it must pass through with
    // interpolation and points untouched, with an empty oxbow mask and a
    // non-empty river mask.
    let input_curve = Curve::default();
    let mut inputs = make_inputs(64, 64, Some(input_curve.clone()), None);
    set(&mut inputs, 5, Value::Integer(0));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();

    assert_eq!(out_curve(&result, 0), input_curve, "curve should pass through unchanged");
    assert!(image_pixels(&result, 1).iter().any(|p| p[0] > 0.5), "river mask should draw the curve");
    assert!(image_pixels(&result, 2).iter().all(|p| p[0] == 0.0), "oxbows should be black");
}

#[tokio::test]
async fn test_curve_evolves_and_endpoints_pinned() {
    let input_curve = line_curve();
    let mut inputs = make_inputs(64, 64, Some(input_curve.clone()), None);
    set(&mut inputs, 5, Value::Integer(400));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let evolved = out_curve(&result, 0);

    assert_ne!(evolved.points, input_curve.points, "curve should have evolved");
    assert!(
        curve_length(&evolved) > curve_length(&input_curve) * 1.02,
        "meandering should lengthen the river: {} vs {}",
        curve_length(&evolved),
        curve_length(&input_curve)
    );
    let first = evolved.points.first().unwrap();
    let last = evolved.points.last().unwrap();
    assert!((first[0] - 0.1).abs() < 1e-5 && (first[1] - 0.5).abs() < 1e-5, "start endpoint moved: {first:?}");
    assert!((last[0] - 0.9).abs() < 1e-5 && (last[1] - 0.5).abs() < 1e-5, "end endpoint moved: {last:?}");
}

#[tokio::test]
async fn test_high_iteration_stability() {
    // The memory-flagged tail: a growth instability must stay bounded at the
    // iteration limit. Saturated curvature + displacement clamp + cutoffs.
    let input_curve = line_curve();
    let mut inputs = make_inputs(64, 64, Some(input_curve.clone()), None);
    set(&mut inputs, 5, Value::Integer(2000));
    set(&mut inputs, 6, Value::Decimal(0.8));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let evolved = out_curve(&result, 0);

    assert!(evolved.points.len() >= 2);
    assert!(evolved.points.len() <= 4000, "output curve too large: {}", evolved.points.len());
    for (i, p) in evolved.points.iter().enumerate() {
        assert!(p[0].is_finite() && p[1].is_finite(), "point {i} not finite: {p:?}");
        assert!((-1.0..=2.0).contains(&p[0]) && (-1.0..=2.0).contains(&p[1]), "point {i} escaped: {p:?}");
    }
    assert!(
        curve_length(&evolved) < curve_length(&input_curve) * 20.0,
        "cutoffs should bound the river length, got {}x",
        curve_length(&evolved) / curve_length(&input_curve)
    );
}

#[tokio::test]
async fn test_cutoffs_produce_oxbows() {
    let mut inputs = make_inputs(128, 128, Some(line_curve()), None);
    set(&mut inputs, 5, Value::Integer(800));
    set(&mut inputs, 6, Value::Decimal(0.8));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let oxbow_pixels = image_pixels(&result, 2);
    let lit = oxbow_pixels.iter().filter(|p| p[0] > 0.5).count();
    assert!(lit > 0, "800 aggressive iterations should have produced at least one oxbow cutoff");
}

#[tokio::test]
async fn test_oxbows_render_at_channel_width() {
    // Oxbow widths are captured from the simulation-scale widths array at
    // cutoff time but must be rescaled to the visual channel width (index 7)
    // before rasterizing — a thin drawn river keeps thin oxbows even when the
    // meander scale driving the physics is larger. Same seed/scale in both
    // runs = identical cutoff loops, so only the stroke width differs.
    let base = || {
        let mut inputs = make_inputs(128, 128, Some(line_curve()), None);
        set(&mut inputs, 5, Value::Integer(800));
        set(&mut inputs, 6, Value::Decimal(0.8));
        inputs
    };
    let mut thin = base();
    let mut thick = base();
    set(&mut thin, 7, Value::Decimal(5.0));
    set(&mut thick, 7, Value::Decimal(20.0));

    let r_thin = OpCurveSimulationMeander::run(&mut thin).await.unwrap();
    let r_thick = OpCurveSimulationMeander::run(&mut thick).await.unwrap();

    let lit = |r: &OperationResponse| image_pixels(r, 2).iter().filter(|p| p[0] > 0.5).count();
    let (ox_thin, ox_thick) = (lit(&r_thin), lit(&r_thick));
    assert!(ox_thin > 0 && ox_thick > 0, "both runs should produce oxbows: {ox_thin} vs {ox_thick}");
    // Before the rescale fix the two masks were pixel-identical (oxbows
    // stroked at the meander scale regardless of channel width), so any real
    // margin here is the regression signal. The factor is well under the 4x
    // width ratio because the rasterizer's 0.5px minimum stroke radius props
    // up the thin run at this output size.
    assert!(
        ox_thick as f64 > ox_thin as f64 * 1.3,
        "oxbows should stroke at the channel width, not the meander scale: {ox_thin} lit at width 5 vs {ox_thick} at width 20"
    );
}

#[tokio::test]
async fn test_erodibility_modulates() {
    // All-black erodibility freezes migration: only the initial wobble
    // (fraction of a channel width) remains. Unconnected = uniform banks moves
    // the channel much further.
    let black = FloatImage::new(8, 8, 1);
    let run = |erod: Option<FloatImage>| async {
        let mut inputs = make_inputs(64, 64, Some(line_curve()), erod);
        set(&mut inputs, 5, Value::Integer(200));
        OpCurveSimulationMeander::run(&mut inputs).await.unwrap()
    };
    let frozen = out_curve(&run(Some(black)).await, 0);
    let free = out_curve(&run(None).await, 0);

    let max_dev = |c: &Curve| {
        c.points.iter().map(|p| (p[1] - 0.5).abs()).fold(0.0f32, f32::max)
    };
    assert!(max_dev(&frozen) < 0.01, "black erodibility should freeze the banks, deviated {}", max_dev(&frozen));
    assert!(max_dev(&free) > max_dev(&frozen) * 2.0, "uniform banks should migrate further than frozen banks");
}

#[tokio::test]
async fn test_degenerate_curves() {
    for points in [vec![], vec![[0.5, 0.5]], vec![[0.5, 0.5], [0.5, 0.5]]] {
        let input_curve = Curve {
            points,
            closed: false,
            interpolation: CurveInterpolation::Linear,
            handles: Vec::new(),
        };
        let mut inputs = make_inputs(32, 32, Some(input_curve.clone()), None);
        let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
        assert_eq!(out_curve(&result, 0), input_curve, "degenerate curve should pass through");
        for i in 1..4 {
            assert!(image_pixels(&result, i).iter().all(|p| p[0] == 0.0), "output {i} should be black");
        }
    }
}

#[tokio::test]
async fn test_width_grows_downstream() {
    // Passthrough render (iterations 0) of the horizontal line: the stroke
    // must be visibly thinner near the source (left) than near the mouth
    // (right). Channel width 40 so both ends are comfortably super-pixel.
    let mut inputs = make_inputs(256, 256, Some(line_curve()), None);
    set(&mut inputs, 5, Value::Integer(0));
    set(&mut inputs, 7, Value::Decimal(40.0));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let mask = image_pixels(&result, 1);
    let column_thickness = |x: usize| (0..256).filter(|y| mask[y * 256 + x][0] > 0.5).count();
    let upstream = column_thickness(45);
    let downstream = column_thickness(210);
    assert!(
        downstream >= upstream + 3,
        "channel should widen downstream: upstream {upstream}px vs downstream {downstream}px"
    );
}

#[tokio::test]
async fn test_channel_width_is_render_only() {
    // Channel width (index 7) is the visual stroke only; meander scale (index 8)
    // drives the shape. Doubling channel width must widen the rendered river
    // without moving the evolved centerline, while both stay deterministic.
    let base = || {
        let mut inputs = make_inputs(128, 128, Some(line_curve()), None);
        set(&mut inputs, 5, Value::Integer(300));
        inputs
    };
    let mut thin = base();
    let mut thick = base();
    set(&mut thick, 7, Value::Decimal(20.0)); // 2x the make_inputs channel width

    let r_thin = OpCurveSimulationMeander::run(&mut thin).await.unwrap();
    let r_thick = OpCurveSimulationMeander::run(&mut thick).await.unwrap();

    // Same meander scale -> identical evolved centerline.
    assert_eq!(
        out_curve(&r_thin, 0).points,
        out_curve(&r_thick, 0).points,
        "channel width must not affect the evolved curve"
    );
    // Wider channel width -> more lit pixels in the river mask.
    let lit = |r: &OperationResponse| image_pixels(r, 1).iter().filter(|p| p[0] > 0.5).count();
    assert!(
        lit(&r_thick) > lit(&r_thin),
        "wider channel width should render a wider river: {} vs {}",
        lit(&r_thick),
        lit(&r_thin)
    );
}

#[tokio::test]
async fn test_meander_scale_drives_shape() {
    // Meander scale (index 8) changes the evolved shape; channel width is held
    // fixed. A different scale must produce a different centerline.
    let base = || {
        let mut inputs = make_inputs(128, 128, Some(line_curve()), None);
        set(&mut inputs, 5, Value::Integer(300));
        inputs
    };
    let mut small = base();
    let mut large = base();
    set(&mut large, 8, Value::Decimal(30.0)); // 3x the make_inputs meander scale

    let r_small = OpCurveSimulationMeander::run(&mut small).await.unwrap();
    let r_large = OpCurveSimulationMeander::run(&mut large).await.unwrap();

    assert_ne!(
        out_curve(&r_small, 0).points,
        out_curve(&r_large, 0).points,
        "meander scale should reshape the evolved curve"
    );
}

#[tokio::test]
async fn test_bank_roughness_alone_evolves() {
    // No initial wobble: only the continuous bank noise can break the straight
    // line's symmetry, and the convective instability amplifies it.
    let input_curve = line_curve();
    let mut inputs = make_inputs(64, 64, Some(input_curve.clone()), None);
    set(&mut inputs, 5, Value::Integer(600));
    set(&mut inputs, 14, Value::Decimal(0.0)); // initial wobble off
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let evolved = out_curve(&result, 0);
    assert!(
        curve_length(&evolved) > curve_length(&input_curve) * 1.01,
        "bank roughness alone should seed meandering: {} vs {}",
        curve_length(&evolved),
        curve_length(&input_curve)
    );
}

#[tokio::test]
async fn test_initial_wobble_alone_evolves() {
    // No continuous noise: the one-time perturbation is seeded in the growing
    // wavelength band, so the instability still develops bends from it.
    let input_curve = line_curve();
    let mut inputs = make_inputs(64, 64, Some(input_curve.clone()), None);
    set(&mut inputs, 5, Value::Integer(400));
    set(&mut inputs, 15, Value::Decimal(0.0)); // bank roughness off
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let evolved = out_curve(&result, 0);
    assert!(
        curve_length(&evolved) > curve_length(&input_curve) * 1.01,
        "initial wobble alone should grow meanders: {} vs {}",
        curve_length(&evolved),
        curve_length(&input_curve)
    );
}

#[tokio::test]
async fn test_bend_widening_is_render_only() {
    // Bend widening (index 10) fattens the rendered stroke at bends without
    // touching the evolved centerline.
    let base = || {
        let mut inputs = make_inputs(128, 128, Some(line_curve()), None);
        set(&mut inputs, 5, Value::Integer(300));
        set(&mut inputs, 11, Value::Decimal(0.0)); // width noise off in both
        inputs
    };
    let mut plain = base();
    let mut widened = base();
    set(&mut plain, 10, Value::Decimal(0.0));
    set(&mut widened, 10, Value::Decimal(1.5));

    let r_plain = OpCurveSimulationMeander::run(&mut plain).await.unwrap();
    let r_widened = OpCurveSimulationMeander::run(&mut widened).await.unwrap();

    assert_eq!(
        out_curve(&r_plain, 0).points,
        out_curve(&r_widened, 0).points,
        "bend widening must not affect the evolved curve"
    );
    let lit = |r: &OperationResponse| image_pixels(r, 1).iter().filter(|p| p[0] > 0.5).count();
    assert!(
        lit(&r_widened) > lit(&r_plain),
        "bend widening should fatten the rendered river: {} vs {}",
        lit(&r_widened),
        lit(&r_plain)
    );
}

#[tokio::test]
async fn test_migration_map_ages() {
    let mut inputs = make_inputs(128, 128, Some(line_curve()), None);
    set(&mut inputs, 5, Value::Integer(600));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    let map = image_pixels(&result, 3);
    let max = map.iter().map(|p| p[0]).fold(0.0f32, f32::max);
    assert!(max > 0.99, "current channel should be stamped at age ~1.0, max {max}");
    let intermediate = map.iter().filter(|p| p[0] > 0.05 && p[0] < 0.9).count();
    assert!(intermediate > 0, "swept corridor should contain intermediate ages, not a binary mask");
}

/// Renders the meander evolution sweep and saves PNGs of the river mask at
/// several iteration counts plus the oxbow and migration maps at the end. Run
/// with `cargo test -p mangler_core meander::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let dir = "C:/Users/danph/AppData/Local/Temp/claude/D--rust-nodemangler-app/d341e4e9-15f9-4589-a1f7-5baff68d372a/scratchpad";
    let save = |result: &OperationResponse, index: usize, name: String| {
        match &result.responses[index].value {
            Value::Image { data, .. } => { data.to_dynamic().save(format!("{dir}/{name}.png")).unwrap(); }
            other => panic!("Expected Image, got {other:?}"),
        }
    };

    for iterations in [0, 50, 100, 200, 400, 800] {
        let mut inputs = make_inputs(512, 512, Some(line_curve()), None);
        set(&mut inputs, 5, Value::Integer(iterations));
        let start = std::time::Instant::now();
        let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
        println!(
            "meander 512x512, {iterations} iterations: {:?}, {} output points",
            start.elapsed(),
            out_curve(&result, 0).points.len()
        );
        save(&result, 1, format!("meander_mask_{iterations:04}"));
        if iterations == 800 {
            save(&result, 2, "meander_oxbows_0800".to_string());
            save(&result, 3, "meander_migration_0800".to_string());
        }
    }

    // Mismatched scales: a thin rendered river (width 3) carrying broad
    // meanders (scale 20) — the case where oxbows used to render at the
    // meander scale and blob out ~7x thicker than the channel.
    let mut inputs = make_inputs(512, 512, Some(line_curve()), None);
    set(&mut inputs, 5, Value::Integer(800));
    set(&mut inputs, 7, Value::Decimal(3.0));
    set(&mut inputs, 8, Value::Decimal(20.0));
    let result = OpCurveSimulationMeander::run(&mut inputs).await.unwrap();
    save(&result, 1, "meander_mask_thin_channel_broad_scale".to_string());
}
