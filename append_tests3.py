import os

base = "D:/rust/nodemangler/crates/mangler/src/operations"

# Shared helper for image tests
IMAGE_HELPERS = """
    use crate::get_id;
    use crate::input::Input;
    use crate::value::Value;
    use image::DynamicImage;
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            *pixel = image::Rgba([r, g, 128, 255]);
        }
        Arc::new(DynamicImage::ImageRgba8(imgbuf))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }
"""

files_and_tests = {}

files_and_tests["images/adjustments/blur.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_blur() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_blur_settings() {{
        let s = OpImageAdjustmentBlur::settings();
        assert_eq!(s.name, "blur");
        assert_eq!(OpImageAdjustmentBlur::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentBlur::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/contrast.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_contrast() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("amount".to_string(), Value::Decimal(1.5), None, None),
        ];
        let result = OpImageAdjustmentContrast::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_contrast_settings() {{
        let s = OpImageAdjustmentContrast::settings();
        assert_eq!(s.name, "contrast");
        assert_eq!(OpImageAdjustmentContrast::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentContrast::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/grayscale.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_grayscale() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageAdjustmentGrayscale::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_grayscale_settings() {{
        let s = OpImageAdjustmentGrayscale::settings();
        assert_eq!(s.name, "grayscale");
        assert_eq!(OpImageAdjustmentGrayscale::create_inputs().len(), 1);
        assert_eq!(OpImageAdjustmentGrayscale::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/invert.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_invert() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageAdjustmentInvert::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_invert_settings() {{
        let s = OpImageAdjustmentInvert::settings();
        assert_eq!(s.name, "invert");
        assert_eq!(OpImageAdjustmentInvert::create_inputs().len(), 1);
        assert_eq!(OpImageAdjustmentInvert::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/brighten.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_brighten() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_brighten_settings() {{
        let s = OpImageAdjustmentBrighten::settings();
        assert_eq!(s.name, "brighten");
        assert_eq!(OpImageAdjustmentBrighten::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentBrighten::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/hue_rotate.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_hue_rotate() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentHueRotate::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_hue_rotate_settings() {{
        let s = OpImageAdjustmentHueRotate::settings();
        assert_eq!(s.name, "hue rotate");
        assert_eq!(OpImageAdjustmentHueRotate::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentHueRotate::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/unsharpen.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_unsharpen() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
            Input::new("threshold".to_string(), Value::Integer(1), None, None),
        ];
        let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_unsharpen_settings() {{
        let s = OpImageAdjustmentUnsharpen::settings();
        assert_eq!(s.name, "unsharpen");
        assert_eq!(OpImageAdjustmentUnsharpen::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentUnsharpen::create_outputs().len(), 1);
    }}
}}
"""

files_and_tests["images/adjustments/levels.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_levels_settings() {{
        let s = OpImageAdjustmentLevels::settings();
        assert_eq!(s.name, "levels");
        assert_eq!(OpImageAdjustmentLevels::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentLevels::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_levels_identity() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("black point".to_string(), Value::Decimal(0.0), None, None),
            Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
            Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_levels_crush_blacks() {{
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([64, 64, 64, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: img, change_id: get_id() }}, None, None),
            Input::new("black point".to_string(), Value::Decimal(0.5), None, None),
            Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
            Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p[0], 0);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/curves.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_curves_settings() {{
        let s = OpImageAdjustmentCurves::settings();
        assert_eq!(s.name, "curves");
        assert_eq!(OpImageAdjustmentCurves::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentCurves::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_curves_zero_strength_identity() {{
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([128, 128, 128, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: img, change_id: get_id() }}, None, None),
            Input::new("strength".to_string(), Value::Decimal(0.0), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert!((p[0] as i32 - 128).abs() <= 1);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_curves_positive_strength() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("strength".to_string(), Value::Decimal(0.5), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/gradient_map.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::color::Color;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_gradient_map_settings() {{
        let s = OpImageAdjustmentGradientMap::settings();
        assert_eq!(s.name, "gradient map");
        assert_eq!(OpImageAdjustmentGradientMap::create_inputs().len(), 6);
        assert_eq!(OpImageAdjustmentGradientMap::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_gradient_map_two_color() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/directional_blur.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_directional_blur_settings() {{
        let s = OpImageAdjustmentDirectionalBlur::settings();
        assert_eq!(s.name, "directional blur");
        assert_eq!(OpImageAdjustmentDirectionalBlur::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentDirectionalBlur::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_directional_blur_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_directional_blur_zero_intensity() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
            Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/radial_blur.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_radial_blur_settings() {{
        let s = OpImageAdjustmentRadialBlur::settings();
        assert_eq!(s.name, "radial blur");
        assert_eq!(OpImageAdjustmentRadialBlur::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentRadialBlur::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_radial_blur_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("angle".to_string(), Value::Decimal(10.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_radial_blur_zero_angle() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/slope_blur.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_slope_blur_settings() {{
        let s = OpImageAdjustmentSlopeBlur::settings();
        assert_eq!(s.name, "slope blur");
        assert_eq!(OpImageAdjustmentSlopeBlur::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentSlopeBlur::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_slope_blur_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("slope map".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentSlopeBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/non_uniform_blur.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_non_uniform_blur_settings() {{
        let s = OpImageAdjustmentNonUniformBlur::settings();
        assert_eq!(s.name, "non-uniform blur");
        assert_eq!(OpImageAdjustmentNonUniformBlur::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentNonUniformBlur::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_non_uniform_blur_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("blur map".to_string(), image_input(8, 8), None, None),
            Input::new("max intensity".to_string(), Value::Decimal(5.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageAdjustmentNonUniformBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/edge_detect.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_edge_detect_settings() {{
        let s = OpImageAdjustmentEdgeDetect::settings();
        assert_eq!(s.name, "edge detect");
        assert_eq!(OpImageAdjustmentEdgeDetect::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentEdgeDetect::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_edge_detect_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_edge_detect_uniform_image() {{
        let uniform = {{
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([128, 128, 128, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        }};
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: uniform, change_id: get_id() }}, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let buf = data.to_rgba8();
                let p = buf.get_pixel(4, 4).0;
                assert!(p[0] < 5, "Expected near-zero edge, got {{}}", p[0]);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/emboss.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_emboss_settings() {{
        let s = OpImageAdjustmentEmboss::settings();
        assert_eq!(s.name, "emboss");
        assert_eq!(OpImageAdjustmentEmboss::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentEmboss::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_emboss_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
        ];
        let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/sharpen.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_sharpen_settings() {{
        let s = OpImageAdjustmentSharpen::settings();
        assert_eq!(s.name, "sharpen");
        assert_eq!(OpImageAdjustmentSharpen::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentSharpen::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_sharpen_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/posterize.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_posterize_settings() {{
        let s = OpImageAdjustmentPosterize::settings();
        assert_eq!(s.name, "posterize");
        assert_eq!(OpImageAdjustmentPosterize::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentPosterize::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_posterize_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("levels".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_posterize_two_levels() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("levels".to_string(), Value::Integer(2), None, None),
        ];
        let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let buf = data.to_rgba32f();
                for pixel in buf.pixels() {{
                    for c in 0..3 {{
                        assert!(pixel[c] == 0.0 || pixel[c] == 1.0,
                            "Expected 0 or 1, got {{}}", pixel[c]);
                    }}
                }}
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/histogram_scan.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_histogram_scan_settings() {{
        let s = OpImageAdjustmentHistogramScan::settings();
        assert_eq!(s.name, "histogram scan");
        assert_eq!(OpImageAdjustmentHistogramScan::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentHistogramScan::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_histogram_scan_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("position".to_string(), Value::Decimal(0.5), None, None),
            Input::new("range".to_string(), Value::Decimal(0.1), None, None),
        ];
        let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_histogram_scan_full_range() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("position".to_string(), Value::Decimal(0.5), None, None),
            Input::new("range".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let buf = data.to_rgba32f();
                for pixel in buf.pixels() {{
                    assert!(pixel[0] > 0.9, "Expected near-white with full range, got {{}}", pixel[0]);
                }}
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/histogram_range.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_histogram_range_settings() {{
        let s = OpImageAdjustmentHistogramRange::settings();
        assert_eq!(s.name, "histogram range");
        assert_eq!(OpImageAdjustmentHistogramRange::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentHistogramRange::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_histogram_range_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("range min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("range max".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/auto_levels.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_auto_levels_settings() {{
        let s = OpImageAdjustmentAutoLevels::settings();
        assert_eq!(s.name, "auto levels");
        assert_eq!(OpImageAdjustmentAutoLevels::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentAutoLevels::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_auto_levels_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
            Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
        ];
        let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/adjustments/distance.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_distance_settings() {{
        let s = OpImageAdjustmentDistance::settings();
        assert_eq!(s.name, "distance");
        assert_eq!(OpImageAdjustmentDistance::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentDistance::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_distance_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
            Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
        ];
        let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_distance_all_white() {{
        let white = {{
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        }};
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: white, change_id: get_id() }}, None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
            Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
        ];
        let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let buf = data.to_rgba32f();
                let p = buf.get_pixel(4, 4).0;
                assert!(p[0] >= 0.5, "Inside pixel should be >= 0.5, got {{}}", p[0]);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

# Image inputs
files_and_tests["images/inputs/color.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_from_color() {{
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageInputColor::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_from_color_settings() {{
        let s = OpImageInputColor::settings();
        assert_eq!(s.name, "from color");
        assert_eq!(OpImageInputColor::create_inputs().len(), 3);
        assert_eq!(OpImageInputColor::create_outputs().len(), 4);
    }}
}}
"""

files_and_tests["images/inputs/gradient.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::color::Color;
    use crate::color::color_spaces::ColorSpace;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_gradient_srgb() {{
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        ];
        let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 4);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_gradient_settings() {{
        let s = OpImageInputGradient::settings();
        assert_eq!(s.name, "from gradient");
        assert_eq!(OpImageInputGradient::create_inputs().len(), 5);
        assert_eq!(OpImageInputGradient::create_outputs().len(), 3);
    }}
}}
"""

files_and_tests["images/inputs/file.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_input_settings() {
        let s = OpImageInputFile::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageInputFile::create_inputs().is_empty());
        assert!(!OpImageInputFile::create_outputs().is_empty());
    }
}
"""

files_and_tests["images/inputs/url.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_input_settings() {
        let s = OpImageInputUrl::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageInputUrl::create_inputs().is_empty());
        assert!(!OpImageInputUrl::create_outputs().is_empty());
    }
}
"""

files_and_tests["images/inputs/clipboard.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_input_settings() {
        let s = OpImageInputClipboard::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageInputClipboard::create_inputs().is_empty());
        assert!(!OpImageInputClipboard::create_outputs().is_empty());
    }
}
"""

files_and_tests["images/outputs/file.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_output_settings() {
        let s = OpImageOutputFile::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageOutputFile::create_inputs().is_empty());
        assert!(!OpImageOutputFile::create_outputs().is_empty());
    }
}
"""

files_and_tests["images/outputs/clipboard.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_output_settings() {
        let s = OpImageOutputClipboard::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageOutputClipboard::create_inputs().is_empty());
        assert_eq!(OpImageOutputClipboard::create_outputs().len(), 0);
    }
}
"""

# Image combine
files_and_tests["images/combine/blit.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_blit_settings() {{
        let s = OpImageCombineBlit::settings();
        assert_eq!(s.name, "blit");
        assert_eq!(OpImageCombineBlit::create_inputs().len(), 4);
        assert_eq!(OpImageCombineBlit::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_blit() {{
        let mut inputs = vec![
            Input::new("background".to_string(), image_input(8, 8), None, None),
            Input::new("foreground".to_string(), image_input(4, 4), None, None),
            Input::new("position x".to_string(), Value::Integer(2), None, None),
            Input::new("position y".to_string(), Value::Integer(2), None, None),
        ];
        let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/combine/blend.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::color::blend::BlendMode;
    use crate::color::color_spaces::ColorSpace;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_blend_settings() {{
        let s = OpImageCombineBlend::settings();
        assert_eq!(s.name, "blend");
        assert_eq!(OpImageCombineBlend::create_inputs().len(), 8);
        assert_eq!(OpImageCombineBlend::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_blend() {{
        let mut inputs = vec![
            Input::new("background".to_string(), image_input(4, 4), None, None),
            Input::new("foreground".to_string(), image_input(4, 4), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("alpha".to_string(), image_input(4, 4), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Normal), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

# Image channels
files_and_tests["images/channels/split.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_split_settings() {{
        let s = OpImageChannelSplit::settings();
        assert_eq!(s.name, "channel split");
        assert_eq!(OpImageChannelSplit::create_inputs().len(), 1);
        assert_eq!(OpImageChannelSplit::create_outputs().len(), 4);
    }}

    #[tokio::test]
    async fn test_split_produces_four_outputs() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        for i in 0..4 {{
            match &result.responses[i].value {{
                Value::DynamicImage {{ data, .. }} => {{
                    assert_eq!(data.width(), 4);
                    assert_eq!(data.height(), 4);
                }}
                other => panic!("Expected DynamicImage, got {{:?}}", other),
            }}
        }}
    }}

    #[tokio::test]
    async fn test_split_channel_values() {{
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([100, 150, 200, 250]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: img, change_id: get_id() }}, None, None),
        ];
        let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p[0], 100);
                assert_eq!(p[1], 100);
                assert_eq!(p[2], 100);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/channels/merge.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_merge_settings() {{
        let s = OpImageChannelMerge::settings();
        assert_eq!(s.name, "channel merge");
        assert_eq!(OpImageChannelMerge::create_inputs().len(), 4);
        assert_eq!(OpImageChannelMerge::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_merge_produces_image() {{
        let mut inputs = vec![
            Input::new("red".to_string(), image_input(4, 4), None, None),
            Input::new("green".to_string(), image_input(4, 4), None, None),
            Input::new("blue".to_string(), image_input(4, 4), None, None),
            Input::new("alpha".to_string(), image_input(4, 4), None, None),
        ];
        let result = OpImageChannelMerge::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 4);
                assert_eq!(data.height(), 4);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/channels/shuffle.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_shuffle_settings() {{
        let s = OpImageChannelShuffle::settings();
        assert_eq!(s.name, "channel shuffle");
        assert_eq!(OpImageChannelShuffle::create_inputs().len(), 5);
        assert_eq!(OpImageChannelShuffle::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_shuffle_identity() {{
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: img, change_id: get_id() }}, None, None),
            Input::new("red source".to_string(), Value::Integer(0), None, None),
            Input::new("green source".to_string(), Value::Integer(1), None, None),
            Input::new("blue source".to_string(), Value::Integer(2), None, None),
            Input::new("alpha source".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p, [10, 20, 30, 40]);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_shuffle_swap_red_blue() {{
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage {{ data: img, change_id: get_id() }}, None, None),
            Input::new("red source".to_string(), Value::Integer(2), None, None),
            Input::new("green source".to_string(), Value::Integer(1), None, None),
            Input::new("blue source".to_string(), Value::Integer(0), None, None),
            Input::new("alpha source".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p, [30, 20, 10, 40]);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

# Image transforms
files_and_tests["images/transform/crop.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_crop_settings() {{
        let s = OpImageTransformCrop::settings();
        assert_eq!(s.name, "crop");
        assert_eq!(OpImageTransformCrop::create_inputs().len(), 5);
        assert_eq!(OpImageTransformCrop::create_outputs().len(), 3);
    }}

    #[tokio::test]
    async fn test_crop() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("x".to_string(), Value::Integer(1), None, None),
            Input::new("y".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3);
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/resize.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_resize_settings() {{
        let s = OpImageTransformResize::settings();
        assert_eq!(s.name, "resize");
        assert_eq!(OpImageTransformResize::create_inputs().len(), 4);
        assert_eq!(OpImageTransformResize::create_outputs().len(), 3);
    }}

    #[tokio::test]
    async fn test_resize() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
        ];
        let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3);
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/resize_exact.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_resize_exact_settings() {{
        let s = OpImageTransformResizeExact::settings();
        assert_eq!(s.name, "resize exact");
        assert_eq!(OpImageTransformResizeExact::create_inputs().len(), 4);
        assert_eq!(OpImageTransformResizeExact::create_outputs().len(), 3);
    }}

    #[tokio::test]
    async fn test_resize_exact() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
        ];
        let result = OpImageTransformResizeExact::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 4);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/resize_fill.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_resize_fill_settings() {{
        let s = OpImageTransformResizeFill::settings();
        assert_eq!(s.name, "resize fill");
        assert_eq!(OpImageTransformResizeFill::create_inputs().len(), 4);
        assert_eq!(OpImageTransformResizeFill::create_outputs().len(), 3);
    }}

    #[tokio::test]
    async fn test_resize_fill() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
        ];
        let result = OpImageTransformResizeFill::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3);
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/flip_horizontal.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_flip_horizontal_settings() {{
        let s = OpImageTransformFlipHorizontal::settings();
        assert_eq!(s.name, "flip horizontal");
        assert_eq!(OpImageTransformFlipHorizontal::create_inputs().len(), 1);
        assert_eq!(OpImageTransformFlipHorizontal::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_flip_horizontal() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/flip_vertical.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_flip_vertical_settings() {{
        let s = OpImageTransformFlipVertical::settings();
        assert_eq!(s.name, "flip vertical");
        assert_eq!(OpImageTransformFlipVertical::create_inputs().len(), 1);
        assert_eq!(OpImageTransformFlipVertical::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_flip_vertical() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/rotate_90.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_rotate_90_settings() {{
        let s = OpImageTransformRotate90::settings();
        assert_eq!(s.name, "rotate 90");
        assert_eq!(OpImageTransformRotate90::create_inputs().len(), 1);
        assert_eq!(OpImageTransformRotate90::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_rotate_90() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 8), None, None)];
        let result = OpImageTransformRotate90::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 4);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/rotate_180.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_rotate_180_settings() {{
        let s = OpImageTransformRotate180::settings();
        assert_eq!(s.name, "rotate 180");
        assert_eq!(OpImageTransformRotate180::create_inputs().len(), 1);
        assert_eq!(OpImageTransformRotate180::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_rotate_180() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformRotate180::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/rotate_270.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_rotate_270_settings() {{
        let s = OpImageTransformRotate270::settings();
        assert_eq!(s.name, "rotate 270");
        assert_eq!(OpImageTransformRotate270::create_inputs().len(), 1);
        assert_eq!(OpImageTransformRotate270::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_rotate_270() {{
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformRotate270::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/rotate_around_center.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::color::Color;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_rotate_around_center_settings() {{
        let s = OpImageTransformRotateAroundCenter::settings();
        assert_eq!(s.name, "rotate around center");
        assert_eq!(OpImageTransformRotateAroundCenter::create_inputs().len(), 3);
        assert_eq!(OpImageTransformRotateAroundCenter::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_rotate_around_center() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("degrees".to_string(), Value::Decimal(45.0), None, None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
        ];
        let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ .. }} => {{}}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/warp.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    fn gradient_h_image(w: u32, h: u32) -> Value {{
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, _y, pixel) in imgbuf.enumerate_pixels_mut() {{
            let v = (x * 255 / w.max(1)) as u8;
            *pixel = image::Rgba([v, v, v, 255]);
        }}
        Value::DynamicImage {{ data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }}
    }}

    #[tokio::test]
    async fn test_warp_settings() {{
        let s = OpImageTransformWarp::settings();
        assert_eq!(s.name, "warp");
        assert_eq!(OpImageTransformWarp::create_inputs().len(), 3);
        assert_eq!(OpImageTransformWarp::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_warp_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("displacement".to_string(), gradient_h_image(16, 16), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 16);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_bilinear_sample_exact_pixel() {{
        let mut img = image::RgbaImage::new(4, 4);
        img.put_pixel(2, 1, image::Rgba([255, 0, 0, 255]));
        let result = bilinear_sample_rgba(&img, 2.0, 1.0);
        assert_eq!(result, [255, 0, 0, 255]);
    }}
}}
"""

files_and_tests["images/transform/directional_warp.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    fn gradient_h_image(w: u32, h: u32) -> Value {{
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, _y, pixel) in imgbuf.enumerate_pixels_mut() {{
            let v = (x * 255 / w.max(1)) as u8;
            *pixel = image::Rgba([v, v, v, 255]);
        }}
        Value::DynamicImage {{ data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }}
    }}

    #[tokio::test]
    async fn test_directional_warp_settings() {{
        let s = OpImageTransformDirectionalWarp::settings();
        assert_eq!(s.name, "directional warp");
        assert_eq!(OpImageTransformDirectionalWarp::create_inputs().len(), 4);
        assert_eq!(OpImageTransformDirectionalWarp::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_directional_warp_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("intensity map".to_string(), gradient_h_image(16, 16), None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 16);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/safe_transform.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_safe_transform_settings() {{
        let s = OpImageTransformSafeTransform::settings();
        assert_eq!(s.name, "safe transform");
        assert_eq!(OpImageTransformSafeTransform::create_inputs().len(), 5);
        assert_eq!(OpImageTransformSafeTransform::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_safe_transform_identity() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/make_tile.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_make_tile_settings() {{
        let s = OpImageTransformMakeTile::settings();
        assert_eq!(s.name, "make tile");
        assert_eq!(OpImageTransformMakeTile::create_inputs().len(), 2);
        assert_eq!(OpImageTransformMakeTile::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_make_tile_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
        ];
        let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 16);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

files_and_tests["images/transform/mirror.rs"] = f"""
#[cfg(test)]
mod tests {{
    use super::*;
{IMAGE_HELPERS}
    #[tokio::test]
    async fn test_mirror_settings() {{
        let s = OpImageTransformMirror::settings();
        assert_eq!(s.name, "mirror");
        assert_eq!(OpImageTransformMirror::create_inputs().len(), 5);
        assert_eq!(OpImageTransformMirror::create_outputs().len(), 1);
    }}

    #[tokio::test]
    async fn test_mirror_x_basic() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 16);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}

    #[tokio::test]
    async fn test_mirror_x_symmetry() {{
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {{
            Value::DynamicImage {{ data, .. }} => {{
                let rgba = data.to_rgba8();
                let left = rgba.get_pixel(3, 0).0;
                let right = rgba.get_pixel(4, 0).0;
                assert_eq!(left, right);
            }}
            other => panic!("Expected DynamicImage, got {{:?}}", other),
        }}
    }}
}}
"""

for rel_path, test_block in files_and_tests.items():
    full_path = os.path.join(base, rel_path)
    with open(full_path, 'a') as f:
        f.write(test_block)
    print(f"Updated: {rel_path}")

print("Done!")
