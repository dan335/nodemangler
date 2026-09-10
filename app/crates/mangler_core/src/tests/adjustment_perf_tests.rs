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
    use std::time::Instant;

    use crate::{input::InputSettings, operations::operation_list, value::Value};
    use crate::tests::op_harness::{flatten_with_path, gradient_image, prepare_inputs};

    /// 6000x4000 — a 24 MP frame, the size a camera raw develops to.
    const WIDTH: u32 = 6000;
    const HEIGHT: u32 = 4000;

    /// The menu category whose every operation is measured. Selecting by
    /// category rather than by name means a new adjustment node joins the table
    /// on its own, and a renamed one cannot silently drop out of it.
    const MEASURED_CATEGORY: &str = "adjustments";

    /// Operations measured even though they live outside `adjustments/` —
    /// nodes that were doing asymptotically more work than they needed to and
    /// are worth watching at full resolution. Every name here must resolve to a
    /// real operation; `adjustment_perf` asserts it, so a rename fails loudly
    /// instead of shrinking the table.
    const EXTRA_OPERATIONS: &[&str] = &[];

    #[tokio::test]
    #[ignore = "timing benchmark; run with --release --ignored --nocapture"]
    async fn adjustment_perf() {
        let ops = flatten_with_path(&operation_list());
        let mut unmatched: Vec<&str> = EXTRA_OPERATIONS.to_vec();
        let mut measured = Vec::new();
        for (path, op) in &ops {
            let name = op.settings().name;
            let in_category = path.split('/').any(|segment| segment == MEASURED_CATEGORY);
            let extra = EXTRA_OPERATIONS.contains(&name.as_str());
            if extra {
                unmatched.retain(|n| *n != name.as_str());
            }
            if in_category || extra {
                measured.push((name, op.clone()));
            }
        }
        assert!(
            unmatched.is_empty(),
            "EXTRA_OPERATIONS names that match no operation: {:?}",
            unmatched
        );
        assert!(
            !measured.is_empty(),
            "no operations found in the '{}' menu category — has it been renamed?",
            MEASURED_CATEGORY
        );

        let image = gradient_image(WIDTH, HEIGHT);

        let mut rows: Vec<(String, f64)> = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        for (name, op) in &measured {
            let mut inputs = op.create_inputs();
            prepare_inputs(&mut inputs, &image);
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
                failed.push(name.clone());
                continue;
            }
            rows.push((name.clone(), start.elapsed().as_secs_f64() * 1000.0));
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
        println!("{} operations measured", rows.len());
        if !failed.is_empty() {
            println!("did not run (errored with default inputs): {:?}", failed);
        }
        println!();
    }
}
