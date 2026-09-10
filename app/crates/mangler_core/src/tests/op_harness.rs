//! Shared harness for the three "run every operation" tables:
//! `all_operations_perf`, `all_operations_golden` and `adjustment_perf`.
//!
//! Each of those is a printing tool rather than an assertion, but they all need
//! the same three things: the operation menu flattened into a list, a synthetic
//! test image to feed every `Value::Image` input, and one error string out of an
//! `OperationError`. Those live here so a change lands in one place.
//!
//! The test image is *parameterised rather than unified*: the harnesses feed
//! deliberately different pixels (see `gradient_image` and friends), and
//! collapsing them would change what the golden hashes cover.

use std::sync::Arc;

use crate::{
    float_image::FloatImage,
    get_id,
    input::Input,
    operations::{Operation, OperationError, OperationListItem},
    value::Value,
};

/// Recursively flatten the operation menu tree into a list of operations,
/// in menu order.
pub fn flatten_operations(items: &[OperationListItem]) -> Vec<Operation> {
    flatten_with_path(items).into_iter().map(|(_, op)| op).collect()
}

/// Like [`flatten_operations`], but each operation is paired with its menu
/// category path (`"images/adjustments"`), so a caller can select a whole
/// menu subtree instead of hardcoding operation names.
pub fn flatten_with_path(items: &[OperationListItem]) -> Vec<(String, Operation)> {
    let mut out = Vec::new();
    walk(items, "", &mut out);
    out
}

fn walk(items: &[OperationListItem], path: &str, out: &mut Vec<(String, Operation)>) {
    for item in items {
        match item {
            OperationListItem::Category { name, operation_list_items } => {
                let child = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", path, name)
                };
                walk(operation_list_items, &child, out);
            }
            OperationListItem::Operation { operation } => {
                out.push((path.to_string(), operation.clone()));
            }
            OperationListItem::Subgraph => {}
        }
    }
}

/// A `width` x `height` gradient: red ramps across, green ramps down, blue is
/// `x ^ y`, fully opaque.
pub fn gradient_image(width: u32, height: u32) -> Arc<FloatImage> {
    build_image(width, height, Blue::Xor, Alpha::Opaque)
}

/// [`gradient_image`] with a non-trivial alpha ramp, so alpha-aware paths
/// (premultiplied resampling, compositing) are exercised rather than running
/// against a fully opaque image.
pub fn gradient_image_alpha_ramp(width: u32, height: u32) -> Arc<FloatImage> {
    build_image(width, height, Blue::Xor, Alpha::Ramp)
}

/// [`gradient_image`] with a flat mid-grey blue channel instead of `x ^ y`.
pub fn gradient_image_flat_blue(width: u32, height: u32) -> Arc<FloatImage> {
    build_image(width, height, Blue::Flat, Alpha::Opaque)
}

enum Blue {
    /// `((x ^ y) % 256) / 255`
    Xor,
    /// A constant `128 / 255`.
    Flat,
}

enum Alpha {
    Opaque,
    /// `0.25 + 0.75 * (y % 256) / 255`
    Ramp,
}

fn build_image(width: u32, height: u32, blue: Blue, alpha: Alpha) -> Arc<FloatImage> {
    let mut data = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for y in 0..height {
        for x in 0..width {
            let yf = (y % 256) as f32 / 255.0;
            data.push((x % 256) as f32 / 255.0);
            data.push(yf);
            data.push(match blue {
                Blue::Xor => ((x ^ y) % 256) as f32 / 255.0,
                Blue::Flat => 128.0 / 255.0,
            });
            data.push(match alpha {
                Alpha::Opaque => 1.0,
                Alpha::Ramp => 0.25 + 0.75 * yf,
            });
        }
    }
    Arc::new(FloatImage::from_raw(width, height, 4, data).expect("data length matches"))
}

/// Replace every `Value::Image` input with the given test image.
pub fn prepare_inputs(inputs: &mut [Input], test_image: &Arc<FloatImage>) {
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

/// Flatten an `OperationError` into one printable line: the node error if there
/// is one, otherwise every input error joined together.
pub fn format_run_error(e: &OperationError) -> String {
    e.node_error.clone().unwrap_or_else(|| {
        e.input_errors
            .iter()
            .map(|(i, m)| format!("input {}: {}", i, m))
            .collect::<Vec<_>>()
            .join("; ")
    })
}
