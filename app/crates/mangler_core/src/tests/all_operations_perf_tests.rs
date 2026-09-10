//! Benchmark that runs every operation with default inputs and prints a sorted timing table.
//!
//! Run with: `cd app && cargo test -p mangler_core all_operations_perf -- --nocapture`

#[cfg(test)]
mod all_operations_perf {
    use std::time::Duration;

    use crate::operations::operation_list;
    use crate::tests::op_harness::{
        flatten_operations, format_run_error, gradient_image_flat_blue, prepare_inputs,
    };

    /// Names of operations to skip (need filesystem, network, or clipboard).
    const SKIP_NAMES: &[&str] = &[
        "from file",
        "from url",
        "from clipboard",
        "image to file",
        "image to clipboard",
        "text from clipboard",
    ];

    enum RunResult {
        Ok { time: Duration },
        Err { message: String },
        Skipped,
    }

    #[tokio::test]
    async fn all_operations_perf() {
        let list = operation_list();
        let all_ops = flatten_operations(&list);
        // 512x512 with a flat blue channel: the original benchmark image, kept
        // so timings stay comparable with previously captured tables.
        let test_image = gradient_image_flat_blue(512, 512);

        let mut results: Vec<(String, RunResult)> = Vec::new();

        for op in &all_ops {
            let name = op.settings().name;

            if SKIP_NAMES.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                results.push((name, RunResult::Skipped));
                continue;
            }

            let mut inputs = op.create_inputs();
            prepare_inputs(&mut inputs, &test_image);

            match op.run(&mut inputs).await {
                Ok(response) => {
                    results.push((name, RunResult::Ok { time: response.time }));
                }
                Err(e) => {
                    results.push((name, RunResult::Err { message: format_run_error(&e) }));
                }
            }
        }

        // Partition results
        let mut ok_results: Vec<(&str, Duration)> = Vec::new();
        let mut skipped: Vec<&str> = Vec::new();
        let mut errors: Vec<(&str, &str)> = Vec::new();

        for (name, result) in &results {
            match result {
                RunResult::Ok { time } => ok_results.push((name, *time)),
                RunResult::Skipped => skipped.push(name),
                RunResult::Err { message } => errors.push((name, message)),
            }
        }

        // Sort slowest first
        ok_results.sort_by(|a, b| b.1.cmp(&a.1));

        // Print table
        println!();
        // Header matches the ` {:<4}| {:<40}| {}` row format below.
        println!(" #   | Operation                               | Time");
        println!("{}", "-".repeat(62));

        for (i, (name, time)) in ok_results.iter().enumerate() {
            let time_str = if time.as_millis() > 0 {
                format!("{:.2}ms", time.as_secs_f64() * 1000.0)
            } else {
                format!("{:.0}us", time.as_micros())
            };
            println!(" {:<4}| {:<40}| {}", i + 1, name, time_str);
        }

        if !skipped.is_empty() {
            println!();
            println!("SKIPPED ({}):", skipped.len());
            for name in &skipped {
                println!("  - {}", name);
            }
        }

        if !errors.is_empty() {
            println!();
            println!("ERRORS ({}):", errors.len());
            for (name, msg) in &errors {
                println!("  - {}: {}", name, msg);
            }
        }

        let total = ok_results.len() + skipped.len() + errors.len();
        println!();
        println!(
            "Total: {} operations | {} OK | {} SKIPPED | {} ERRORS",
            total,
            ok_results.len(),
            skipped.len(),
            errors.len()
        );
        println!();
    }
}
