//! Timing table for the per-pixel adjustment operations at a realistic
//! photographic resolution.
//!
//! The `all_operations_perf` benchmark runs everything at 512x512, where a
//! serial pass over 260k pixels is fast enough that nothing stands out. These
//! nodes are the ones a user drags a slider on while looking at a 24-megapixel
//! photo, and that is the size at which a single-threaded loop is felt.
//!
//! `#[ignore]`d and printing rather than asserting: wall-clock thresholds turn
//! into flaky tests on shared CI. Run it with
//! `cargo test -p mangler_core --release adjustment_perf -- --ignored --nocapture`.

#[cfg(test)]
mod adjustment_perf {
    use std::sync::Arc;
    use std::time::Instant;

    use crate::{
        float_image::FloatImage,
        get_id,
        input::{Input, InputSettings},
        operations::{operation_list, Operation, OperationListItem},
        value::Value,
    };

    /// 6000x4000 — a 24 MP frame, the size a camera raw develops to.
    const WIDTH: u32 = 6000;
    const HEIGHT: u32 = 4000;

    /// The operations this measures: everything under `adjustments/`, plus the
    /// two that were doing asymptotically more work than they needed to.
    const MEASURED: &[&str] = &[
        "clarity",
        "dehaze",
        "tone map",
        "hsl mixer",
        "color grade",
        "curves",
        "levels",
        "white balance",
        "selective color",
        "vibrance",
        "color balance",
        "tone equalizer",
        "photo filter",
        "exposure",
        "saturation",
        "negadoctor",
        "black and white",
        "shadows highlights",
        "color lookup",
        "threshold",
        "posterize",
        "vignette",
        "texture",
        "defringe",
    ];

    fn make_test_image() -> Arc<FloatImage> {
        let mut data = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                data.push((x % 256) as f32 / 255.0);
                data.push((y % 256) as f32 / 255.0);
                data.push(((x ^ y) % 256) as f32 / 255.0);
                data.push(1.0);
            }
        }
        Arc::new(FloatImage::from_raw(WIDTH, HEIGHT, 4, data).expect("data length matches"))
    }

    fn flatten(items: &[OperationListItem], out: &mut Vec<Operation>) {
        for item in items {
            match item {
                OperationListItem::Category { operation_list_items, .. } => {
                    flatten(operation_list_items, out)
                }
                OperationListItem::Operation { operation } => out.push(operation.clone()),
                OperationListItem::Subgraph => {}
            }
        }
    }

    #[tokio::test]
    #[ignore = "timing benchmark; run with --release --ignored --nocapture"]
    async fn adjustment_perf() {
        let mut ops = Vec::new();
        flatten(&operation_list(), &mut ops);
        let image = make_test_image();

        let mut rows: Vec<(String, f64)> = Vec::new();
        for op in &ops {
            let name = op.settings().name;
            if !MEASURED.contains(&name.as_str()) {
                continue;
            }
            let mut inputs = op.create_inputs();
            for input in inputs.iter_mut() {
                if matches!(input.value, Value::Image { .. }) {
                    input.value = Value::Image { data: Arc::clone(&image), change_id: get_id() };
                }
            }
            // Most of these nodes short-circuit on their neutral default (an
            // amount of 0 is the identity), which would time an early return
            // rather than the pass being measured. Input 1 is the main
            // strength knob on essentially every adjustment; push it 60% along
            // its range so the real work happens.
            if let Some(input) = inputs.get_mut(1) {
                if let Value::Decimal(_) = input.value {
                    let range = match &input.settings {
                        Some(InputSettings::Slider { range, .. }) => Some(*range),
                        Some(InputSettings::DragValue { clamp: Some(c), .. }) => Some(*c),
                        _ => None,
                    };
                    if let Some((lo, hi)) = range {
                        input.value = Value::Decimal(lo + 0.6 * (hi - lo));
                    }
                }
            }
            let start = Instant::now();
            if op.run(&mut inputs).await.is_err() {
                continue;
            }
            rows.push((name, start.elapsed().as_secs_f64() * 1000.0));
        }

        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        println!();
        println!("{}x{} ({:.1} MP)", WIDTH, HEIGHT, (WIDTH as f64 * HEIGHT as f64) / 1e6);
        println!(" {:<24}| {}", "Operation", "Time");
        println!("{}", "-".repeat(40));
        for (name, ms) in &rows {
            println!(" {:<24}| {:>8.1} ms", name, ms);
        }
        println!();
    }
}
