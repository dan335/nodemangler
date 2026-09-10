use super::{default_image, operation_list, Operation, OperationListItem};

#[test]
fn test_operation_list_not_empty() {
    let list = operation_list();
    assert!(!list.is_empty());
}

#[test]
fn test_default_image() {
    let img = default_image();
    assert_eq!(img.width(), 1);
    assert_eq!(img.height(), 1);
}

#[test]
fn test_all_operations_have_valid_settings() {
    fn check_items(items: &[OperationListItem]) {
        for item in items {
            match item {
                OperationListItem::Category { name, operation_list_items } => {
                    assert!(!name.is_empty());
                    check_items(operation_list_items);
                }
                OperationListItem::Operation { operation } => {
                    let settings = operation.settings();
                    assert!(!settings.name.is_empty());
                    let _inputs = operation.create_inputs();
                    let _outputs = operation.create_outputs();
                }
                OperationListItem::Subgraph => {}
            }
        }
    }
    check_items(&operation_list());
}

/// Every registered operation must be reachable from the add-node menu / search
/// -- both are driven by `operation_list()` in the GUI (`menu_panel.rs`,
/// `node_search_popup.rs`). A node registered in the `operations!` macro but
/// left out of `operation_list()` compiles and unit-tests fine, passes both
/// README gates (they read `operation_list()` too), and yet can never be placed
/// in a graph.
///
/// This checks the whole registry rather than a frozen list of names, so a new
/// operation is covered the moment it is registered.
#[test]
fn test_added_nodes_are_reachable_in_menu() {
    fn collect_names(items: &[OperationListItem], out: &mut std::collections::HashSet<String>) {
        for item in items {
            match item {
                OperationListItem::Category { operation_list_items, .. } => {
                    collect_names(operation_list_items, out)
                }
                OperationListItem::Operation { operation } => {
                    out.insert(operation.settings().name);
                }
                OperationListItem::Subgraph => {}
            }
        }
    }

    let mut names = std::collections::HashSet::new();
    collect_names(&operation_list(), &mut names);

    let missing: Vec<String> = Operation::all_variants()
        .into_iter()
        .map(|op| op.settings().name)
        .filter(|name| !names.contains(name))
        .collect();

    assert!(
        missing.is_empty(),
        "these operations are registered but missing from the node menu \
         (operation_list), so they can never be placed in a graph: {missing:?}"
    );
}

/// Collapses runs of non-alphanumeric characters to a single space and
/// lowercases, so e.g. "Non-Uniform Blur" and "non-uniform blur" compare equal.
fn normalize(s: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    out.trim_end().to_string()
}

fn collect_operation_names(items: &[OperationListItem], out: &mut Vec<String>) {
    for item in items {
        match item {
            OperationListItem::Category { operation_list_items, .. } => {
                collect_operation_names(operation_list_items, out)
            }
            OperationListItem::Operation { operation } => out.push(operation.settings().name),
            OperationListItem::Subgraph => {}
        }
    }
}

/// Reads the top-level README.md and returns the text of its "## Node
/// Reference" section (up to, but not including, the next "## " heading).
fn read_node_reference_section() -> String {
    let readme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../README.md");
    let readme = std::fs::read_to_string(readme_path)
        .unwrap_or_else(|e| panic!("failed to read top-level README.md at {readme_path}: {e}"));

    let start = readme
        .find("## Node Reference")
        .expect("README.md is missing a '## Node Reference' section");
    let end = readme[start..]
        .find("\n## ")
        .map(|i| start + i)
        .unwrap_or(readme.len());
    readme[start..end].to_string()
}

