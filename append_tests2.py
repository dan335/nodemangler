import os

base = "D:/rust/nodemangler/crates/mangler/src/operations"

files_and_tests = {}

# Color inputs
for cs in ["srgb", "hsl", "hsv", "lab", "lch", "rgb_linear", "xyz", "yuv"]:
    struct_map = {
        "srgb": ("OpColorInputRgba", "rgb", 4, 4, 4),
        "hsl": ("OpColorInputHsla", "hsl", 4, 4, 4),
        "hsv": ("OpColorInputHsva", "hsv", 4, 4, 4),
        "lab": ("OpColorInputLab", "lab", 4, 4, 4),
        "lch": ("OpColorInputLch", "lch", 4, 4, 4),
        "rgb_linear": ("OpColorInputRgbaLinear", "rgb linear", 4, 4, 4),
        "xyz": ("OpColorInputXyz", "xyz", 4, 4, 4),
        "yuv": ("OpColorInputYuv", "yuv", 4, 4, 4),
    }
    struct_name, node_name, nin, nout, num_vals = struct_map[cs]
    if cs == "srgb":
        val_str = "[1.0, 0.0, 0.0, 1.0]"
    elif cs == "hsl":
        val_str = "[180.0, 1.0, 0.5, 1.0]"
    elif cs == "hsv":
        val_str = "[120.0, 1.0, 1.0, 1.0]"
    elif cs == "lab":
        val_str = "[50.0, 20.0, -30.0, 1.0]"
    elif cs == "lch":
        val_str = "[0.6, 0.5, 180.0, 1.0]"
    elif cs == "rgb_linear":
        val_str = "[0.5, 0.5, 0.5, 1.0]"
    elif cs == "xyz":
        val_str = "[0.5, 0.2, 0.1, 1.0]"
    elif cs == "yuv":
        val_str = "[0.5, 0.3, 0.2, 1.0]"

    files_and_tests[f"colors/inputs/{cs}.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn decimal_inputs(vals: &[f32]) -> Vec<Input> {{
        vals.iter()
            .enumerate()
            .map(|(i, v)| Input::new(format!("v{{}}",  i), Value::Decimal(*v), None, None))
            .collect()
    }}

    #[tokio::test]
    async fn test_{cs}_input() {{
        let mut inputs = decimal_inputs(&{val_str});
        let result = {struct_name}::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::Color(_) => {{}}
            other => panic!("Expected Color, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_{cs}_settings() {{
        let s = {struct_name}::settings();
        assert_eq!(s.name, "{node_name}");
        assert_eq!({struct_name}::create_inputs().len(), {nin});
        assert_eq!({struct_name}::create_outputs().len(), {nout - 3});
    }}
}}
"""

# cmyk is special (5 inputs)
files_and_tests["colors/inputs/cmyk.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
        vals.iter()
            .enumerate()
            .map(|(i, v)| Input::new(format!("v{}", i), Value::Decimal(*v), None, None))
            .collect()
    }

    #[tokio::test]
    async fn test_cmyk_input() {
        let mut inputs = decimal_inputs(&[0.0, 1.0, 1.0, 0.0, 1.0]);
        let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cmyk_settings() {
        let s = OpColorInputCmyk::settings();
        assert_eq!(s.name, "cmyk");
        assert_eq!(OpColorInputCmyk::create_inputs().len(), 5);
        assert_eq!(OpColorInputCmyk::create_outputs().len(), 1);
    }
}
"""

