//! Benchmark that runs every operation with default inputs and prints a sorted timing table.
//!
//! Run with: `cd app && cargo test -p mangler_core all_operations_perf -- --nocapture`

#[cfg(test)]
mod all_operations_perf {
    use std::sync::Arc;
    use std::time::Duration;

    use image::{DynamicImage, RgbaImage};

    use crate::{
        get_id,
        input::Input,
        operations::{operation_list, Operation, OperationListItem},
        value::Value,
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

    /// Create a 512x512 gradient test image.
    fn make_test_image() -> Arc<DynamicImage> {
        let img = RgbaImage::from_fn(512, 512, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        });
        Arc::new(DynamicImage::ImageRgba8(img))
    }

    /// Recursively flatten the operation menu tree into a list of operations.
    fn flatten_operations(items: &[OperationListItem]) -> Vec<Operation> {
        let mut ops = Vec::new();
        for item in items {
            match item {
                OperationListItem::Category { operation_list_items, .. } => {
                    ops.extend(flatten_operations(operation_list_items));
                }
                OperationListItem::Operation { operation } => {
                    ops.push(operation.clone());
                }
                OperationListItem::Subgraph => {}
            }
        }
        ops
    }

    /// Replace any DynamicImage inputs with a 512x512 test image.
    fn prepare_inputs(inputs: &mut [Input], test_image: &Arc<DynamicImage>) {
        for input in inputs.iter_mut() {
            if matches!(input.value, Value::DynamicImage { .. }) {
                let img_value = Value::DynamicImage {
                    data: Arc::clone(test_image),
                    change_id: get_id(),
                };
                input.value = img_value.clone();
                input.default_value = img_value;
            }
        }
    }

    enum RunResult {
        Ok { time: Duration },
        Err { message: String },
        Skipped,
    }

    #[tokio::test]
    async fn all_operations_perf() {
        let list = operation_list();
        let all_ops = flatten_operations(&list);
        let test_image = make_test_image();

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
                    let msg = e
                        .node_error
                        .unwrap_or_else(|| {
                            e.input_errors
                                .iter()
                                .map(|(i, m)| format!("input {}: {}", i, m))
                                .collect::<Vec<_>>()
                                .join("; ")
                        });
                    results.push((name, RunResult::Err { message: msg }));
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
        println!(
            " {:<4}| {:<40}| {}",
            "#", "Operation", "Time"
        );
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
