//! Golden-output harness: runs every operation with default inputs against a
//! fixed test image and prints one `name<TAB>hash` line per output.
//!
//! This is not an assertion test — it is a *diffing* tool for refactors that
//! must not change any node's pixels. Capture the table before a change and
//! after it, and `diff` the two:
//!
//! ```text
//! cargo test -p mangler_core all_operations_golden -- --ignored --nocapture > before.txt
//! # ...refactor...
//! cargo test -p mangler_core all_operations_golden -- --ignored --nocapture > after.txt
//! diff before.txt after.txt
//! ```
//!
//! `#[ignore]`d because it is a manual tool, not a gate: a handful of
//! operations are legitimately nondeterministic (clock- or filesystem-seeded),
//! and pinning platform-specific float results into the repo would fail on
//! other targets. The self-consistency check inside the harness reports which
//! operations are unstable so a diff can discount them.

#[cfg(test)]
mod all_operations_golden {
    use std::sync::Arc;

    use crate::{
        float_image::FloatImage,
        get_id,
        input::Input,
        operations::{operation_list, Operation, OperationListItem},
        value::Value,
    };

    /// Operations that need the filesystem, network, or OS clipboard.
    const SKIP_NAMES: &[&str] = &[
        "from file",
        "from url",
        "from clipboard",
        "image to file",
        "image to clipboard",
        "material",
        "text from clipboard",
        "from raw",
        "from folder",
    ];

    /// A 256x256 gradient with a non-trivial alpha ramp, so alpha-aware paths
    /// (premultiplied resampling, compositing) are exercised rather than
    /// running against a fully opaque image.
    fn make_test_image() -> Arc<FloatImage> {
        let mut data = Vec::with_capacity(256 * 256 * 4);
        for y in 0..256u32 {
            for x in 0..256u32 {
                data.push(x as f32 / 255.0);
                data.push(y as f32 / 255.0);
                data.push(((x ^ y) % 256) as f32 / 255.0);
                data.push(0.25 + 0.75 * (y as f32 / 255.0));
            }
        }
        Arc::new(FloatImage::from_raw(256, 256, 4, data).expect("data length matches"))
    }

    fn flatten_operations(items: &[OperationListItem]) -> Vec<Operation> {
        let mut ops = Vec::new();
        for item in items {
            match item {
                OperationListItem::Category { operation_list_items, .. } => {
                    ops.extend(flatten_operations(operation_list_items));
                }
                OperationListItem::Operation { operation } => ops.push(operation.clone()),
                OperationListItem::Subgraph => {}
            }
        }
        ops
    }

    fn prepare_inputs(inputs: &mut [Input], test_image: &Arc<FloatImage>) {
        for input in inputs.iter_mut() {
            if matches!(input.value, Value::Image { .. }) {
                let img_value = Value::Image {
                    data: Arc::clone(test_image),
                    change_id: get_id(),
                };
                input.value = img_value.clone();
                input.default_value = img_value;
            }
        }
    }

    /// Hash a value's *content* — for images, every pixel bit — so any change
    /// in output shows up. `change_id` is deliberately excluded: it is a fresh
    /// nanoid on every run and says nothing about the pixels.
    fn hash_value(value: &Value) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        std::mem::discriminant(value).hash(&mut h);
        match value {
            Value::Image { data, change_id: _ } => {
                data.width().hash(&mut h);
                data.height().hash(&mut h);
                data.channels().hash(&mut h);
                for v in data.as_raw() {
                    // Normalise the two zeros so a -0.0 vs 0.0 difference in an
                    // untouched pixel doesn't read as a behaviour change.
                    let bits = if *v == 0.0 { 0f32.to_bits() } else { v.to_bits() };
                    bits.hash(&mut h);
                }
            }
            Value::Decimal(v) => {
                let bits = if *v == 0.0 { 0f32.to_bits() } else { v.to_bits() };
                bits.hash(&mut h);
            }
            Value::Color(c) => {
                for v in [c.r, c.g, c.b, c.a] {
                    v.to_bits().hash(&mut h);
                }
            }
            Value::Curve(c) => {
                for p in &c.points {
                    p[0].to_bits().hash(&mut h);
                    p[1].to_bits().hash(&mut h);
                }
                for p in &c.handles {
                    p[0].to_bits().hash(&mut h);
                    p[1].to_bits().hash(&mut h);
                }
                c.closed.hash(&mut h);
                std::mem::discriminant(&c.interpolation).hash(&mut h);
            }
            // Everything else is small and already Hash-able via its Debug form.
            other => format!("{:?}", other).hash(&mut h),
        }
        h.finish()
    }

    /// Run one operation and return a `hash` per output, or an error string.
    async fn run_op(op: &Operation, test_image: &Arc<FloatImage>) -> Result<Vec<u64>, String> {
        let mut inputs = op.create_inputs();
        prepare_inputs(&mut inputs, test_image);
        match op.run(&mut inputs).await {
            Ok(response) => Ok(response
                .responses
                .iter()
                .map(|r| hash_value(&r.value))
                .collect()),
            Err(e) => Err(e.node_error.unwrap_or_else(|| {
                e.input_errors
                    .iter()
                    .map(|(i, m)| format!("input {}: {}", i, m))
                    .collect::<Vec<_>>()
                    .join("; ")
            })),
        }
    }

    #[tokio::test]
    #[ignore = "manual refactor tool; prints a table rather than asserting"]
    async fn all_operations_golden() {
        let all_ops = flatten_operations(&operation_list());
        let test_image = make_test_image();

        let mut rows: Vec<String> = Vec::new();
        let mut unstable: Vec<String> = Vec::new();

        for op in &all_ops {
            let name = op.settings().name;
            if SKIP_NAMES.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                continue;
            }

            let first = run_op(op, &test_image).await;
            // Run twice: an operation whose two runs disagree is seeded from
            // something outside its inputs, and its hash carries no signal.
            let second = run_op(op, &test_image).await;

            let row = match (&first, &second) {
                (Ok(a), Ok(b)) if a == b => a
                    .iter()
                    .map(|h| format!("{:016x}", h))
                    .collect::<Vec<_>>()
                    .join(","),
                (Ok(_), Ok(_)) => {
                    unstable.push(name.clone());
                    "UNSTABLE".to_string()
                }
                (Err(e), _) => format!("ERROR: {}", e),
                (_, Err(e)) => format!("ERROR: {}", e),
            };
            rows.push(format!("{}\t{}", name, row));
        }

        rows.sort();
        println!("--- BEGIN GOLDEN ---");
        for row in &rows {
            println!("{}", row);
        }
        println!("--- END GOLDEN ---");
        println!("operations hashed: {}", rows.len());
        if !unstable.is_empty() {
            println!("nondeterministic (excluded from comparison): {:?}", unstable);
        }
    }
}