# Color outputs
files_and_tests["colors/outputs/to_cmyk.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new(
            "input".to_string(),
            Value::Color(Color::from_srgb_float(r, g, b, a)),
            None, None,
        )]
    }

    #[tokio::test]
    async fn test_to_cmyk() {
        let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 5);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_cmyk_settings() {
        let s = OpColorOutputCmyk::settings();
        assert_eq!(s.name, "to cmyk");
        assert_eq!(OpColorOutputCmyk::create_inputs().len(), 1);
        assert_eq!(OpColorOutputCmyk::create_outputs().len(), 5);
    }
}
"""

files_and_tests["colors/outputs/to_hsl.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_hsl() {
        let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_hsl_settings() {
        let s = OpColorOutputHsl::settings();
        assert_eq!(s.name, "to hsl");
        assert_eq!(OpColorOutputHsl::create_inputs().len(), 1);
        assert_eq!(OpColorOutputHsl::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_hsv.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_hsv() {
        let mut inputs = color_input(0.0, 1.0, 0.0, 1.0);
        let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 120.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_hsv_settings() {
        let s = OpColorOutputHsv::settings();
        assert_eq!(s.name, "to hsv");
        assert_eq!(OpColorOutputHsv::create_inputs().len(), 1);
        assert_eq!(OpColorOutputHsv::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_lab.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_lab() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputLab::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
    }

    #[tokio::test]
    async fn test_to_lab_settings() {
        let s = OpColorOutputLab::settings();
        assert_eq!(s.name, "to lab");
        assert_eq!(OpColorOutputLab::create_inputs().len(), 1);
        assert_eq!(OpColorOutputLab::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_lch.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_lch() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
    }

    #[tokio::test]
    async fn test_to_lch_settings() {
        let s = OpColorOutputLch::settings();
        assert_eq!(s.name, "to lch");
        assert_eq!(OpColorOutputLch::create_inputs().len(), 1);
        assert_eq!(OpColorOutputLch::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_rgb_linear.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_rgb_linear() {
        let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[3].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_rgb_linear_settings() {
        let s = OpColorOutputRgbLinear::settings();
        assert_eq!(s.name, "to rgb linear");
        assert_eq!(OpColorOutputRgbLinear::create_inputs().len(), 1);
        assert_eq!(OpColorOutputRgbLinear::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_srgb.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_srgb() {
        let mut inputs = color_input(0.8, 0.2, 0.4, 0.5);
        let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.8).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_srgb_settings() {
        let s = OpColorOutputRgb::settings();
        assert_eq!(s.name, "to rgb");
        assert_eq!(OpColorOutputRgb::create_inputs().len(), 1);
        assert_eq!(OpColorOutputRgb::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_xyz.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_xyz() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
    }

    #[tokio::test]
    async fn test_to_xyz_settings() {
        let s = OpColorOutputXyz::settings();
        assert_eq!(s.name, "to xyz");
        assert_eq!(OpColorOutputXyz::create_inputs().len(), 1);
        assert_eq!(OpColorOutputXyz::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/outputs/to_yuv.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_yuv() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputYuv::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
    }

    #[tokio::test]
    async fn test_to_yuv_settings() {
        let s = OpColorOutputYuv::settings();
        assert_eq!(s.name, "to yuv");
        assert_eq!(OpColorOutputYuv::create_inputs().len(), 1);
        assert_eq!(OpColorOutputYuv::create_outputs().len(), 4);
    }
}
"""

files_and_tests["colors/blend/lerp.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::color::color_spaces::ColorSpace;
    use crate::input::Input;
    use crate::value::Value;

    fn blend_inputs(color_space: ColorSpace, amount: f32) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
            Input::new("amount".to_string(), Value::Decimal(amount), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(color_space), None, None),
        ]
    }

    #[tokio::test]
    async fn test_blend_srgb() {
        let mut inputs = blend_inputs(ColorSpace::Srgb, 0.5);
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_amount_zero() {
        let mut inputs = blend_inputs(ColorSpace::Srgb, 0.0);
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_amount_one() {
        let mut inputs = blend_inputs(ColorSpace::Srgb, 1.0);
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_settings() {
        let s = OpColorBlendLerp::settings();
        assert_eq!(s.name, "blend");
        assert_eq!(OpColorBlendLerp::create_inputs().len(), 4);
        assert_eq!(OpColorBlendLerp::create_outputs().len(), 1);
    }
}
"""

files_and_tests["colors/sample_image/most_common_colors.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_id;
    use crate::input::Input;
    use crate::value::Value;
    use image::DynamicImage;
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Value {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            *pixel = image::Rgba([r, g, 128, 255]);
        }
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }
    }

    #[tokio::test]
    async fn test_most_common_colors() {
        let mut inputs = vec![
            Input::new("image".to_string(), test_image(4, 4), None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
        assert!(result.responses.len() <= 5);
        for resp in &result.responses {
            match &resp.value {
                Value::Color(_) => {}
                other => panic!("Expected Color, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_most_common_colors_settings() {
        let s = OpColorSampleMostCommonColors::settings();
        assert_eq!(s.name, "most common colors");
        assert_eq!(OpColorSampleMostCommonColors::create_inputs().len(), 4);
        assert_eq!(OpColorSampleMostCommonColors::create_outputs().len(), 5);
    }
}
"""

for rel_path, test_block in files_and_tests.items():
    full_path = os.path.join(base, rel_path)
    with open(full_path, 'a') as f:
        f.write(test_block)
    print(f"Updated: {rel_path}")

print("Done!")
