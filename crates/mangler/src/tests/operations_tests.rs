#[cfg(test)]
mod operations_tests {
    use crate::color::Color;
    use crate::color::blend::BlendMode;
    use crate::color::color_spaces::ColorSpace;
    use crate::get_id;
    use crate::input::Input;
    use crate::operations::default_image;
    use crate::value::Value;
    use image::DynamicImage;
    use std::sync::Arc;

    macro_rules! assert_decimal {
        ($val:expr, $expected:expr) => {
            match &$val {
                Value::Decimal(v) => assert!(
                    (*v - $expected).abs() < 0.01,
                    "Expected ~{}, got {}",
                    $expected, v
                ),
                other => panic!("Expected Decimal(~{}), got {:?}", $expected, other),
            }
        };
    }

    macro_rules! assert_integer {
        ($val:expr, $expected:expr) => {
            match &$val {
                Value::Integer(v) => assert_eq!(*v, $expected),
                other => panic!("Expected Integer({}), got {:?}", $expected, other),
            }
        };
    }

    macro_rules! assert_color {
        ($val:expr) => {
            match &$val {
                Value::Color(_) => {}
                other => panic!("Expected Color, got {:?}", other),
            }
        };
    }

    macro_rules! assert_image {
        ($val:expr) => {
            match &$val {
                Value::DynamicImage { data, .. } => data.clone(),
                other => panic!("Expected DynamicImage, got {:?}", other),
            }
        };
    }

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
        Value::DynamicImage {
            data: test_image(w, h),
            change_id: get_id(),
        }
    }

    // ==================== NUMBER INPUTS ====================

    mod number_inputs {
        use super::*;
        use crate::operations::numbers::inputs::integer::OpNumberInputInteger;
        use crate::operations::numbers::inputs::decimal::OpNumberInputDecimal;

        #[tokio::test]
        async fn test_integer_input_passthrough() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
            let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 42);
        }

        #[tokio::test]
        async fn test_integer_input_negative() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Integer(-100), None, None)];
            let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, -100);
        }

        #[tokio::test]
        async fn test_integer_input_zero() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
            let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 0);
        }

        #[tokio::test]
        async fn test_integer_input_from_decimal() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.7), None, None)];
            let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 5);
        }

        #[tokio::test]
        async fn test_integer_settings() {
            let s = OpNumberInputInteger::settings();
            assert_eq!(s.name, "integer");
            assert_eq!(OpNumberInputInteger::create_inputs().len(), 1);
            assert_eq!(OpNumberInputInteger::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_decimal_input_passthrough() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
            let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.14);
        }

        #[tokio::test]
        async fn test_decimal_input_negative() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-2.5), None, None)];
            let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, -2.5);
        }

        #[tokio::test]
        async fn test_decimal_input_zero() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
            let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_decimal_input_from_integer() {
            let mut inputs = vec![Input::new("input".to_string(), Value::Integer(7), None, None)];
            let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 7.0);
        }

        #[tokio::test]
        async fn test_decimal_settings() {
            let s = OpNumberInputDecimal::settings();
            assert_eq!(s.name, "decimal");
            assert_eq!(OpNumberInputDecimal::create_inputs().len(), 1);
            assert_eq!(OpNumberInputDecimal::create_outputs().len(), 1);
        }
    }

    // ==================== NUMBER RANDOM ====================

    mod number_random {
        use super::*;
        use crate::operations::numbers::random::random_integer::OpNumberRandomInteger;
        use crate::operations::numbers::random::random_decimal::OpNumberRandomDecimal;

        #[tokio::test]
        async fn test_random_integer_in_range() {
            let mut inputs = vec![
                Input::new("generate".to_string(), Value::Trigger, None, None),
                Input::new("min".to_string(), Value::Integer(0), None, None),
                Input::new("max".to_string(), Value::Integer(100), None, None),
            ];
            let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
            match &result.responses[0].value {
                Value::Integer(v) => assert!(*v >= 0 && *v < 100),
                other => panic!("Expected Integer, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn test_random_integer_min_equals_max() {
            let mut inputs = vec![
                Input::new("generate".to_string(), Value::Trigger, None, None),
                Input::new("min".to_string(), Value::Integer(5), None, None),
                Input::new("max".to_string(), Value::Integer(5), None, None),
            ];
            let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
            // max gets clamped to min+1, so result should be 5
            assert_integer!(result.responses[0].value, 5);
        }

        #[tokio::test]
        async fn test_random_integer_settings() {
            let s = OpNumberRandomInteger::settings();
            assert_eq!(s.name, "random integer");
            assert_eq!(OpNumberRandomInteger::create_inputs().len(), 3);
            assert_eq!(OpNumberRandomInteger::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_random_decimal_returns_float() {
            let inputs = vec![Input::new("generate".to_string(), Value::Trigger, None, None)];
            let result = OpNumberRandomDecimal::run(&inputs).await.unwrap();
            match &result.responses[0].value {
                Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 1.0),
                other => panic!("Expected Decimal, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn test_random_decimal_settings() {
            let s = OpNumberRandomDecimal::settings();
            assert_eq!(s.name, "random decimal");
            assert_eq!(OpNumberRandomDecimal::create_inputs().len(), 1);
            assert_eq!(OpNumberRandomDecimal::create_outputs().len(), 1);
        }
    }

    // ==================== COLOR INPUTS ====================

    mod color_inputs {
        use super::*;
        use crate::operations::colors::inputs::srgb::OpColorInputRgba;
        use crate::operations::colors::inputs::hsl::OpColorInputHsla;
        use crate::operations::colors::inputs::hsv::OpColorInputHsva;
        use crate::operations::colors::inputs::lab::OpColorInputLab;
        use crate::operations::colors::inputs::lch::OpColorInputLch;
        use crate::operations::colors::inputs::rgb_linear::OpColorInputRgbaLinear;
        use crate::operations::colors::inputs::xyz::OpColorInputXyz;
        use crate::operations::colors::inputs::yuv::OpColorInputYuv;
        use crate::operations::colors::inputs::cmyk::OpColorInputCmyk;

        fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
            vals.iter()
                .enumerate()
                .map(|(i, v)| Input::new(format!("v{}", i), Value::Decimal(*v), None, None))
                .collect()
        }

        #[tokio::test]
        async fn test_srgb_input() {
            let mut inputs = decimal_inputs(&[1.0, 0.0, 0.0, 1.0]);
            let result = OpColorInputRgba::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_srgb_settings() {
            let s = OpColorInputRgba::settings();
            assert_eq!(s.name, "rgb");
            assert_eq!(OpColorInputRgba::create_inputs().len(), 4);
            assert_eq!(OpColorInputRgba::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_hsl_input() {
            let mut inputs = decimal_inputs(&[180.0, 1.0, 0.5, 1.0]);
            let result = OpColorInputHsla::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_hsl_settings() {
            let s = OpColorInputHsla::settings();
            assert_eq!(s.name, "hsl");
            assert_eq!(OpColorInputHsla::create_inputs().len(), 4);
            assert_eq!(OpColorInputHsla::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_hsv_input() {
            let mut inputs = decimal_inputs(&[120.0, 1.0, 1.0, 1.0]);
            let result = OpColorInputHsva::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_hsv_settings() {
            let s = OpColorInputHsva::settings();
            assert_eq!(s.name, "hsv");
            assert_eq!(OpColorInputHsva::create_inputs().len(), 4);
            assert_eq!(OpColorInputHsva::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_lab_input() {
            let mut inputs = decimal_inputs(&[50.0, 20.0, -30.0, 1.0]);
            let result = OpColorInputLab::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_lab_settings() {
            let s = OpColorInputLab::settings();
            assert_eq!(s.name, "lab");
            assert_eq!(OpColorInputLab::create_inputs().len(), 4);
            assert_eq!(OpColorInputLab::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_lch_input() {
            let mut inputs = decimal_inputs(&[0.6, 0.5, 180.0, 1.0]);
            let result = OpColorInputLch::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_lch_settings() {
            let s = OpColorInputLch::settings();
            assert_eq!(s.name, "lch");
            assert_eq!(OpColorInputLch::create_inputs().len(), 4);
            assert_eq!(OpColorInputLch::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rgb_linear_input() {
            let mut inputs = decimal_inputs(&[0.5, 0.5, 0.5, 1.0]);
            let result = OpColorInputRgbaLinear::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_rgb_linear_settings() {
            let s = OpColorInputRgbaLinear::settings();
            assert_eq!(s.name, "rgb linear");
            assert_eq!(OpColorInputRgbaLinear::create_inputs().len(), 4);
            assert_eq!(OpColorInputRgbaLinear::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_xyz_input() {
            let mut inputs = decimal_inputs(&[0.5, 0.2, 0.1, 1.0]);
            let result = OpColorInputXyz::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_xyz_settings() {
            let s = OpColorInputXyz::settings();
            assert_eq!(s.name, "xyz");
            assert_eq!(OpColorInputXyz::create_inputs().len(), 4);
            assert_eq!(OpColorInputXyz::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_yuv_input() {
            let mut inputs = decimal_inputs(&[0.5, 0.3, 0.2, 1.0]);
            let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_yuv_settings() {
            let s = OpColorInputYuv::settings();
            assert_eq!(s.name, "yuv");
            assert_eq!(OpColorInputYuv::create_inputs().len(), 4);
            assert_eq!(OpColorInputYuv::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_cmyk_input() {
            let mut inputs = decimal_inputs(&[0.0, 1.0, 1.0, 0.0, 1.0]);
            let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_cmyk_settings() {
            let s = OpColorInputCmyk::settings();
            assert_eq!(s.name, "cmyk");
            assert_eq!(OpColorInputCmyk::create_inputs().len(), 5);
            assert_eq!(OpColorInputCmyk::create_outputs().len(), 1);
        }
    }

    // ==================== COLOR OUTPUTS ====================

    mod color_outputs {
        use super::*;
        use crate::operations::colors::outputs::to_cmyk::OpColorOutputCmyk;
        use crate::operations::colors::outputs::to_hsl::OpColorOutputHsl;
        use crate::operations::colors::outputs::to_hsv::OpColorOutputHsv;
        use crate::operations::colors::outputs::to_lab::OpColorOutputLab;
        use crate::operations::colors::outputs::to_lch::OpColorOutputLch;
        use crate::operations::colors::outputs::to_rgb_linear::OpColorOutputRgbLinear;
        use crate::operations::colors::outputs::to_srgb::OpColorOutputRgb;
        use crate::operations::colors::outputs::to_xyz::OpColorOutputXyz;
        use crate::operations::colors::outputs::to_yuv::OpColorOutputYuv;

        fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
            vec![Input::new(
                "input".to_string(),
                Value::Color(Color::from_srgb_float(r, g, b, a)),
                None,
                None,
            )]
        }

        #[tokio::test]
        async fn test_to_cmyk() {
            let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
            let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 5);
            // Red in CMYK: C=0, M=1, Y=1, K=0
            assert_decimal!(result.responses[0].value, 0.0); // cyan
            assert_decimal!(result.responses[1].value, 1.0); // magenta
            assert_decimal!(result.responses[2].value, 1.0); // yellow
            assert_decimal!(result.responses[3].value, 0.0); // key
            assert_decimal!(result.responses[4].value, 1.0); // alpha
        }

        #[tokio::test]
        async fn test_to_cmyk_settings() {
            let s = OpColorOutputCmyk::settings();
            assert_eq!(s.name, "to cmyk");
            assert_eq!(OpColorOutputCmyk::create_inputs().len(), 1);
            assert_eq!(OpColorOutputCmyk::create_outputs().len(), 5);
        }

        #[tokio::test]
        async fn test_to_hsl() {
            let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
            let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 4);
            assert_decimal!(result.responses[0].value, 0.0); // hue
        }

        #[tokio::test]
        async fn test_to_hsl_settings() {
            let s = OpColorOutputHsl::settings();
            assert_eq!(s.name, "to hsl");
            assert_eq!(OpColorOutputHsl::create_inputs().len(), 1);
            assert_eq!(OpColorOutputHsl::create_outputs().len(), 4);
        }

        #[tokio::test]
        async fn test_to_hsv() {
            let mut inputs = color_input(0.0, 1.0, 0.0, 1.0);
            let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 4);
            assert_decimal!(result.responses[0].value, 120.0); // hue
        }

        #[tokio::test]
        async fn test_to_hsv_settings() {
            let s = OpColorOutputHsv::settings();
            assert_eq!(s.name, "to hsv");
            assert_eq!(OpColorOutputHsv::create_inputs().len(), 1);
            assert_eq!(OpColorOutputHsv::create_outputs().len(), 4);
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

        #[tokio::test]
        async fn test_to_rgb_linear() {
            let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
            let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 4);
            assert_decimal!(result.responses[3].value, 1.0); // alpha
        }

        #[tokio::test]
        async fn test_to_rgb_linear_settings() {
            let s = OpColorOutputRgbLinear::settings();
            assert_eq!(s.name, "to rgb linear");
            assert_eq!(OpColorOutputRgbLinear::create_inputs().len(), 1);
            assert_eq!(OpColorOutputRgbLinear::create_outputs().len(), 4);
        }

        #[tokio::test]
        async fn test_to_srgb() {
            let mut inputs = color_input(0.8, 0.2, 0.4, 0.5);
            let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 4);
            assert_decimal!(result.responses[0].value, 0.8);
            assert_decimal!(result.responses[3].value, 0.5); // alpha
        }

        #[tokio::test]
        async fn test_to_srgb_settings() {
            let s = OpColorOutputRgb::settings();
            assert_eq!(s.name, "to rgb");
            assert_eq!(OpColorOutputRgb::create_inputs().len(), 1);
            assert_eq!(OpColorOutputRgb::create_outputs().len(), 4);
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

    // ==================== COLOR BLEND ====================

    mod color_blend {
        use super::*;
        use crate::operations::colors::blend::lerp::OpColorBlendLerp;

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
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_rgb_linear() {
            let mut inputs = blend_inputs(ColorSpace::RgbLinear, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_hsl() {
            let mut inputs = blend_inputs(ColorSpace::Hsl, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_hsv() {
            let mut inputs = blend_inputs(ColorSpace::Hsv, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_lch() {
            let mut inputs = blend_inputs(ColorSpace::Lch, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_xyz() {
            let mut inputs = blend_inputs(ColorSpace::Xyz, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_lab() {
            let mut inputs = blend_inputs(ColorSpace::Lab, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_yuv() {
            let mut inputs = blend_inputs(ColorSpace::Yuv, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_cmyk() {
            let mut inputs = blend_inputs(ColorSpace::Cmyk, 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_amount_zero() {
            let mut inputs = blend_inputs(ColorSpace::Srgb, 0.0);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_amount_one() {
            let mut inputs = blend_inputs(ColorSpace::Srgb, 1.0);
            let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
            assert_color!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_settings() {
            let s = OpColorBlendLerp::settings();
            assert_eq!(s.name, "blend");
            assert_eq!(OpColorBlendLerp::create_inputs().len(), 4);
            assert_eq!(OpColorBlendLerp::create_outputs().len(), 1);
        }
    }

    // ==================== COLOR SAMPLE ====================

    mod color_sample {
        use super::*;
        use crate::operations::colors::sample_image::most_common_colors::OpColorSampleMostCommonColors;

        #[tokio::test]
        async fn test_most_common_colors() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
                Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
                Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
            assert!(result.responses.len() <= 5);
            for resp in &result.responses {
                assert_color!(resp.value);
            }
        }

        #[tokio::test]
        async fn test_most_common_colors_solid() {
            // Solid red image - should return ~1 color
            let mut imgbuf = image::RgbaImage::new(4, 4);
            for pixel in imgbuf.pixels_mut() {
                *pixel = image::Rgba([255, 0, 0, 255]);
            }
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
                Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
                Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
            assert!(result.responses.len() >= 1);
        }

        #[tokio::test]
        async fn test_most_common_colors_settings() {
            let s = OpColorSampleMostCommonColors::settings();
            assert_eq!(s.name, "most common colors");
            assert_eq!(OpColorSampleMostCommonColors::create_inputs().len(), 4);
            assert_eq!(OpColorSampleMostCommonColors::create_outputs().len(), 5);
        }
    }

    // ==================== IMAGE INPUTS ====================

    mod image_inputs {
        use super::*;
        use crate::operations::images::inputs::color::OpImageInputColor;
        use crate::operations::images::inputs::gradient::OpImageInputGradient;
        use crate::operations::images::inputs::file::OpImageInputFile;
        use crate::operations::images::inputs::url::OpImageInputUrl;
        use crate::operations::images::inputs::clipboard::OpImageInputClipboard;

        #[tokio::test]
        async fn test_from_color() {
            let mut inputs = vec![
                Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
            ];
            let result = OpImageInputColor::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 4);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
            assert_color!(result.responses[1].value);
            assert_integer!(result.responses[2].value, 8);
            assert_integer!(result.responses[3].value, 8);
        }

        #[tokio::test]
        async fn test_from_color_min_size() {
            let mut inputs = vec![
                Input::new("color".to_string(), Value::Color(Color::default()), None, None),
                Input::new("width".to_string(), Value::Integer(0), None, None),
                Input::new("height".to_string(), Value::Integer(-5), None, None),
            ];
            let result = OpImageInputColor::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert!(img.width() >= 1);
            assert!(img.height() >= 1);
        }

        #[tokio::test]
        async fn test_from_color_settings() {
            let s = OpImageInputColor::settings();
            assert_eq!(s.name, "from color");
            assert_eq!(OpImageInputColor::create_inputs().len(), 3);
            assert_eq!(OpImageInputColor::create_outputs().len(), 4);
        }

        #[tokio::test]
        async fn test_gradient_srgb() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
                Input::new("width".to_string(), Value::Integer(4), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            ];
            let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 4);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_gradient_all_color_spaces() {
            let spaces = [
                ColorSpace::Srgb, ColorSpace::RgbLinear, ColorSpace::Hsl,
                ColorSpace::Hsv, ColorSpace::Lch, ColorSpace::Xyz,
                ColorSpace::Lab, ColorSpace::Yuv, ColorSpace::Cmyk,
            ];
            for cs in spaces {
                let mut inputs = vec![
                    Input::new("a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
                    Input::new("b".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
                    Input::new("width".to_string(), Value::Integer(4), None, None),
                    Input::new("height".to_string(), Value::Integer(4), None, None),
                    Input::new("color space".to_string(), Value::ColorSpace(cs), None, None),
                ];
                let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
                assert_image!(result.responses[0].value);
            }
        }

        #[tokio::test]
        async fn test_gradient_settings() {
            let s = OpImageInputGradient::settings();
            assert_eq!(s.name, "from gradient");
            assert_eq!(OpImageInputGradient::create_inputs().len(), 5);
            assert_eq!(OpImageInputGradient::create_outputs().len(), 4);
        }

        // I/O operations: test settings/inputs/outputs only (no filesystem/network/clipboard)
        #[tokio::test]
        async fn test_file_input_settings() {
            let s = OpImageInputFile::settings();
            assert!(!s.name.is_empty());
            assert!(!OpImageInputFile::create_inputs().is_empty());
            assert!(!OpImageInputFile::create_outputs().is_empty());
        }

        #[tokio::test]
        async fn test_url_input_settings() {
            let s = OpImageInputUrl::settings();
            assert!(!s.name.is_empty());
            assert!(!OpImageInputUrl::create_inputs().is_empty());
            assert!(!OpImageInputUrl::create_outputs().is_empty());
        }

        #[tokio::test]
        async fn test_clipboard_input_settings() {
            let s = OpImageInputClipboard::settings();
            assert!(!s.name.is_empty());
            assert!(!OpImageInputClipboard::create_inputs().is_empty());
            assert!(!OpImageInputClipboard::create_outputs().is_empty());
        }
    }

    // ==================== IMAGE OUTPUTS ====================

    mod image_outputs {
        use super::*;
        use crate::operations::images::outputs::file::OpImageOutputFile;
        use crate::operations::images::outputs::clipboard::OpImageOutputClipboard;

        #[tokio::test]
        async fn test_file_output_settings() {
            let s = OpImageOutputFile::settings();
            assert!(!s.name.is_empty());
            assert!(!OpImageOutputFile::create_inputs().is_empty());
            assert!(!OpImageOutputFile::create_outputs().is_empty());
        }

        #[tokio::test]
        async fn test_clipboard_output_settings() {
            let s = OpImageOutputClipboard::settings();
            assert!(!s.name.is_empty());
            assert!(!OpImageOutputClipboard::create_inputs().is_empty());
            // clipboard output has no outputs (it writes to clipboard)
            assert_eq!(OpImageOutputClipboard::create_outputs().len(), 0);
        }
    }

    // ==================== IMAGE ADJUSTMENTS ====================

    mod image_adjustments {
        use super::*;
        use crate::operations::images::adjustments::blur::OpImageAdjustmentBlur;
        use crate::operations::images::adjustments::contrast::OpImageAdjustmentContrast;
        use crate::operations::images::adjustments::grayscale::OpImageAdjustmentGrayscale;
        use crate::operations::images::adjustments::invert::OpImageAdjustmentInvert;
        use crate::operations::images::adjustments::brighten::OpImageAdjustmentBrighten;
        use crate::operations::images::adjustments::hue_rotate::OpImageAdjustmentHueRotate;
        use crate::operations::images::adjustments::unsharpen::OpImageAdjustmentUnsharpen;

        #[tokio::test]
        async fn test_blur() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blur_zero_sigma() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("sigma".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blur_negative_sigma_clamped() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("sigma".to_string(), Value::Decimal(-5.0), None, None),
            ];
            let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blur_settings() {
            let s = OpImageAdjustmentBlur::settings();
            assert_eq!(s.name, "blur");
            assert_eq!(OpImageAdjustmentBlur::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentBlur::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_contrast() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("amount".to_string(), Value::Decimal(1.5), None, None),
            ];
            let result = OpImageAdjustmentContrast::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_contrast_settings() {
            let s = OpImageAdjustmentContrast::settings();
            assert_eq!(s.name, "contrast");
            assert_eq!(OpImageAdjustmentContrast::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentContrast::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_grayscale() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageAdjustmentGrayscale::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_grayscale_settings() {
            let s = OpImageAdjustmentGrayscale::settings();
            assert_eq!(s.name, "grayscale");
            assert_eq!(OpImageAdjustmentGrayscale::create_inputs().len(), 1);
            assert_eq!(OpImageAdjustmentGrayscale::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_invert() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageAdjustmentInvert::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_invert_settings() {
            let s = OpImageAdjustmentInvert::settings();
            assert_eq!(s.name, "invert");
            assert_eq!(OpImageAdjustmentInvert::create_inputs().len(), 1);
            assert_eq!(OpImageAdjustmentInvert::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_brighten() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_brighten_negative() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("amount".to_string(), Value::Decimal(-0.5), None, None),
            ];
            let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_brighten_settings() {
            let s = OpImageAdjustmentBrighten::settings();
            assert_eq!(s.name, "brighten");
            assert_eq!(OpImageAdjustmentBrighten::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentBrighten::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_hue_rotate() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentHueRotate::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_hue_rotate_settings() {
            let s = OpImageAdjustmentHueRotate::settings();
            assert_eq!(s.name, "hue rotate");
            assert_eq!(OpImageAdjustmentHueRotate::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentHueRotate::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_unsharpen() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
                Input::new("threshold".to_string(), Value::Integer(1), None, None),
            ];
            let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_unsharpen_negative_sigma() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("sigma".to_string(), Value::Decimal(-2.0), None, None),
                Input::new("threshold".to_string(), Value::Integer(1), None, None),
            ];
            let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_unsharpen_settings() {
            let s = OpImageAdjustmentUnsharpen::settings();
            assert_eq!(s.name, "unsharpen");
            assert_eq!(OpImageAdjustmentUnsharpen::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentUnsharpen::create_outputs().len(), 1);
        }
    }

    // ==================== IMAGE TRANSFORMS ====================

    mod image_transforms {
        use super::*;
        use crate::operations::images::transform::crop::OpImageTransformCrop;
        use crate::operations::images::transform::resize::OpImageTransformResize;
        use crate::operations::images::transform::resize_exact::OpImageTransformResizeExact;
        use crate::operations::images::transform::resize_fill::OpImageTransformResizeFill;
        use crate::operations::images::transform::flip_horizontal::OpImageTransformFlipHorizontal;
        use crate::operations::images::transform::flip_vertical::OpImageTransformFlipVertical;
        use crate::operations::images::transform::rotate_90::OpImageTransformRotate90;
        use crate::operations::images::transform::rotate_180::OpImageTransformRotate180;
        use crate::operations::images::transform::rotate_270::OpImageTransformRotate270;
        use crate::operations::images::transform::rotate_around_center::OpImageTransformRotateAroundCenter;

        #[tokio::test]
        async fn test_crop() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("x".to_string(), Value::Integer(1), None, None),
                Input::new("y".to_string(), Value::Integer(1), None, None),
                Input::new("width".to_string(), Value::Integer(4), None, None),
                Input::new("height".to_string(), Value::Integer(4), None, None),
            ];
            let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 3);
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_crop_clamp_negative() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("x".to_string(), Value::Integer(-10), None, None),
                Input::new("y".to_string(), Value::Integer(-10), None, None),
                Input::new("width".to_string(), Value::Integer(0), None, None),
                Input::new("height".to_string(), Value::Integer(0), None, None),
            ];
            let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_crop_settings() {
            let s = OpImageTransformCrop::settings();
            assert_eq!(s.name, "crop");
            assert_eq!(OpImageTransformCrop::create_inputs().len(), 5);
            assert_eq!(OpImageTransformCrop::create_outputs().len(), 3);
        }

        #[tokio::test]
        async fn test_resize() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("width".to_string(), Value::Integer(4), None, None),
                Input::new("height".to_string(), Value::Integer(4), None, None),
                Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
            ];
            let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 3);
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_resize_min_clamp() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("width".to_string(), Value::Integer(0), None, None),
                Input::new("height".to_string(), Value::Integer(-1), None, None),
                Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
            ];
            let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_resize_settings() {
            let s = OpImageTransformResize::settings();
            assert_eq!(s.name, "resize");
            assert_eq!(OpImageTransformResize::create_inputs().len(), 4);
            assert_eq!(OpImageTransformResize::create_outputs().len(), 3);
        }

        #[tokio::test]
        async fn test_resize_exact() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("width".to_string(), Value::Integer(16), None, None),
                Input::new("height".to_string(), Value::Integer(4), None, None),
                Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
            ];
            let result = OpImageTransformResizeExact::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 3);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 4);
        }

        #[tokio::test]
        async fn test_resize_exact_settings() {
            let s = OpImageTransformResizeExact::settings();
            assert_eq!(s.name, "resize exact");
            assert_eq!(OpImageTransformResizeExact::create_inputs().len(), 4);
            assert_eq!(OpImageTransformResizeExact::create_outputs().len(), 3);
        }

        #[tokio::test]
        async fn test_resize_fill() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("width".to_string(), Value::Integer(4), None, None),
                Input::new("height".to_string(), Value::Integer(4), None, None),
                Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
            ];
            let result = OpImageTransformResizeFill::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 3);
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_resize_fill_settings() {
            let s = OpImageTransformResizeFill::settings();
            assert_eq!(s.name, "resize fill");
            assert_eq!(OpImageTransformResizeFill::create_inputs().len(), 4);
            assert_eq!(OpImageTransformResizeFill::create_outputs().len(), 3);
        }

        #[tokio::test]
        async fn test_flip_horizontal() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_flip_horizontal_settings() {
            let s = OpImageTransformFlipHorizontal::settings();
            assert_eq!(s.name, "flip horizontal");
            assert_eq!(OpImageTransformFlipHorizontal::create_inputs().len(), 1);
            assert_eq!(OpImageTransformFlipHorizontal::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_flip_vertical() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_flip_vertical_settings() {
            let s = OpImageTransformFlipVertical::settings();
            assert_eq!(s.name, "flip vertical");
            assert_eq!(OpImageTransformFlipVertical::create_inputs().len(), 1);
            assert_eq!(OpImageTransformFlipVertical::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rotate_90() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 8), None, None),
            ];
            let result = OpImageTransformRotate90::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 4);
        }

        #[tokio::test]
        async fn test_rotate_90_settings() {
            let s = OpImageTransformRotate90::settings();
            assert_eq!(s.name, "rotate 90");
            assert_eq!(OpImageTransformRotate90::create_inputs().len(), 1);
            assert_eq!(OpImageTransformRotate90::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rotate_180() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageTransformRotate180::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_rotate_180_settings() {
            let s = OpImageTransformRotate180::settings();
            assert_eq!(s.name, "rotate 180");
            assert_eq!(OpImageTransformRotate180::create_inputs().len(), 1);
            assert_eq!(OpImageTransformRotate180::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rotate_270() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageTransformRotate270::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_rotate_270_settings() {
            let s = OpImageTransformRotate270::settings();
            assert_eq!(s.name, "rotate 270");
            assert_eq!(OpImageTransformRotate270::create_inputs().len(), 1);
            assert_eq!(OpImageTransformRotate270::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rotate_around_center() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("degrees".to_string(), Value::Decimal(45.0), None, None),
                Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
            ];
            let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_rotate_around_center_settings() {
            let s = OpImageTransformRotateAroundCenter::settings();
            assert_eq!(s.name, "rotate around center");
            assert_eq!(OpImageTransformRotateAroundCenter::create_inputs().len(), 3);
            assert_eq!(OpImageTransformRotateAroundCenter::create_outputs().len(), 1);
        }
    }

    // ==================== IMAGE COMBINE ====================

    mod image_combine {
        use super::*;
        use crate::operations::images::combine::blit::OpImageCombineBlit;
        use crate::operations::images::combine::blend::OpImageCombineBlend;

        #[tokio::test]
        async fn test_blit() {
            let mut inputs = vec![
                Input::new("background".to_string(), image_input(8, 8), None, None),
                Input::new("foreground".to_string(), image_input(4, 4), None, None),
                Input::new("position x".to_string(), Value::Integer(2), None, None),
                Input::new("position y".to_string(), Value::Integer(2), None, None),
            ];
            let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_blit_at_origin() {
            let mut inputs = vec![
                Input::new("background".to_string(), image_input(8, 8), None, None),
                Input::new("foreground".to_string(), image_input(4, 4), None, None),
                Input::new("position x".to_string(), Value::Integer(0), None, None),
                Input::new("position y".to_string(), Value::Integer(0), None, None),
            ];
            let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blit_settings() {
            let s = OpImageCombineBlit::settings();
            assert_eq!(s.name, "blit");
            assert_eq!(OpImageCombineBlit::create_inputs().len(), 4);
            assert_eq!(OpImageCombineBlit::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_blend() {
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
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_all_color_spaces() {
            let spaces = [
                ColorSpace::Srgb, ColorSpace::RgbLinear, ColorSpace::Hsl,
                ColorSpace::Hsv, ColorSpace::Lch, ColorSpace::Xyz,
                ColorSpace::Lab, ColorSpace::Yuv, ColorSpace::Cmyk,
            ];
            for cs in spaces {
                let mut inputs = vec![
                    Input::new("background".to_string(), image_input(2, 2), None, None),
                    Input::new("foreground".to_string(), image_input(2, 2), None, None),
                    Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
                    Input::new("alpha".to_string(), image_input(2, 2), None, None),
                    Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Normal), None, None),
                    Input::new("color space".to_string(), Value::ColorSpace(cs), None, None),
                    Input::new("position x".to_string(), Value::Integer(0), None, None),
                    Input::new("position y".to_string(), Value::Integer(0), None, None),
                ];
                let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
                assert_image!(result.responses[0].value);
            }
        }

        #[tokio::test]
        async fn test_blend_with_offset() {
            let mut inputs = vec![
                Input::new("background".to_string(), image_input(8, 8), None, None),
                Input::new("foreground".to_string(), image_input(4, 4), None, None),
                Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
                Input::new("alpha".to_string(), image_input(8, 8), None, None),
                Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Lerp), None, None),
                Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None),
                Input::new("position x".to_string(), Value::Integer(2), None, None),
                Input::new("position y".to_string(), Value::Integer(2), None, None),
            ];
            let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_blend_settings() {
            let s = OpImageCombineBlend::settings();
            assert_eq!(s.name, "blend");
            assert_eq!(OpImageCombineBlend::create_inputs().len(), 8);
            assert_eq!(OpImageCombineBlend::create_outputs().len(), 1);
        }
    }

    // ==================== IMAGE NOISE ====================

    mod image_noise {
        use super::*;
        use crate::operations::images::noise::perlin::OpImageNoisePerlin;
        use crate::operations::images::noise::billow::OpImageNoiseBillow;
        use crate::operations::images::noise::cylinders::OpImageNoiseCylinders;
        use crate::operations::images::noise::fbm::OpImageNoiseFbm;
        use crate::operations::images::noise::heterogenous_multifractal::OpImageNoiseHeterogenousMultifractalNoise;
        use crate::operations::images::noise::hybrid_multifractal::OpImageNoiseHybridMultifractalNoise;
        use crate::operations::images::noise::open_simplex::OpImageNoiseOpenSimplex;
        use crate::operations::images::noise::perlin_surflet::OpImageNoisePerlinSurflet;
        use crate::operations::images::noise::ridged_multifractal::OpImageNoiseRidgedMultifractalNoise;
        use crate::operations::images::noise::simplex::OpImageNoiseSimplex;
        use crate::operations::images::noise::super_simplex::OpImageNoiseSuperSimplex;
        use crate::operations::images::noise::value::OpImageNoiseValue;
        use crate::operations::images::noise::worley_distance::OpImageNoiseWorleyDistance;
        use crate::operations::images::noise::worley_value::OpImageNoiseWorleyValue;
        use crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction;

        fn simple_noise_inputs() -> Vec<Input> {
            vec![
                Input::new("seed".to_string(), Value::Integer(1), None, None),
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("scale".to_string(), Value::Decimal(5.0), None, None),
            ]
        }

        fn multifractal_noise_inputs() -> Vec<Input> {
            vec![
                Input::new("seed".to_string(), Value::Integer(1), None, None),
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("octaves".to_string(), Value::Integer(4), None, None),
                Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
                Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
                Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
            ]
        }

        #[tokio::test]
        async fn test_perlin() {
            let mut inputs = simple_noise_inputs();
            let result = OpImageNoisePerlin::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_perlin_deterministic() {
            let mut inputs1 = simple_noise_inputs();
            let mut inputs2 = simple_noise_inputs();
            let r1 = OpImageNoisePerlin::run(&mut inputs1).await.unwrap();
            let r2 = OpImageNoisePerlin::run(&mut inputs2).await.unwrap();
            let img1 = assert_image!(r1.responses[0].value);
            let img2 = assert_image!(r2.responses[0].value);
            assert_eq!(img1.to_bytes(), img2.to_bytes());
        }

        #[tokio::test]
        async fn test_perlin_settings() {
            let s = OpImageNoisePerlin::settings();
            assert_eq!(s.name, "perlin noise");
            assert_eq!(OpImageNoisePerlin::create_inputs().len(), 4);
            assert_eq!(OpImageNoisePerlin::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_billow() {
            let mut inputs = multifractal_noise_inputs();
            let result = OpImageNoiseBillow::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_billow_settings() {
            let s = OpImageNoiseBillow::settings();
            assert_eq!(s.name, "billow noise");
            assert_eq!(OpImageNoiseBillow::create_inputs().len(), 7);
            assert_eq!(OpImageNoiseBillow::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_cylinders() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageNoiseCylinders::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_cylinders_settings() {
            let s = OpImageNoiseCylinders::settings();
            assert_eq!(s.name, "cylinders noise");
            assert_eq!(OpImageNoiseCylinders::create_inputs().len(), 3);
            assert_eq!(OpImageNoiseCylinders::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_fbm() {
            let mut inputs = multifractal_noise_inputs();
            let result = OpImageNoiseFbm::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_fbm_settings() {
            let s = OpImageNoiseFbm::settings();
            assert_eq!(s.name, "fbm noise");
            assert_eq!(OpImageNoiseFbm::create_inputs().len(), 7);
            assert_eq!(OpImageNoiseFbm::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_heterogenous_multifractal() {
            let mut inputs = multifractal_noise_inputs();
            let result = OpImageNoiseHeterogenousMultifractalNoise::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_heterogenous_multifractal_settings() {
            let s = OpImageNoiseHeterogenousMultifractalNoise::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseHeterogenousMultifractalNoise::create_inputs().len(), 7);
            assert_eq!(OpImageNoiseHeterogenousMultifractalNoise::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_hybrid_multifractal() {
            let mut inputs = multifractal_noise_inputs();
            let result = OpImageNoiseHybridMultifractalNoise::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_hybrid_multifractal_settings() {
            let s = OpImageNoiseHybridMultifractalNoise::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseHybridMultifractalNoise::create_inputs().len(), 7);
            assert_eq!(OpImageNoiseHybridMultifractalNoise::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_open_simplex() {
            let mut inputs = simple_noise_inputs();
            let result = OpImageNoiseOpenSimplex::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_open_simplex_settings() {
            let s = OpImageNoiseOpenSimplex::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseOpenSimplex::create_inputs().len(), 4);
            assert_eq!(OpImageNoiseOpenSimplex::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_simplex() {
            let mut inputs = simple_noise_inputs();
            let result = OpImageNoiseSimplex::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_simplex_settings() {
            let s = OpImageNoiseSimplex::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseSimplex::create_inputs().len(), 4);
            assert_eq!(OpImageNoiseSimplex::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_super_simplex() {
            let mut inputs = simple_noise_inputs();
            let result = OpImageNoiseSuperSimplex::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_super_simplex_settings() {
            let s = OpImageNoiseSuperSimplex::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseSuperSimplex::create_inputs().len(), 4);
            assert_eq!(OpImageNoiseSuperSimplex::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_perlin_surflet() {
            let mut inputs = simple_noise_inputs();
            let result = OpImageNoisePerlinSurflet::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_perlin_surflet_settings() {
            let s = OpImageNoisePerlinSurflet::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoisePerlinSurflet::create_inputs().len(), 4);
            assert_eq!(OpImageNoisePerlinSurflet::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_ridged_multifractal() {
            let mut inputs = multifractal_noise_inputs();
            inputs.push(Input::new("attenuation".to_string(), Value::Decimal(2.0), None, None));
            let result = OpImageNoiseRidgedMultifractalNoise::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_ridged_multifractal_settings() {
            let s = OpImageNoiseRidgedMultifractalNoise::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseRidgedMultifractalNoise::create_inputs().len(), 8);
            assert_eq!(OpImageNoiseRidgedMultifractalNoise::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_value_noise() {
            let mut inputs = simple_noise_inputs();
            let result = OpImageNoiseValue::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_value_noise_settings() {
            let s = OpImageNoiseValue::settings();
            assert!(!s.name.is_empty());
            assert_eq!(OpImageNoiseValue::create_inputs().len(), 4);
            assert_eq!(OpImageNoiseValue::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_worley_distance() {
            let mut inputs = vec![
                Input::new("seed".to_string(), Value::Integer(1), None, None),
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
                Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageNoiseWorleyDistance::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_worley_distance_all_functions() {
            let funcs = [
                NoiseWorleyDistanceFunction::Chebyshev,
                NoiseWorleyDistanceFunction::Euclidean,
                NoiseWorleyDistanceFunction::EuclideanSquared,
                NoiseWorleyDistanceFunction::Manhattan,
                NoiseWorleyDistanceFunction::Quadratic,
            ];
            for func in funcs {
                let mut inputs = vec![
                    Input::new("seed".to_string(), Value::Integer(1), None, None),
                    Input::new("width".to_string(), Value::Integer(4), None, None),
                    Input::new("height".to_string(), Value::Integer(4), None, None),
                    Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(func), None, None),
                    Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
                ];
                let result = OpImageNoiseWorleyDistance::run(&mut inputs).await.unwrap();
                assert_image!(result.responses[0].value);
            }
        }

        #[tokio::test]
        async fn test_worley_distance_settings() {
            let s = OpImageNoiseWorleyDistance::settings();
            assert_eq!(s.name, "worley noise distance");
            assert_eq!(OpImageNoiseWorleyDistance::create_inputs().len(), 5);
            assert_eq!(OpImageNoiseWorleyDistance::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_worley_distance_function_types() {
            let types = NoiseWorleyDistanceFunction::types();
            assert_eq!(types.len(), 5);
        }

        #[tokio::test]
        async fn test_worley_value() {
            let mut inputs = vec![
                Input::new("seed".to_string(), Value::Integer(1), None, None),
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
                Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageNoiseWorleyValue::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_worley_value_settings() {
            let s = OpImageNoiseWorleyValue::settings();
            assert_eq!(s.name, "worley noise value");
            assert_eq!(OpImageNoiseWorleyValue::create_inputs().len(), 5);
            assert_eq!(OpImageNoiseWorleyValue::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_quadratic_distance_fn() {
            use crate::operations::images::noise::worley_distance::quadratic_distance;
            let d = quadratic_distance(&[1.0, 2.0], &[3.0, 4.0]);
            assert!(d.is_finite());
        }
    }

    // ==================== IMAGE CHANNELS ====================

    mod image_channels {
        use super::*;
        use crate::operations::images::channels::split::OpImageChannelSplit;
        use crate::operations::images::channels::merge::OpImageChannelMerge;
        use crate::operations::images::channels::shuffle::OpImageChannelShuffle;

        #[tokio::test]
        async fn test_split_settings() {
            let s = OpImageChannelSplit::settings();
            assert_eq!(s.name, "channel split");
            assert_eq!(OpImageChannelSplit::create_inputs().len(), 1);
            assert_eq!(OpImageChannelSplit::create_outputs().len(), 4);
        }

        #[tokio::test]
        async fn test_split_produces_four_outputs() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 4);
            for i in 0..4 {
                let img = assert_image!(result.responses[i].value);
                assert_eq!(img.width(), 4);
                assert_eq!(img.height(), 4);
            }
        }

        #[tokio::test]
        async fn test_split_channel_values() {
            // Create a 1x1 image with known RGBA values
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([100, 150, 200, 250]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            ];
            let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();

            // Red channel output should have R=100 in all RGB channels
            let red_img = assert_image!(result.responses[0].value);
            let red_pixel = red_img.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(red_pixel[0], 100);
            assert_eq!(red_pixel[1], 100);
            assert_eq!(red_pixel[2], 100);

            // Green channel output should have G=150
            let green_img = assert_image!(result.responses[1].value);
            let green_pixel = green_img.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(green_pixel[0], 150);

            // Blue channel output should have B=200
            let blue_img = assert_image!(result.responses[2].value);
            let blue_pixel = blue_img.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(blue_pixel[0], 200);

            // Alpha channel output should have A=250
            let alpha_img = assert_image!(result.responses[3].value);
            let alpha_pixel = alpha_img.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(alpha_pixel[0], 250);
        }

        #[tokio::test]
        async fn test_merge_settings() {
            let s = OpImageChannelMerge::settings();
            assert_eq!(s.name, "channel merge");
            assert_eq!(OpImageChannelMerge::create_inputs().len(), 4);
            assert_eq!(OpImageChannelMerge::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_merge_produces_image() {
            let mut inputs = vec![
                Input::new("red".to_string(), image_input(4, 4), None, None),
                Input::new("green".to_string(), image_input(4, 4), None, None),
                Input::new("blue".to_string(), image_input(4, 4), None, None),
                Input::new("alpha".to_string(), image_input(4, 4), None, None),
            ];
            let result = OpImageChannelMerge::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 1);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 4);
            assert_eq!(img.height(), 4);
        }

        #[tokio::test]
        async fn test_split_merge_roundtrip() {
            // Create a known image
            let mut imgbuf = image::RgbaImage::new(2, 2);
            imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 255]));
            imgbuf.put_pixel(1, 0, image::Rgba([40, 50, 60, 255]));
            imgbuf.put_pixel(0, 1, image::Rgba([70, 80, 90, 255]));
            imgbuf.put_pixel(1, 1, image::Rgba([100, 110, 120, 255]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            // Split
            let mut split_inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            ];
            let split_result = OpImageChannelSplit::run(&mut split_inputs).await.unwrap();

            // Merge back
            let mut merge_inputs = vec![
                Input::new("red".to_string(), split_result.responses[0].value.clone(), None, None),
                Input::new("green".to_string(), split_result.responses[1].value.clone(), None, None),
                Input::new("blue".to_string(), split_result.responses[2].value.clone(), None, None),
                Input::new("alpha".to_string(), split_result.responses[3].value.clone(), None, None),
            ];
            let merge_result = OpImageChannelMerge::run(&mut merge_inputs).await.unwrap();
            let merged_img = assert_image!(merge_result.responses[0].value);
            let merged_rgba = merged_img.to_rgba8();

            // Check roundtrip preserves values
            let p = merged_rgba.get_pixel(0, 0).0;
            assert_eq!(p[0], 10);
            assert_eq!(p[1], 20);
            assert_eq!(p[2], 30);
            assert_eq!(p[3], 255);
        }

        #[tokio::test]
        async fn test_shuffle_settings() {
            let s = OpImageChannelShuffle::settings();
            assert_eq!(s.name, "channel shuffle");
            assert_eq!(OpImageChannelShuffle::create_inputs().len(), 5);
            assert_eq!(OpImageChannelShuffle::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_shuffle_identity() {
            // Default mapping (0,1,2,3) should be identity
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("red source".to_string(), Value::Integer(0), None, None),
                Input::new("green source".to_string(), Value::Integer(1), None, None),
                Input::new("blue source".to_string(), Value::Integer(2), None, None),
                Input::new("alpha source".to_string(), Value::Integer(3), None, None),
            ];
            let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
            let out_img = assert_image!(result.responses[0].value);
            let p = out_img.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p, [10, 20, 30, 40]);
        }

        #[tokio::test]
        async fn test_shuffle_swap_red_blue() {
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("red source".to_string(), Value::Integer(2), None, None),   // B -> R
                Input::new("green source".to_string(), Value::Integer(1), None, None), // G -> G
                Input::new("blue source".to_string(), Value::Integer(0), None, None),  // R -> B
                Input::new("alpha source".to_string(), Value::Integer(3), None, None), // A -> A
            ];
            let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
            let out_img = assert_image!(result.responses[0].value);
            let p = out_img.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p, [30, 20, 10, 40]);
        }
    }

    // ==================== IMAGE ADJUSTMENTS (new) ====================

    mod image_adjustments_new {
        use super::*;
        use crate::operations::images::adjustments::levels::OpImageAdjustmentLevels;
        use crate::operations::images::adjustments::curves::OpImageAdjustmentCurves;
        use crate::operations::images::adjustments::gradient_map::OpImageAdjustmentGradientMap;

        #[tokio::test]
        async fn test_levels_settings() {
            let s = OpImageAdjustmentLevels::settings();
            assert_eq!(s.name, "levels");
            assert_eq!(OpImageAdjustmentLevels::create_inputs().len(), 4);
            assert_eq!(OpImageAdjustmentLevels::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_levels_identity() {
            // black=0, white=1, gamma=1 should be identity
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("black point".to_string(), Value::Decimal(0.0), None, None),
                Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
                Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_levels_crush_blacks() {
            // Raising black point should crush dark values to 0
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([64, 64, 64, 255])); // ~0.25 intensity
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("black point".to_string(), Value::Decimal(0.5), None, None),
                Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
                Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
            let out = assert_image!(result.responses[0].value);
            let p = out.to_rgba8().get_pixel(0, 0).0;
            // 0.25 is below black point 0.5, should clamp to 0
            assert_eq!(p[0], 0);
        }

        #[tokio::test]
        async fn test_levels_gamma() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("black point".to_string(), Value::Decimal(0.0), None, None),
                Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
                Input::new("gamma".to_string(), Value::Decimal(2.2), None, None),
            ];
            let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_curves_settings() {
            let s = OpImageAdjustmentCurves::settings();
            assert_eq!(s.name, "curves");
            assert_eq!(OpImageAdjustmentCurves::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentCurves::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_curves_zero_strength_identity() {
            // strength=0 should not change the image
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([128, 128, 128, 255]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("strength".to_string(), Value::Decimal(0.0), None, None),
                Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
            let out = assert_image!(result.responses[0].value);
            let p = out.to_rgba8().get_pixel(0, 0).0;
            // Should be approximately unchanged
            assert!((p[0] as i32 - 128).abs() <= 1);
        }

        #[tokio::test]
        async fn test_curves_positive_strength() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("strength".to_string(), Value::Decimal(0.5), None, None),
                Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_curves_negative_strength() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("strength".to_string(), Value::Decimal(-0.5), None, None),
                Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_gradient_map_settings() {
            let s = OpImageAdjustmentGradientMap::settings();
            assert_eq!(s.name, "gradient map");
            assert_eq!(OpImageAdjustmentGradientMap::create_inputs().len(), 6);
            assert_eq!(OpImageAdjustmentGradientMap::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_gradient_map_two_color() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
                Input::new("use mid color".to_string(), Value::Bool(false), None, None),
                Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_gradient_map_three_color() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
                Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.0, 1.0, 0.0, 1.0)), None, None),
                Input::new("use mid color".to_string(), Value::Bool(true), None, None),
                Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_gradient_map_black_maps_to_color_a() {
            // Pure black pixel should map to color_a
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
                Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
                Input::new("use mid color".to_string(), Value::Bool(false), None, None),
                Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
            let out = assert_image!(result.responses[0].value);
            let p = out.to_rgba8().get_pixel(0, 0).0;
            // Should be close to red (color_a)
            assert!(p[0] > 200); // red channel high
            assert!(p[2] < 50);  // blue channel low
        }

        #[tokio::test]
        async fn test_gradient_map_white_maps_to_color_b() {
            // Pure white pixel should map to color_b
            let mut imgbuf = image::RgbaImage::new(1, 1);
            imgbuf.put_pixel(0, 0, image::Rgba([255, 255, 255, 255]));
            let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));

            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
                Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
                Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
                Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
                Input::new("use mid color".to_string(), Value::Bool(false), None, None),
                Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
            let out = assert_image!(result.responses[0].value);
            let p = out.to_rgba8().get_pixel(0, 0).0;
            // Should be close to blue (color_b)
            assert!(p[0] < 50);  // red channel low
            assert!(p[2] > 200); // blue channel high
        }
    }

    // ==================== PHASE 2: DISTORTION & TILING ====================

    mod image_distortion_tiling {
        use super::*;
        use crate::operations::images::transform::warp::OpImageTransformWarp;
        use crate::operations::images::transform::directional_warp::OpImageTransformDirectionalWarp;
        use crate::operations::images::transform::safe_transform::OpImageTransformSafeTransform;
        use crate::operations::images::transform::make_tile::OpImageTransformMakeTile;
        use crate::operations::images::transform::mirror::OpImageTransformMirror;

        // Helper: create a grayscale gradient displacement map (left=black, right=white)
        fn gradient_h_image(w: u32, h: u32) -> Value {
            let mut imgbuf = image::RgbaImage::new(w, h);
            for (x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
                let v = (x * 255 / w.max(1)) as u8;
                *pixel = image::Rgba([v, v, v, 255]);
            }
            Value::DynamicImage {
                data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
                change_id: get_id(),
            }
        }

        // Helper: create a uniform gray image (127,127,127)
        fn uniform_gray_image(w: u32, h: u32) -> Value {
            let mut imgbuf = image::RgbaImage::new(w, h);
            for (_x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
                *pixel = image::Rgba([127, 127, 127, 255]);
            }
            Value::DynamicImage {
                data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
                change_id: get_id(),
            }
        }

        // Helper: solid color image
        fn solid_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> Value {
            let mut imgbuf = image::RgbaImage::new(w, h);
            for (_x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
                *pixel = image::Rgba([r, g, b, a]);
            }
            Value::DynamicImage {
                data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
                change_id: get_id(),
            }
        }

        // ==================== WARP ====================

        #[tokio::test]
        async fn test_warp_settings() {
            let s = OpImageTransformWarp::settings();
            assert_eq!(s.name, "warp");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageTransformWarp::create_inputs().len(), 3);
            assert_eq!(OpImageTransformWarp::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_warp_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("displacement".to_string(), gradient_h_image(16, 16), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 1);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_warp_zero_intensity() {
            // With zero intensity, output should match input
            let src = image_input(8, 8);
            let mut inputs = vec![
                Input::new("image".to_string(), src.clone(), None, None),
                Input::new("displacement".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_warp_neutral_displacement() {
            // Neutral displacement (128,128) at any intensity should barely move pixels
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("displacement".to_string(), uniform_gray_image(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
        }

        #[tokio::test]
        async fn test_warp_different_displacement_size() {
            // Displacement map size differs from source
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("displacement".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_warp_high_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("displacement".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(200.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_warp_negative_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("displacement".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(-10.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_warp_1x1_image() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(1, 1), None, None),
                Input::new("displacement".to_string(), gradient_h_image(1, 1), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
            assert_eq!(img.height(), 1);
        }

        #[tokio::test]
        async fn test_warp_preserves_dimensions() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(32, 16), None, None),
                Input::new("displacement".to_string(), gradient_h_image(32, 16), None, None),
                Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
            assert_eq!(img.height(), 16);
        }

        // ==================== DIRECTIONAL WARP ====================

        #[tokio::test]
        async fn test_directional_warp_settings() {
            let s = OpImageTransformDirectionalWarp::settings();
            assert_eq!(s.name, "directional warp");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageTransformDirectionalWarp::create_inputs().len(), 4);
            assert_eq!(OpImageTransformDirectionalWarp::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_directional_warp_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(16, 16), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 1);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_directional_warp_zero_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
        }

        #[tokio::test]
        async fn test_directional_warp_90_degrees() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(90.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_directional_warp_180_degrees() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(180.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_directional_warp_different_map_size() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(8, 4), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_directional_warp_neutral_map() {
            // Uniform gray map = luminance 127/255 ≈ 0.498, centered ≈ -0.002, barely moves
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity map".to_string(), uniform_gray_image(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_directional_warp_high_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(200.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_directional_warp_preserves_dimensions() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(32, 16), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(32, 16), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
            assert_eq!(img.height(), 16);
        }

        // ==================== SAFE TRANSFORM ====================

        #[tokio::test]
        async fn test_safe_transform_settings() {
            let s = OpImageTransformSafeTransform::settings();
            assert_eq!(s.name, "safe transform");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageTransformSafeTransform::create_inputs().len(), 5);
            assert_eq!(OpImageTransformSafeTransform::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_safe_transform_identity() {
            // No translate, no rotation, scale=1 should produce same image
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 1);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_safe_transform_translate() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.25), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_safe_transform_rotate() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
        }

        #[tokio::test]
        async fn test_safe_transform_scale() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(2.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_safe_transform_all_combined() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.3), None, None),
                Input::new("translate y".to_string(), Value::Decimal(-0.2), None, None),
                Input::new("rotation".to_string(), Value::Decimal(90.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.5), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
        }

        #[tokio::test]
        async fn test_safe_transform_near_zero_scale() {
            // Scale very close to 0 should be clamped, not crash
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_safe_transform_negative_scale() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(-1.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_safe_transform_full_rotation() {
            // 360 degrees should effectively be identity
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(360.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_safe_transform_preserves_dimensions() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(32, 16), None, None),
                Input::new("translate x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("translate y".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.5), None, None),
            ];
            let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
            assert_eq!(img.height(), 16);
        }

        // ==================== MAKE TILE ====================

        #[tokio::test]
        async fn test_make_tile_settings() {
            let s = OpImageTransformMakeTile::settings();
            assert_eq!(s.name, "make tile");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageTransformMakeTile::create_inputs().len(), 2);
            assert_eq!(OpImageTransformMakeTile::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_make_tile_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 1);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_make_tile_small_blend() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.01), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
        }

        #[tokio::test]
        async fn test_make_tile_max_blend() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
        }

        #[tokio::test]
        async fn test_make_tile_clamps_blend_size() {
            // blend_size > 0.5 should be clamped
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.8), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_make_tile_clamps_negative_blend() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("blend size".to_string(), Value::Decimal(-0.1), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_make_tile_solid_color_unchanged() {
            // A solid-color image should stay solid after tiling
            let mut inputs = vec![
                Input::new("image".to_string(), solid_image(16, 16, 100, 100, 100, 255), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            // Check a few pixels - solid color should remain solid
            let p = rgba.get_pixel(0, 0).0;
            assert_eq!(p, [100, 100, 100, 255]);
            let p = rgba.get_pixel(8, 8).0;
            assert_eq!(p, [100, 100, 100, 255]);
        }

        #[tokio::test]
        async fn test_make_tile_preserves_dimensions() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(32, 16), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_make_tile_1x1_image() {
            // Tiny image where blend region is 0 pixels
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(1, 1), None, None),
                Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
            ];
            let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }

        // ==================== MIRROR ====================

        #[tokio::test]
        async fn test_mirror_settings() {
            let s = OpImageTransformMirror::settings();
            assert_eq!(s.name, "mirror");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageTransformMirror::create_inputs().len(), 5);
            assert_eq!(OpImageTransformMirror::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_mirror_x_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(false), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            assert_eq!(result.responses.len(), 1);
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_mirror_y_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("mirror x".to_string(), Value::Bool(false), None, None),
                Input::new("mirror y".to_string(), Value::Bool(true), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
        }

        #[tokio::test]
        async fn test_mirror_both_axes() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(16, 16), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(true), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 16);
        }

        #[tokio::test]
        async fn test_mirror_neither_axis() {
            // Neither axis mirrored should be identity
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("mirror x".to_string(), Value::Bool(false), None, None),
                Input::new("mirror y".to_string(), Value::Bool(false), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
        }

        #[tokio::test]
        async fn test_mirror_x_symmetry() {
            // With mirror_x at offset 0.5, left half should equal reversed right half
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(false), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            // Pixel at x=3 (left of center) should match pixel at x=4 (right of center, mirrored)
            let left = rgba.get_pixel(3, 0).0;
            let right = rgba.get_pixel(4, 0).0;
            assert_eq!(left, right);
        }

        #[tokio::test]
        async fn test_mirror_offset_at_edge() {
            // offset_x=0.0 means split at left edge, so entire image is mirrored (all pixels map to x=0)
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(false), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_mirror_offset_at_right_edge() {
            // offset_x=1.0 means split at right edge, nothing to mirror
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(false), None, None),
                Input::new("offset x".to_string(), Value::Decimal(1.0), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_mirror_preserves_dimensions() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(32, 16), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(true), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
            assert_eq!(img.height(), 16);
        }

        #[tokio::test]
        async fn test_mirror_1x1_image() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(1, 1), None, None),
                Input::new("mirror x".to_string(), Value::Bool(true), None, None),
                Input::new("mirror y".to_string(), Value::Bool(true), None, None),
                Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }

        // ==================== BILINEAR SAMPLING ====================

        #[tokio::test]
        async fn test_bilinear_sample_exact_pixel() {
            use crate::operations::images::transform::warp::bilinear_sample_rgba;
            let mut img = image::RgbaImage::new(4, 4);
            img.put_pixel(2, 1, image::Rgba([255, 0, 0, 255]));
            let result = bilinear_sample_rgba(&img, 2.0, 1.0);
            assert_eq!(result, [255, 0, 0, 255]);
        }

        #[tokio::test]
        async fn test_bilinear_sample_interpolated() {
            use crate::operations::images::transform::warp::bilinear_sample_rgba;
            let mut img = image::RgbaImage::new(2, 1);
            img.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
            img.put_pixel(1, 0, image::Rgba([255, 255, 255, 255]));
            let result = bilinear_sample_rgba(&img, 0.5, 0.0);
            // Should be roughly midpoint
            assert!(result[0] > 100 && result[0] < 200);
        }

        #[tokio::test]
        async fn test_bilinear_sample_out_of_bounds() {
            use crate::operations::images::transform::warp::bilinear_sample_rgba;
            let mut img = image::RgbaImage::new(4, 4);
            img.put_pixel(0, 0, image::Rgba([100, 100, 100, 255]));
            // Negative coords should clamp to edge
            let result = bilinear_sample_rgba(&img, -5.0, -5.0);
            assert_eq!(result, [100, 100, 100, 255]);
        }

        #[tokio::test]
        async fn test_bilinear_sample_large_coords() {
            use crate::operations::images::transform::warp::bilinear_sample_rgba;
            let mut img = image::RgbaImage::new(4, 4);
            img.put_pixel(3, 3, image::Rgba([200, 200, 200, 255]));
            // Large coords should clamp to edge
            let result = bilinear_sample_rgba(&img, 100.0, 100.0);
            assert_eq!(result, [200, 200, 200, 255]);
        }
    }

    // ==================== OPERATION LIST & MOD ====================

    mod operation_list {
        use crate::operations::{operation_list, default_image, OperationListItem};

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
    }
}
