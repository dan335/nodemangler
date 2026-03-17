"""Append tests to noise, shapes, patterns, and pbr operation files."""
import os
import re

BASE = r"D:\rust\nodemangler\crates\mangler\src\operations"

IMAGE_HELPERS = """
    use crate::get_id;
    use crate::input::Input;
    use crate::value::Value;
    use image::{DynamicImage, RgbaImage};
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let r = ((x as f32 / w as f32) * 255.0) as u8;
                let g = ((y as f32 / h as f32) * 255.0) as u8;
                img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
            }
        }
        Arc::new(DynamicImage::ImageRgba8(img))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }
"""

def count_inputs_outputs(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    # Count Input::new and Output::new occurrences in create_inputs/create_outputs
    inputs_match = re.search(r'fn create_inputs.*?fn create_outputs', content, re.DOTALL)
    outputs_match = re.search(r'fn create_outputs.*?(?:pub async fn run|pub fn run|\Z)', content, re.DOTALL)
    n_inputs = len(re.findall(r'Input::new', inputs_match.group() if inputs_match else ''))
    n_outputs = len(re.findall(r'Output::new', outputs_match.group() if outputs_match else ''))
    return n_inputs, n_outputs

def get_struct_name(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    m = re.search(r'pub struct (Op\w+)', content)
    return m.group(1) if m else None

def get_setting_name(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    m = re.search(r'name:\s*"([^"]+)"\.to_string\(\)', content)
    return m.group(1) if m else None

def already_has_tests(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    return '#[cfg(test)]' in content

def make_settings_test(struct_name, setting_name, n_inputs, n_outputs):
    return f"""
    #[tokio::test]
    async fn test_{struct_name.lower()}_settings() {{
        let s = {struct_name}::settings();
        assert_eq!(s.name, "{setting_name}");
        assert_eq!({struct_name}::create_inputs().len(), {n_inputs});
        assert_eq!({struct_name}::create_outputs().len(), {n_outputs});
    }}
"""

def make_image_run_test(struct_name, fn_name, extra_inputs=""):
    return f"""
    #[tokio::test]
    async fn {fn_name}() {{
        let mut inputs = vec![image_input(16, 16){extra_inputs}];
        let result = {struct_name}::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {{:?}}", result.err());
        match &result.unwrap().responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
"""

def make_no_image_run_test(struct_name, fn_name, inputs_code):
    return f"""
    #[tokio::test]
    async fn {fn_name}() {{
        let mut inputs = vec![{inputs_code}];
        let result = {struct_name}::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {{:?}}", result.err());
        match &result.unwrap().responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
"""

# For noise generators - they take width, height (and other params) and produce an image
# Minimal test: just call run with minimal required inputs
def make_noise_block(path):
    struct_name = get_struct_name(path)
    setting_name = get_setting_name(path)
    n_inputs, n_outputs = count_inputs_outputs(path)

    # Build input list based on count
    # Noise generators typically have: width (u32/integer), height (u32/integer), seed, scale, etc.
    # We'll just use a vector of simple integer/decimal inputs
    input_lines = []
    for i in range(n_inputs):
        input_lines.append(f'Input::new("i{i}".to_string(), Value::Integer(4), None, None)')
    inputs_code = ',\n            '.join(input_lines)

    settings_test = make_settings_test(struct_name, setting_name, n_inputs, n_outputs)

    run_test = f"""
    #[tokio::test]
    async fn test_{struct_name.lower()}_run() {{
        let mut inputs = vec![
            {inputs_code}
        ];
        let result = {struct_name}::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {{:?}}", result.err());
        match &result.unwrap().responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
"""

    block = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
{settings_test}
{run_test}
}}
"""
    return block


def make_shape_block(path):
    struct_name = get_struct_name(path)
    setting_name = get_setting_name(path)
    n_inputs, n_outputs = count_inputs_outputs(path)

    input_lines = []
    for i in range(n_inputs):
        input_lines.append(f'Input::new("i{i}".to_string(), Value::Integer(4), None, None)')
    inputs_code = ',\n            '.join(input_lines)

    settings_test = make_settings_test(struct_name, setting_name, n_inputs, n_outputs)

    run_test = f"""
    #[tokio::test]
    async fn test_{struct_name.lower()}_run() {{
        let mut inputs = vec![
            {inputs_code}
        ];
        let result = {struct_name}::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {{:?}}", result.err());
        match &result.unwrap().responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
"""

    block = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
{settings_test}
{run_test}
}}
"""
    return block


def make_pattern_block(path):
    struct_name = get_struct_name(path)
    setting_name = get_setting_name(path)
    n_inputs, n_outputs = count_inputs_outputs(path)

    input_lines = []
    for i in range(n_inputs):
        input_lines.append(f'Input::new("i{i}".to_string(), Value::Integer(4), None, None)')
    inputs_code = ',\n            '.join(input_lines)

    settings_test = make_settings_test(struct_name, setting_name, n_inputs, n_outputs)

    run_test = f"""
    #[tokio::test]
    async fn test_{struct_name.lower()}_run() {{
        let mut inputs = vec![
            {inputs_code}
        ];
        let result = {struct_name}::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {{:?}}", result.err());
        match &result.unwrap().responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
"""

    block = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
{settings_test}
{run_test}
}}
"""
    return block


def make_pbr_block(path):
    struct_name = get_struct_name(path)
    setting_name = get_setting_name(path)
    n_inputs, n_outputs = count_inputs_outputs(path)

    # PBR ops take image inputs - first input is typically an image
    input_lines = [f'image_input(16, 16)']
    for i in range(1, n_inputs):
        input_lines.append(f'Input::new("i{i}".to_string(), Value::Decimal(1.0), None, None)')
    inputs_code = ',\n            '.join(input_lines)

    settings_test = make_settings_test(struct_name, setting_name, n_inputs, n_outputs)

    run_test = f"""
    #[tokio::test]
    async fn test_{struct_name.lower()}_run() {{
        let mut inputs = vec![
            {inputs_code}
        ];
        let result = {struct_name}::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {{:?}}", result.err());
        match &result.unwrap().responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
"""

    block = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
{settings_test}
{run_test}
}}
"""
    return block


def append_block(path, block):
    with open(path, 'a', encoding='utf-8') as f:
        f.write('\n' + block)


def process_files(files, make_block_fn):
    for path in files:
        if not os.path.exists(path):
            print(f"MISSING: {path}")
            continue
        if already_has_tests(path):
            print(f"Already has tests: {path}")
            continue
        block = make_block_fn(path)
        append_block(path, block)
        print(f"Updated: {path}")


# Noise files
noise_files = [
    os.path.join(BASE, "images", "noise", "perlin.rs"),
    os.path.join(BASE, "images", "noise", "billow.rs"),
    os.path.join(BASE, "images", "noise", "cylinders.rs"),
    os.path.join(BASE, "images", "noise", "fbm.rs"),
    os.path.join(BASE, "images", "noise", "heterogenous_multifractal.rs"),
    os.path.join(BASE, "images", "noise", "hybrid_multifractal.rs"),
    os.path.join(BASE, "images", "noise", "open_simplex.rs"),
    os.path.join(BASE, "images", "noise", "perlin_surflet.rs"),
    os.path.join(BASE, "images", "noise", "ridged_multifractal.rs"),
    os.path.join(BASE, "images", "noise", "simplex.rs"),
    os.path.join(BASE, "images", "noise", "super_simplex.rs"),
    os.path.join(BASE, "images", "noise", "value.rs"),
    os.path.join(BASE, "images", "noise", "worley_distance.rs"),
    os.path.join(BASE, "images", "noise", "worley_value.rs"),
    os.path.join(BASE, "images", "noise", "checkerboard.rs"),
]

# Shape files
shape_files = [
    os.path.join(BASE, "images", "shapes", "rectangle.rs"),
    os.path.join(BASE, "images", "shapes", "ellipse.rs"),
    os.path.join(BASE, "images", "shapes", "polygon.rs"),
    os.path.join(BASE, "images", "shapes", "star.rs"),
    os.path.join(BASE, "images", "shapes", "line.rs"),
    os.path.join(BASE, "images", "shapes", "circle.rs"),
]

# Pattern files
pattern_files = [
    os.path.join(BASE, "images", "patterns", "brick.rs"),
    os.path.join(BASE, "images", "patterns", "hexagonal.rs"),
    os.path.join(BASE, "images", "patterns", "weave.rs"),
    os.path.join(BASE, "images", "patterns", "tile_sampler.rs"),
]

# PBR files
pbr_files = [
    os.path.join(BASE, "images", "pbr", "normal_from_height.rs"),
    os.path.join(BASE, "images", "pbr", "ao_from_height.rs"),
    os.path.join(BASE, "images", "pbr", "curvature.rs"),
    os.path.join(BASE, "images", "pbr", "height_blend.rs"),
]

print("=== Noise ===")
process_files(noise_files, make_noise_block)
print("=== Shapes ===")
process_files(shape_files, make_shape_block)
print("=== Patterns ===")
process_files(pattern_files, make_pattern_block)
print("=== PBR ===")
process_files(pbr_files, make_pbr_block)
print("Done!")