/// Parses the `- **Subcategory:** Name1, Name2, ...` bullet lines in the
/// "Node Reference" section into individual node display names.
fn parse_documented_node_names(section: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with("- **") {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, "**").collect();
        let Some(list_part) = parts.get(2) else { continue };
        for name in list_part.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// The top-level README advertises a full "Node Reference" listing every node
/// by name. This pins that list to the actual `operation_list()` registry so
/// newly added operations can't be silently left out of the docs.
#[test]
fn test_all_operations_documented_in_readme() {
    let section = read_node_reference_section();
    let section_normalized = normalize(&section);

    let mut names = Vec::new();
    collect_operation_names(&operation_list(), &mut names);

    let missing: Vec<String> = names
        .into_iter()
        .filter(|name| !section_normalized.contains(&normalize(name)))
        .collect();

    assert!(
        missing.is_empty(),
        "operations missing from the README.md Node Reference section: {missing:?}"
    );
}

/// The inverse of `test_all_operations_documented_in_readme`: catches nodes
/// listed in the README that no longer exist in `operation_list()` — e.g. an
/// operation was renamed or deleted but the README entry was never updated.
#[test]
fn test_no_stale_operations_in_readme() {
    let section = read_node_reference_section();
    let documented = parse_documented_node_names(&section);

    let mut real_names = std::collections::HashSet::new();
    let mut names = Vec::new();
    collect_operation_names(&operation_list(), &mut names);
    for name in names {
        real_names.insert(normalize(&name));
    }

    let extra: Vec<String> = documented
        .into_iter()
        .filter(|name| !real_names.contains(&normalize(name)))
        .collect();

    assert!(
        extra.is_empty(),
        "README.md Node Reference lists nodes that no longer exist in operation_list(): {extra:?}"
    );
}

/// Diagnostic: run every operation with its default inputs and report any that
/// panic (e.g. an input-index-out-of-bounds from a `create_inputs()`/`run()`
/// mismatch). Panics inside a spawned tokio task surface as a JoinError.
#[test]
fn test_no_operation_panics_on_default_inputs() {
    use crate::operations::Operation;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut panicked = Vec::new();
    for op in Operation::all_variants() {
        let name = format!("{:?}", op);
        let mut inputs = op.create_inputs();
        let res = rt.block_on(async move {
            tokio::spawn(async move { op.run(&mut inputs).await }).await
        });
        if let Err(join) = res {
            if join.is_panic() {
                panicked.push(name);
            }
        }
    }
    assert!(panicked.is_empty(), "operations panicked on default inputs: {panicked:?}");
}

// ---------------------------------------------------------------------------
// convert_inputs! — the macro that replaced ~1,500 hand-written
// convert-then-destructure pairs across the operation library.
// ---------------------------------------------------------------------------

mod convert_inputs_macro {
    use crate::float_image::FloatImage;
    use crate::input::Input;
    use crate::operations::{OperationError, OperationResponse, OutputResponse};
    use crate::value::Value;
    use crate::get_id;
    use std::sync::Arc;
    use std::time::Duration;

    fn input(name: &str, value: Value) -> Input {
        Input::new(name.to_string(), value, None, None)
    }

    fn ok(values: Vec<Value>) -> Result<OperationResponse, OperationError> {
        Ok(OperationResponse {
            time: Duration::ZERO,
            responses: values.into_iter().map(|value| OutputResponse { value }).collect(),
        })
    }

    /// A stand-in operation with the shape the macro is built for.
    fn run_typical(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(amount) = 1,
            Integer(mut count) = 2,
            Bool(flag) = 3,
        }
        count += 1;
        ok(vec![
            Value::Decimal(data.width() as f32 + amount + count as f32 + flag as i32 as f32),
        ])
    }

    #[test]
    fn unpacks_each_variant_into_its_inner_value() {
        let image = Arc::new(FloatImage::new(7, 3, 4));
        let mut inputs = vec![
            input("image", Value::Image { data: image, change_id: get_id() }),
            input("amount", Value::Decimal(0.5)),
            input("count", Value::Integer(10)),
            input("flag", Value::Bool(true)),
        ];
        let result = run_typical(&mut inputs).expect("all inputs convert");
        // 7 (width) + 0.5 (amount) + 11 (count, incremented) + 1 (flag)
        let Value::Decimal(v) = result.responses[0].value else { panic!() };
        assert_eq!(v, 19.5);
    }

    /// `mut` in the binding is what the longhand `let Value::Integer(mut n)`
    /// gave, and a dozen shape and simulation nodes rely on it. If the macro
    /// stopped accepting patterns this would not compile.
    #[test]
    fn bindings_may_be_mutable() {
        let mut inputs = vec![
            input("image", Value::Image { data: Arc::new(FloatImage::new(1, 1, 1)), change_id: get_id() }),
            input("amount", Value::Decimal(0.0)),
            input("count", Value::Integer(0)),
            input("flag", Value::Bool(false)),
        ];
        let result = run_typical(&mut inputs).expect("all inputs convert");
        let Value::Decimal(v) = result.responses[0].value else { panic!() };
        assert_eq!(v, 2.0, "count should have been incremented in place");
    }

    /// Conversions that the value system allows still happen: an Integer input
    /// asked for as a Decimal arrives as one. The macro must not have narrowed
    /// this to an exact-variant match.
    #[test]
    fn applies_the_value_systems_conversions() {
        let mut inputs = vec![
            input("image", Value::Image { data: Arc::new(FloatImage::new(2, 2, 1)), change_id: get_id() }),
            // Integer where a Decimal is wanted.
            input("amount", Value::Integer(3)),
            // Decimal where an Integer is wanted.
            input("count", Value::Decimal(4.0)),
            input("flag", Value::Bool(false)),
        ];
        let result = run_typical(&mut inputs).expect("Integer/Decimal are interconvertible");
        let Value::Decimal(v) = result.responses[0].value else { panic!() };
        assert_eq!(v, 2.0 + 3.0 + 5.0);
    }

    /// The load-bearing property of the longhand this replaced: every input is
    /// *attempted* before the first failure returns, so a node with two bad
    /// inputs highlights both in the UI rather than one at a time. A macro that
    /// returned at the first failure would silently degrade that.
    #[test]
    fn reports_every_failing_input_not_just_the_first() {
        fn run_two_texts(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
            convert_inputs! { inputs;
                Decimal(a) = 0,
                Decimal(b) = 1,
            }
            ok(vec![Value::Decimal(a + b)])
        }

        let mut inputs = vec![
            input("a", Value::Text("not a number".to_string())),
            input("b", Value::Text("also not a number".to_string())),
        ];
        let error = run_two_texts(&mut inputs).expect_err("neither input converts");
        let indices: Vec<usize> = error.input_errors.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1], "both failing inputs should be reported");
        assert!(error.node_error.is_none());
    }

    /// An index past the end of the slice is recorded as an input error rather
    /// than panicking — the safety net `convert_input` documents, which the
    /// macro must not have bypassed.
    #[test]
    fn a_missing_input_is_an_error_not_a_panic() {
        fn run_three(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
            convert_inputs! { inputs;
                Decimal(a) = 0,
                Decimal(b) = 1,
                Decimal(c) = 2,
            }
            ok(vec![Value::Decimal(a + b + c)])
        }

        let mut inputs = vec![input("a", Value::Decimal(1.0))];
        let error = run_three(&mut inputs).expect_err("inputs 1 and 2 are missing");
        let indices: Vec<usize> = error.input_errors.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![1, 2]);
    }

    /// Indices are honoured as written, not inferred from the entry order.
    #[test]
    fn entries_may_reference_inputs_out_of_order() {
        fn run_reversed(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
            convert_inputs! { inputs;
                Decimal(second) = 1,
                Decimal(first) = 0,
            }
            ok(vec![Value::Decimal(second * 10.0 + first)])
        }

        let mut inputs = vec![input("a", Value::Decimal(3.0)), input("b", Value::Decimal(4.0))];
        let result = run_reversed(&mut inputs).unwrap();
        let Value::Decimal(v) = result.responses[0].value else { panic!() };
        assert_eq!(v, 43.0);
    }

    /// The `Image` arm discards `change_id` and hands back the `Arc` itself —
    /// the same `Arc`, not a copy of the pixels.
    #[test]
    fn image_unpacks_to_the_shared_arc() {
        fn run_image(inputs: &mut [Input]) -> Arc<FloatImage> {
            fn inner(inputs: &mut [Input]) -> Result<Arc<FloatImage>, OperationError> {
                convert_inputs! { inputs; Image(data) = 0 }
                Ok(data)
            }
            inner(inputs).unwrap()
        }

        let original = Arc::new(FloatImage::new(4, 4, 3));
        let mut inputs = vec![input(
            "image",
            Value::Image { data: Arc::clone(&original), change_id: get_id() },
        )];
        let unpacked = run_image(&mut inputs);
        assert!(Arc::ptr_eq(&original, &unpacked), "the image should not be copied");
    }
}
