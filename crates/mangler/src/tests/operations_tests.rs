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

    // ==================== NUMBER ARITHMETIC ====================

    mod number_arithmetic {
        use super::*;
        use crate::operations::numbers::arithmetic::subtract::OpNumberMathSubtract;
        use crate::operations::numbers::arithmetic::multiply::OpNumberMathMultiply;
        use crate::operations::numbers::arithmetic::divide::OpNumberMathDivide;
        use crate::operations::numbers::arithmetic::increment::OpNumberMathIncrement;
        use crate::operations::numbers::arithmetic::decrement::OpNumberMathDecrement;
        use crate::operations::numbers::arithmetic::max::OpNumberMathMax;
        use crate::operations::numbers::arithmetic::min::OpNumberMathMin;
        use crate::operations::numbers::arithmetic::clamp::OpNumberMathClamp;
        use crate::operations::numbers::arithmetic::modulus::OpNumberMathModulus;
        use crate::operations::numbers::arithmetic::round::OpNumberMathRound;
        use crate::operations::numbers::arithmetic::sign::OpNumberMathSign;
        use crate::operations::numbers::arithmetic::rand::OpNumberMathRand;

        #[tokio::test]
        async fn test_subtract_settings() {
            let s = OpNumberMathSubtract::settings();
            assert_eq!(s.name, "subtract");
            assert_eq!(OpNumberMathSubtract::create_inputs().len(), 2);
            assert_eq!(OpNumberMathSubtract::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_subtract_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(10.0), None, None),
                Input::new("b".to_string(), Value::Decimal(3.0), None, None),
            ];
            let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 7.0);
        }

        #[tokio::test]
        async fn test_subtract_negative_result() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(3.0), None, None),
                Input::new("b".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, -7.0);
        }

        #[tokio::test]
        async fn test_multiply_settings() {
            let s = OpNumberMathMultiply::settings();
            assert_eq!(s.name, "multiply");
            assert_eq!(OpNumberMathMultiply::create_inputs().len(), 2);
            assert_eq!(OpNumberMathMultiply::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_multiply_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(4.0), None, None),
                Input::new("b".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 20.0);
        }

        #[tokio::test]
        async fn test_multiply_by_zero() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(100.0), None, None),
                Input::new("b".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_divide_settings() {
            let s = OpNumberMathDivide::settings();
            assert_eq!(s.name, "divide");
            assert_eq!(OpNumberMathDivide::create_inputs().len(), 2);
            assert_eq!(OpNumberMathDivide::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_divide_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(20.0), None, None),
                Input::new("b".to_string(), Value::Decimal(4.0), None, None),
            ];
            let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 5.0);
        }

        #[tokio::test]
        async fn test_increment_settings() {
            let s = OpNumberMathIncrement::settings();
            assert_eq!(s.name, "increment");
            assert_eq!(OpNumberMathIncrement::create_inputs().len(), 1);
            assert_eq!(OpNumberMathIncrement::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_increment_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 6.0);
        }

        #[tokio::test]
        async fn test_decrement_settings() {
            let s = OpNumberMathDecrement::settings();
            assert_eq!(s.name, "decrement");
            assert_eq!(OpNumberMathDecrement::create_inputs().len(), 1);
            assert_eq!(OpNumberMathDecrement::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_decrement_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 4.0);
        }

        #[tokio::test]
        async fn test_max_settings() {
            let s = OpNumberMathMax::settings();
            assert_eq!(s.name, "max");
            assert_eq!(OpNumberMathMax::create_inputs().len(), 2);
            assert_eq!(OpNumberMathMax::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_max_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(3.0), None, None),
                Input::new("b".to_string(), Value::Decimal(7.0), None, None),
            ];
            let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 7.0);
        }

        #[tokio::test]
        async fn test_max_equal() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(5.0), None, None),
                Input::new("b".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 5.0);
        }

        #[tokio::test]
        async fn test_min_settings() {
            let s = OpNumberMathMin::settings();
            assert_eq!(s.name, "min");
            assert_eq!(OpNumberMathMin::create_inputs().len(), 2);
            assert_eq!(OpNumberMathMin::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_min_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(3.0), None, None),
                Input::new("b".to_string(), Value::Decimal(7.0), None, None),
            ];
            let result = OpNumberMathMin::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.0);
        }

        #[tokio::test]
        async fn test_clamp_settings() {
            let s = OpNumberMathClamp::settings();
            assert_eq!(s.name, "clamp");
            assert_eq!(OpNumberMathClamp::create_inputs().len(), 3);
            assert_eq!(OpNumberMathClamp::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_clamp_within_range() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(5.0), None, None),
                Input::new("min".to_string(), Value::Decimal(0.0), None, None),
                Input::new("max".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 5.0);
        }

        #[tokio::test]
        async fn test_clamp_below_min() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(-5.0), None, None),
                Input::new("min".to_string(), Value::Decimal(0.0), None, None),
                Input::new("max".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_clamp_above_max() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(15.0), None, None),
                Input::new("min".to_string(), Value::Decimal(0.0), None, None),
                Input::new("max".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 10.0);
        }

        #[tokio::test]
        async fn test_modulus_settings() {
            let s = OpNumberMathModulus::settings();
            assert_eq!(s.name, "modulus");
            assert_eq!(OpNumberMathModulus::create_inputs().len(), 2);
            assert_eq!(OpNumberMathModulus::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_modulus_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Decimal(10.0), None, None),
                Input::new("b".to_string(), Value::Decimal(3.0), None, None),
            ];
            let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 1.0);
        }

        #[tokio::test]
        async fn test_round_settings() {
            let s = OpNumberMathRound::settings();
            assert_eq!(s.name, "round");
            assert_eq!(OpNumberMathRound::create_inputs().len(), 1);
            assert_eq!(OpNumberMathRound::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_round_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(3.7), None, None),
            ];
            let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 4.0);
        }

        #[tokio::test]
        async fn test_round_down() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(3.2), None, None),
            ];
            let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.0);
        }

        #[tokio::test]
        async fn test_sign_settings() {
            let s = OpNumberMathSign::settings();
            assert_eq!(s.name, "sign");
            assert_eq!(OpNumberMathSign::create_inputs().len(), 1);
            assert_eq!(OpNumberMathSign::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_sign_positive() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 1.0);
        }

        #[tokio::test]
        async fn test_sign_negative() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(-5.0), None, None),
            ];
            let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, -1.0);
        }

        #[tokio::test]
        async fn test_sign_zero() {
            // Rust's f32::signum() returns 1.0 for +0.0
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 1.0);
        }

        #[tokio::test]
        async fn test_rand_settings() {
            let s = OpNumberMathRand::settings();
            assert_eq!(s.name, "random");
            assert_eq!(OpNumberMathRand::create_inputs().len(), 2);
            assert_eq!(OpNumberMathRand::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rand_returns_decimal() {
            let mut inputs = vec![
                Input::new("min".to_string(), Value::Decimal(0.0), None, None),
                Input::new("max".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpNumberMathRand::run(&mut inputs).await.unwrap();
            match &result.responses[0].value {
                Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 1.0, "Got {}", v),
                other => panic!("Expected Decimal, got {:?}", other),
            }
        }
    }

    // ==================== NUMBER ALGEBRA ====================

    mod number_algebra {
        use super::*;
        use crate::operations::numbers::algebra::abs::OpNumberMathAbs;
        use crate::operations::numbers::algebra::sqrt::OpNumberMathSqrt;
        use crate::operations::numbers::algebra::cbrt::OpNumberMathCbrt;
        use crate::operations::numbers::algebra::nth_root::OpNumberMathNthRt;
        use crate::operations::numbers::algebra::pow::OpNumberMathPow;
        use crate::operations::numbers::algebra::factorial::OpNumberMathFactorial;
        use crate::operations::numbers::algebra::gcd::OpNumberMathGcd;
        use crate::operations::numbers::algebra::lcm::OpNumberMathLcm;
        use crate::operations::numbers::algebra::frac::OpNumberMathFrac;
        use crate::operations::numbers::algebra::trunc::OpNumberMathTrunc;

        #[tokio::test]
        async fn test_abs_settings() {
            let s = OpNumberMathAbs::settings();
            assert_eq!(s.name, "absolute value");
            assert_eq!(OpNumberMathAbs::create_inputs().len(), 1);
            assert_eq!(OpNumberMathAbs::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_abs_negative() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(-5.0), None, None),
            ];
            let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 5.0);
        }

        #[tokio::test]
        async fn test_abs_positive() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 5.0);
        }

        #[tokio::test]
        async fn test_abs_zero() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_sqrt_settings() {
            let s = OpNumberMathSqrt::settings();
            assert_eq!(s.name, "square root");
            assert_eq!(OpNumberMathSqrt::create_inputs().len(), 1);
            assert_eq!(OpNumberMathSqrt::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_sqrt_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(9.0), None, None),
            ];
            let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.0);
        }

        #[tokio::test]
        async fn test_sqrt_zero() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_cbrt_settings() {
            let s = OpNumberMathCbrt::settings();
            assert_eq!(s.name, "cube root");
            assert_eq!(OpNumberMathCbrt::create_inputs().len(), 1);
            assert_eq!(OpNumberMathCbrt::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_cbrt_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(27.0), None, None),
            ];
            let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.0);
        }

        #[tokio::test]
        async fn test_nth_root_settings() {
            let s = OpNumberMathNthRt::settings();
            assert_eq!(s.name, "nth root");
            assert_eq!(OpNumberMathNthRt::create_inputs().len(), 2);
            assert_eq!(OpNumberMathNthRt::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_nth_root_square() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(16.0), None, None),
                Input::new("n".to_string(), Value::Decimal(2.0), None, None),
            ];
            let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 4.0);
        }

        #[tokio::test]
        async fn test_nth_root_cube() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(8.0), None, None),
                Input::new("n".to_string(), Value::Decimal(3.0), None, None),
            ];
            let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 2.0);
        }

        #[tokio::test]
        async fn test_pow_settings() {
            let s = OpNumberMathPow::settings();
            assert_eq!(s.name, "power");
            assert_eq!(OpNumberMathPow::create_inputs().len(), 2);
            assert_eq!(OpNumberMathPow::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_pow_basic() {
            let mut inputs = vec![
                Input::new("base".to_string(), Value::Decimal(2.0), None, None),
                Input::new("exponent".to_string(), Value::Decimal(3.0), None, None),
            ];
            let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 8.0);
        }

        #[tokio::test]
        async fn test_pow_zero_exponent() {
            let mut inputs = vec![
                Input::new("base".to_string(), Value::Decimal(5.0), None, None),
                Input::new("exponent".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 1.0);
        }

        #[tokio::test]
        async fn test_pow_fractional() {
            let mut inputs = vec![
                Input::new("base".to_string(), Value::Decimal(4.0), None, None),
                Input::new("exponent".to_string(), Value::Decimal(0.5), None, None),
            ];
            let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 2.0);
        }

        #[tokio::test]
        async fn test_factorial_settings() {
            let s = OpNumberMathFactorial::settings();
            assert_eq!(s.name, "factorial");
            assert_eq!(OpNumberMathFactorial::create_inputs().len(), 1);
            assert_eq!(OpNumberMathFactorial::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_factorial_5() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Integer(5), None, None),
            ];
            let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 120);
        }

        #[tokio::test]
        async fn test_factorial_0() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Integer(0), None, None),
            ];
            let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 1);
        }

        #[tokio::test]
        async fn test_factorial_1() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Integer(1), None, None),
            ];
            let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 1);
        }

        #[tokio::test]
        async fn test_gcd_settings() {
            let s = OpNumberMathGcd::settings();
            assert_eq!(s.name, "gcd");
            assert_eq!(OpNumberMathGcd::create_inputs().len(), 2);
            assert_eq!(OpNumberMathGcd::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_gcd_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Integer(12), None, None),
                Input::new("b".to_string(), Value::Integer(8), None, None),
            ];
            let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 4);
        }

        #[tokio::test]
        async fn test_gcd_coprime() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Integer(7), None, None),
                Input::new("b".to_string(), Value::Integer(13), None, None),
            ];
            let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 1);
        }

        #[tokio::test]
        async fn test_gcd_with_zero() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Integer(5), None, None),
                Input::new("b".to_string(), Value::Integer(0), None, None),
            ];
            let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 5);
        }

        #[tokio::test]
        async fn test_lcm_settings() {
            let s = OpNumberMathLcm::settings();
            assert_eq!(s.name, "lcm");
            assert_eq!(OpNumberMathLcm::create_inputs().len(), 2);
            assert_eq!(OpNumberMathLcm::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_lcm_basic() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Integer(4), None, None),
                Input::new("b".to_string(), Value::Integer(6), None, None),
            ];
            let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 12);
        }

        #[tokio::test]
        async fn test_lcm_with_zero() {
            let mut inputs = vec![
                Input::new("a".to_string(), Value::Integer(5), None, None),
                Input::new("b".to_string(), Value::Integer(0), None, None),
            ];
            let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 0);
        }

        #[tokio::test]
        async fn test_frac_settings() {
            let s = OpNumberMathFrac::settings();
            assert_eq!(s.name, "frac");
            assert_eq!(OpNumberMathFrac::create_inputs().len(), 1);
            assert_eq!(OpNumberMathFrac::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_frac_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(3.14), None, None),
            ];
            let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.14);
        }

        #[tokio::test]
        async fn test_frac_whole_number() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_trunc_settings() {
            let s = OpNumberMathTrunc::settings();
            assert_eq!(s.name, "trunc");
            assert_eq!(OpNumberMathTrunc::create_inputs().len(), 1);
            assert_eq!(OpNumberMathTrunc::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_trunc_basic() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(3.14), None, None),
            ];
            let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.0);
        }

        #[tokio::test]
        async fn test_trunc_negative() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(-3.7), None, None),
            ];
            let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, -3.0);
        }
    }

    // ==================== NUMBER CAST ====================

    mod number_cast {
        use super::*;
        use crate::operations::numbers::cast::to_decimal::OpNumberCastToDecimal;
        use crate::operations::numbers::cast::to_integer::OpNumberCastToInteger;

        #[tokio::test]
        async fn test_to_decimal_settings() {
            let s = OpNumberCastToDecimal::settings();
            assert_eq!(s.name, "to decimal");
            assert_eq!(OpNumberCastToDecimal::create_inputs().len(), 1);
            assert_eq!(OpNumberCastToDecimal::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_to_decimal_from_integer() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Integer(42), None, None),
            ];
            let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 42.0);
        }

        #[tokio::test]
        async fn test_to_decimal_passthrough() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(3.14), None, None),
            ];
            let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.14);
        }

        #[tokio::test]
        async fn test_to_integer_settings() {
            let s = OpNumberCastToInteger::settings();
            assert_eq!(s.name, "to integer");
            assert_eq!(OpNumberCastToInteger::create_inputs().len(), 1);
            assert_eq!(OpNumberCastToInteger::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_to_integer_from_decimal() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(3.7), None, None),
            ];
            let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 3);
        }

        #[tokio::test]
        async fn test_to_integer_passthrough() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Integer(42), None, None),
            ];
            let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
            assert_integer!(result.responses[0].value, 42);
        }
    }

    // ==================== NUMBER LOGARITHMIC ====================

    mod number_logarithmic {
        use super::*;
        use crate::operations::numbers::logarithmic::log::OpNumberMathLog;
        use crate::operations::numbers::logarithmic::ln::OpNumberMathLn;
        use crate::operations::numbers::logarithmic::exp::OpNumberMathExp;

        #[tokio::test]
        async fn test_log_settings() {
            let s = OpNumberMathLog::settings();
            assert_eq!(s.name, "log");
            assert_eq!(OpNumberMathLog::create_inputs().len(), 2);
            assert_eq!(OpNumberMathLog::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_log_base_10() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(100.0), None, None),
                Input::new("base".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 2.0);
        }

        #[tokio::test]
        async fn test_log_base_2() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(8.0), None, None),
                Input::new("base".to_string(), Value::Decimal(2.0), None, None),
            ];
            let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 3.0);
        }

        #[tokio::test]
        async fn test_log_invalid_input() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(-1.0), None, None),
                Input::new("base".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpNumberMathLog::run(&mut inputs).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_ln_settings() {
            let s = OpNumberMathLn::settings();
            assert_eq!(s.name, "ln");
            assert_eq!(OpNumberMathLn::create_inputs().len(), 1);
            assert_eq!(OpNumberMathLn::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_ln_e() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(std::f32::consts::E), None, None),
            ];
            let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 1.0);
        }

        #[tokio::test]
        async fn test_ln_1() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 0.0);
        }

        #[tokio::test]
        async fn test_ln_invalid() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(-1.0), None, None),
            ];
            let result = OpNumberMathLn::run(&mut inputs).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_exp_settings() {
            let s = OpNumberMathExp::settings();
            assert_eq!(s.name, "exp");
            assert_eq!(OpNumberMathExp::create_inputs().len(), 1);
            assert_eq!(OpNumberMathExp::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_exp_zero() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, 1.0);
        }

        #[tokio::test]
        async fn test_exp_one() {
            let mut inputs = vec![
                Input::new("input".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
            assert_decimal!(result.responses[0].value, std::f32::consts::E);
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
            assert_eq!(OpImageInputGradient::create_outputs().len(), 3);
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

    // ==================== PHASE 4: ADVANCED FILTERS ====================

    mod phase4_advanced_filters {
        use super::*;
        use crate::operations::images::adjustments::directional_blur::OpImageAdjustmentDirectionalBlur;
        use crate::operations::images::adjustments::radial_blur::OpImageAdjustmentRadialBlur;
        use crate::operations::images::adjustments::slope_blur::OpImageAdjustmentSlopeBlur;
        use crate::operations::images::adjustments::non_uniform_blur::OpImageAdjustmentNonUniformBlur;
        use crate::operations::images::adjustments::edge_detect::OpImageAdjustmentEdgeDetect;
        use crate::operations::images::adjustments::emboss::OpImageAdjustmentEmboss;
        use crate::operations::images::adjustments::sharpen::OpImageAdjustmentSharpen;
        use crate::operations::images::adjustments::posterize::OpImageAdjustmentPosterize;
        use crate::operations::images::adjustments::histogram_scan::OpImageAdjustmentHistogramScan;
        use crate::operations::images::adjustments::histogram_range::OpImageAdjustmentHistogramRange;
        use crate::operations::images::adjustments::auto_levels::OpImageAdjustmentAutoLevels;
        use crate::operations::images::adjustments::distance::OpImageAdjustmentDistance;

        // ==================== DIRECTIONAL BLUR ====================

        #[tokio::test]
        async fn test_directional_blur_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
                Input::new("samples".to_string(), Value::Integer(8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_directional_blur_zero_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("samples".to_string(), Value::Integer(4), None, None),
                Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_directional_blur_settings() {
            let s = OpImageAdjustmentDirectionalBlur::settings();
            assert_eq!(s.name, "directional blur");
            assert_eq!(OpImageAdjustmentDirectionalBlur::create_inputs().len(), 4);
            assert_eq!(OpImageAdjustmentDirectionalBlur::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_directional_blur_horizontal() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("samples".to_string(), Value::Integer(16), None, None),
                Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        // ==================== RADIAL BLUR ====================

        #[tokio::test]
        async fn test_radial_blur_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(10.0), None, None),
                Input::new("samples".to_string(), Value::Integer(8), None, None),
            ];
            let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_radial_blur_zero_angle() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
                Input::new("samples".to_string(), Value::Integer(4), None, None),
            ];
            let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_radial_blur_settings() {
            let s = OpImageAdjustmentRadialBlur::settings();
            assert_eq!(s.name, "radial blur");
            assert_eq!(OpImageAdjustmentRadialBlur::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentRadialBlur::create_outputs().len(), 1);
        }

        // ==================== SLOPE BLUR ====================

        #[tokio::test]
        async fn test_slope_blur_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("slope map".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
                Input::new("samples".to_string(), Value::Integer(4), None, None),
            ];
            let result = OpImageAdjustmentSlopeBlur::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_slope_blur_different_map_size() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("slope map".to_string(), image_input(4, 4), None, None),
                Input::new("intensity".to_string(), Value::Decimal(3.0), None, None),
                Input::new("samples".to_string(), Value::Integer(4), None, None),
            ];
            let result = OpImageAdjustmentSlopeBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_slope_blur_settings() {
            let s = OpImageAdjustmentSlopeBlur::settings();
            assert_eq!(s.name, "slope blur");
            assert_eq!(OpImageAdjustmentSlopeBlur::create_inputs().len(), 4);
            assert_eq!(OpImageAdjustmentSlopeBlur::create_outputs().len(), 1);
        }

        // ==================== NON-UNIFORM BLUR ====================

        #[tokio::test]
        async fn test_non_uniform_blur_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("blur map".to_string(), image_input(8, 8), None, None),
                Input::new("max intensity".to_string(), Value::Decimal(5.0), None, None),
                Input::new("samples".to_string(), Value::Integer(8), None, None),
            ];
            let result = OpImageAdjustmentNonUniformBlur::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_non_uniform_blur_zero_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("blur map".to_string(), image_input(4, 4), None, None),
                Input::new("max intensity".to_string(), Value::Decimal(0.0), None, None),
                Input::new("samples".to_string(), Value::Integer(4), None, None),
            ];
            let result = OpImageAdjustmentNonUniformBlur::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_non_uniform_blur_settings() {
            let s = OpImageAdjustmentNonUniformBlur::settings();
            assert_eq!(s.name, "non-uniform blur");
            assert_eq!(OpImageAdjustmentNonUniformBlur::create_inputs().len(), 4);
            assert_eq!(OpImageAdjustmentNonUniformBlur::create_outputs().len(), 1);
        }

        // ==================== EDGE DETECT ====================

        #[tokio::test]
        async fn test_edge_detect_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_edge_detect_high_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_edge_detect_uniform_image() {
            // Uniform image should have no edges
            let uniform = {
                let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([128, 128, 128, 255]));
                Arc::new(DynamicImage::ImageRgba8(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            // Interior pixels should be near-zero (no edges)
            let buf = img.to_rgba8();
            let p = buf.get_pixel(4, 4).0;
            assert!(p[0] < 5, "Expected near-zero edge, got {}", p[0]);
        }

        #[tokio::test]
        async fn test_edge_detect_settings() {
            let s = OpImageAdjustmentEdgeDetect::settings();
            assert_eq!(s.name, "edge detect");
            assert_eq!(OpImageAdjustmentEdgeDetect::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentEdgeDetect::create_outputs().len(), 1);
        }

        // ==================== EMBOSS ====================

        #[tokio::test]
        async fn test_emboss_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
                Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
            ];
            let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_emboss_zero_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
                Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
            ];
            let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            // Zero intensity should give ~0.5 gray everywhere
            let buf = img.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            assert!((p[0] - 0.5).abs() < 0.1, "Expected ~0.5, got {}", p[0]);
        }

        #[tokio::test]
        async fn test_emboss_settings() {
            let s = OpImageAdjustmentEmboss::settings();
            assert_eq!(s.name, "emboss");
            assert_eq!(OpImageAdjustmentEmboss::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentEmboss::create_outputs().len(), 1);
        }

        // ==================== SHARPEN ====================

        #[tokio::test]
        async fn test_sharpen_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_sharpen_zero_intensity() {
            // Zero intensity should be identity
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_sharpen_settings() {
            let s = OpImageAdjustmentSharpen::settings();
            assert_eq!(s.name, "sharpen");
            assert_eq!(OpImageAdjustmentSharpen::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentSharpen::create_outputs().len(), 1);
        }

        // ==================== POSTERIZE ====================

        #[tokio::test]
        async fn test_posterize_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("levels".to_string(), Value::Integer(4), None, None),
            ];
            let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_posterize_two_levels() {
            // 2 levels should give binary black/white
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("levels".to_string(), Value::Integer(2), None, None),
            ];
            let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            for pixel in buf.pixels() {
                for c in 0..3 {
                    assert!(pixel[c] == 0.0 || pixel[c] == 1.0, "Expected 0 or 1, got {}", pixel[c]);
                }
            }
        }

        #[tokio::test]
        async fn test_posterize_high_levels() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("levels".to_string(), Value::Integer(256), None, None),
            ];
            let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_posterize_settings() {
            let s = OpImageAdjustmentPosterize::settings();
            assert_eq!(s.name, "posterize");
            assert_eq!(OpImageAdjustmentPosterize::create_inputs().len(), 2);
            assert_eq!(OpImageAdjustmentPosterize::create_outputs().len(), 1);
        }

        // ==================== HISTOGRAM SCAN ====================

        #[tokio::test]
        async fn test_histogram_scan_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("position".to_string(), Value::Decimal(0.5), None, None),
                Input::new("range".to_string(), Value::Decimal(0.1), None, None),
            ];
            let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_histogram_scan_full_range() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(4, 4), None, None),
                Input::new("position".to_string(), Value::Decimal(0.5), None, None),
                Input::new("range".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            // Full range should give all white
            let buf = img.to_rgba32f();
            for pixel in buf.pixels() {
                assert!(pixel[0] > 0.9, "Expected near-white with full range, got {}", pixel[0]);
            }
        }

        #[tokio::test]
        async fn test_histogram_scan_settings() {
            let s = OpImageAdjustmentHistogramScan::settings();
            assert_eq!(s.name, "histogram scan");
            assert_eq!(OpImageAdjustmentHistogramScan::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentHistogramScan::create_outputs().len(), 1);
        }

        // ==================== HISTOGRAM RANGE ====================

        #[tokio::test]
        async fn test_histogram_range_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("range min".to_string(), Value::Decimal(0.0), None, None),
                Input::new("range max".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_histogram_range_compress() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("range min".to_string(), Value::Decimal(0.25), None, None),
                Input::new("range max".to_string(), Value::Decimal(0.75), None, None),
            ];
            let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            for pixel in buf.pixels() {
                for c in 0..3 {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "Pixel out of range: {}", pixel[c]);
                }
            }
        }

        #[tokio::test]
        async fn test_histogram_range_settings() {
            let s = OpImageAdjustmentHistogramRange::settings();
            assert_eq!(s.name, "histogram range");
            assert_eq!(OpImageAdjustmentHistogramRange::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentHistogramRange::create_outputs().len(), 1);
        }

        // ==================== AUTO LEVELS ====================

        #[tokio::test]
        async fn test_auto_levels_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
                Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
            ];
            let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_auto_levels_uniform_image() {
            let uniform = {
                let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([128, 128, 128, 255]));
                Arc::new(DynamicImage::ImageRgba8(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
                Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
                Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
            ];
            let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_auto_levels_settings() {
            let s = OpImageAdjustmentAutoLevels::settings();
            assert_eq!(s.name, "auto levels");
            assert_eq!(OpImageAdjustmentAutoLevels::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentAutoLevels::create_outputs().len(), 1);
        }

        // ==================== DISTANCE ====================

        #[tokio::test]
        async fn test_distance_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
                Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
            ];
            let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_distance_all_white() {
            // All white image, all "inside" -> should be > 0.5
            let white = {
                let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
                Arc::new(DynamicImage::ImageRgba8(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: white, change_id: get_id() }, None, None),
                Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
                Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
            ];
            let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            assert!(p[0] >= 0.5, "Inside pixel should be >= 0.5, got {}", p[0]);
        }

        #[tokio::test]
        async fn test_distance_all_black() {
            // All black image, all "outside" -> should be <= 0.5
            let black = {
                let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
                Arc::new(DynamicImage::ImageRgba8(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: black, change_id: get_id() }, None, None),
                Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
                Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
            ];
            let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            assert!(p[0] <= 0.5, "Outside pixel should be <= 0.5, got {}", p[0]);
        }

        #[tokio::test]
        async fn test_distance_settings() {
            let s = OpImageAdjustmentDistance::settings();
            assert_eq!(s.name, "distance");
            assert_eq!(OpImageAdjustmentDistance::create_inputs().len(), 3);
            assert_eq!(OpImageAdjustmentDistance::create_outputs().len(), 1);
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

    // ==================== SHAPES ====================

    mod shape_rectangle_tests {
        use super::*;
        use crate::operations::images::shapes::rectangle::OpImageShapeRectangle;

        #[tokio::test]
        async fn test_rectangle_settings() {
            let s = OpImageShapeRectangle::settings();
            assert_eq!(s.name, "rectangle");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageShapeRectangle::create_inputs().len(), 6);
            assert_eq!(OpImageShapeRectangle::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_rectangle_basic() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("rect width".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rect height".to_string(), Value::Decimal(0.5), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_rectangle_center_is_white() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("rect width".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rect height".to_string(), Value::Decimal(0.5), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let center = rgba.get_pixel(32, 32);
            assert!(center[0] > 200, "Center should be white, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_rectangle_corner_is_black() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("rect width".to_string(), Value::Decimal(0.3), None, None),
                Input::new("rect height".to_string(), Value::Decimal(0.3), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let corner = rgba.get_pixel(0, 0);
            assert!(corner[0] < 50, "Corner should be black, got {}", corner[0]);
        }

        #[tokio::test]
        async fn test_rectangle_with_rotation() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("rect width".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rect height".to_string(), Value::Decimal(0.5), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_rectangle_with_corner_radius() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("rect width".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rect height".to_string(), Value::Decimal(0.5), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.2), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_rectangle_full_size() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(32), None, None),
                Input::new("height".to_string(), Value::Integer(32), None, None),
                Input::new("rect width".to_string(), Value::Decimal(1.0), None, None),
                Input::new("rect height".to_string(), Value::Decimal(1.0), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            // Center should be fully white when rect fills image
            let center = rgba.get_pixel(16, 16);
            assert!(center[0] > 200, "Full-size rect center should be white, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_rectangle_1x1() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(1), None, None),
                Input::new("height".to_string(), Value::Integer(1), None, None),
                Input::new("rect width".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rect height".to_string(), Value::Decimal(0.5), None, None),
                Input::new("corner radius".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
            assert_eq!(img.height(), 1);
        }
    }

    mod shape_ellipse_tests {
        use super::*;
        use crate::operations::images::shapes::ellipse::OpImageShapeEllipse;

        #[tokio::test]
        async fn test_ellipse_settings() {
            let s = OpImageShapeEllipse::settings();
            assert_eq!(s.name, "ellipse");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageShapeEllipse::create_inputs().len(), 5);
            assert_eq!(OpImageShapeEllipse::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_ellipse_basic() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("radius x".to_string(), Value::Decimal(0.4), None, None),
                Input::new("radius y".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_ellipse_center_is_white() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("radius x".to_string(), Value::Decimal(0.4), None, None),
                Input::new("radius y".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let center = rgba.get_pixel(32, 32);
            assert!(center[0] > 200, "Center should be white, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_ellipse_corner_is_black() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("radius x".to_string(), Value::Decimal(0.3), None, None),
                Input::new("radius y".to_string(), Value::Decimal(0.3), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let corner = rgba.get_pixel(0, 0);
            assert!(corner[0] < 50, "Corner should be black, got {}", corner[0]);
        }

        #[tokio::test]
        async fn test_ellipse_with_rotation() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("radius x".to_string(), Value::Decimal(0.4), None, None),
                Input::new("radius y".to_string(), Value::Decimal(0.2), None, None),
                Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
            ];
            let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_ellipse_circle() {
            // Equal radii = circle
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("radius x".to_string(), Value::Decimal(0.4), None, None),
                Input::new("radius y".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            // Symmetric: pixel at same distance from center on x and y should be similar
            let px1 = rgba.get_pixel(42, 32); // right of center
            let px2 = rgba.get_pixel(32, 42); // below center
            assert!((px1[0] as i32 - px2[0] as i32).abs() < 10, "Circle should be symmetric");
        }

        #[tokio::test]
        async fn test_ellipse_1x1() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(1), None, None),
                Input::new("height".to_string(), Value::Integer(1), None, None),
                Input::new("radius x".to_string(), Value::Decimal(0.4), None, None),
                Input::new("radius y".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }
    }

    mod shape_polygon_tests {
        use super::*;
        use crate::operations::images::shapes::polygon::OpImageShapePolygon;

        #[tokio::test]
        async fn test_polygon_settings() {
            let s = OpImageShapePolygon::settings();
            assert_eq!(s.name, "polygon");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageShapePolygon::create_inputs().len(), 5);
            assert_eq!(OpImageShapePolygon::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_polygon_hexagon() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("sides".to_string(), Value::Integer(6), None, None),
                Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_polygon_center_is_white() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("sides".to_string(), Value::Integer(6), None, None),
                Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let center = rgba.get_pixel(32, 32);
            assert!(center[0] > 200, "Center should be white, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_polygon_corner_is_black() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("sides".to_string(), Value::Integer(5), None, None),
                Input::new("radius".to_string(), Value::Decimal(0.3), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let corner = rgba.get_pixel(0, 0);
            assert!(corner[0] < 50, "Corner should be black, got {}", corner[0]);
        }

        #[tokio::test]
        async fn test_polygon_triangle() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("sides".to_string(), Value::Integer(3), None, None),
                Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_polygon_with_rotation() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("sides".to_string(), Value::Integer(4), None, None),
                Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
            ];
            let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_polygon_many_sides_approaches_circle() {
            // A 64-sided polygon should look very similar to a circle
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("sides".to_string(), Value::Integer(64), None, None),
                Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            // Should be roughly symmetric
            let px1 = rgba.get_pixel(42, 32);
            let px2 = rgba.get_pixel(32, 42);
            assert!((px1[0] as i32 - px2[0] as i32).abs() < 20);
        }
    }

    mod shape_star_tests {
        use super::*;
        use crate::operations::images::shapes::star::OpImageShapeStar;

        #[tokio::test]
        async fn test_star_settings() {
            let s = OpImageShapeStar::settings();
            assert_eq!(s.name, "star");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageShapeStar::create_inputs().len(), 6);
            assert_eq!(OpImageShapeStar::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_star_basic() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("points".to_string(), Value::Integer(5), None, None),
                Input::new("outer radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("inner radius".to_string(), Value::Decimal(0.2), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_star_center_is_white() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("points".to_string(), Value::Integer(5), None, None),
                Input::new("outer radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("inner radius".to_string(), Value::Decimal(0.2), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let center = rgba.get_pixel(32, 32);
            assert!(center[0] > 200, "Center should be white, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_star_corner_is_black() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("points".to_string(), Value::Integer(5), None, None),
                Input::new("outer radius".to_string(), Value::Decimal(0.3), None, None),
                Input::new("inner radius".to_string(), Value::Decimal(0.15), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let corner = rgba.get_pixel(0, 0);
            assert!(corner[0] < 50, "Corner should be black, got {}", corner[0]);
        }

        #[tokio::test]
        async fn test_star_with_rotation() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("points".to_string(), Value::Integer(5), None, None),
                Input::new("outer radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("inner radius".to_string(), Value::Decimal(0.2), None, None),
                Input::new("rotation".to_string(), Value::Decimal(36.0), None, None),
            ];
            let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_star_3_points() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("points".to_string(), Value::Integer(3), None, None),
                Input::new("outer radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("inner radius".to_string(), Value::Decimal(0.15), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_star_equal_radii_is_polygon() {
            // When inner == outer, star should look like a polygon
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("points".to_string(), Value::Integer(5), None, None),
                Input::new("outer radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("inner radius".to_string(), Value::Decimal(0.4), None, None),
                Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }
    }

    mod shape_line_tests {
        use super::*;
        use crate::operations::images::shapes::line::OpImageShapeLine;

        #[tokio::test]
        async fn test_line_settings() {
            let s = OpImageShapeLine::settings();
            assert_eq!(s.name, "line");
            assert!(!s.description.is_empty());
            assert_eq!(OpImageShapeLine::create_inputs().len(), 7);
            assert_eq!(OpImageShapeLine::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_line_horizontal() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("start x".to_string(), Value::Decimal(0.1), None, None),
                Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
                Input::new("end x".to_string(), Value::Decimal(0.9), None, None),
                Input::new("end y".to_string(), Value::Decimal(0.5), None, None),
                Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
            // Center of horizontal line should be bright (may not be fully white due to AA)
            let rgba = img.to_rgba8();
            let center = rgba.get_pixel(32, 32);
            assert!(center[0] > 100, "Line center should be bright, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_line_vertical() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("start x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("start y".to_string(), Value::Decimal(0.1), None, None),
                Input::new("end x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("end y".to_string(), Value::Decimal(0.9), None, None),
                Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let center = rgba.get_pixel(32, 32);
            assert!(center[0] > 100, "Line center should be bright, got {}", center[0]);
        }

        #[tokio::test]
        async fn test_line_diagonal() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("start x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("start y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("end x".to_string(), Value::Decimal(1.0), None, None),
                Input::new("end y".to_string(), Value::Decimal(1.0), None, None),
                Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_line_corner_is_black() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("start x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
                Input::new("end x".to_string(), Value::Decimal(0.5), None, None),
                Input::new("end y".to_string(), Value::Decimal(0.6), None, None),
                Input::new("thickness".to_string(), Value::Decimal(0.02), None, None),
            ];
            let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let corner = rgba.get_pixel(0, 0);
            assert!(corner[0] < 50, "Corner should be black, got {}", corner[0]);
        }

        #[tokio::test]
        async fn test_line_thick() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("start x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
                Input::new("end x".to_string(), Value::Decimal(1.0), None, None),
                Input::new("end y".to_string(), Value::Decimal(0.5), None, None),
                Input::new("thickness".to_string(), Value::Decimal(0.2), None, None),
            ];
            let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_line_1x1() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(1), None, None),
                Input::new("height".to_string(), Value::Integer(1), None, None),
                Input::new("start x".to_string(), Value::Decimal(0.0), None, None),
                Input::new("start y".to_string(), Value::Decimal(0.0), None, None),
                Input::new("end x".to_string(), Value::Decimal(1.0), None, None),
                Input::new("end y".to_string(), Value::Decimal(1.0), None, None),
                Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }
    }

    // ==================== PATTERNS ====================

    mod pattern_brick_tests {
        use super::*;
        use crate::operations::images::patterns::brick::OpImagePatternBrick;

        #[tokio::test]
        async fn test_brick_settings() {
            let s = OpImagePatternBrick::settings();
            assert_eq!(s.name, "brick");
            assert!(!s.description.is_empty());
            assert_eq!(OpImagePatternBrick::create_inputs().len(), 6);
            assert_eq!(OpImagePatternBrick::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_brick_basic() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("columns".to_string(), Value::Integer(8), None, None),
                Input::new("rows".to_string(), Value::Integer(16), None, None),
                Input::new("offset".to_string(), Value::Decimal(0.5), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternBrick::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_brick_has_gaps() {
            // With gaps, some pixels should be dark (gap) and some bright (brick)
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("columns".to_string(), Value::Integer(4), None, None),
                Input::new("rows".to_string(), Value::Integer(4), None, None),
                Input::new("offset".to_string(), Value::Decimal(0.5), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.1), None, None),
            ];
            let result = OpImagePatternBrick::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let mut has_dark = false;
            let mut has_light = false;
            for y in 0..64 {
                for x in 0..64 {
                    let p = rgba.get_pixel(x, y)[0];
                    if p < 50 { has_dark = true; }
                    if p > 200 { has_light = true; }
                }
            }
            assert!(has_dark, "Should have dark (gap) pixels");
            assert!(has_light, "Should have light (brick) pixels");
        }

        #[tokio::test]
        async fn test_brick_no_gap() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("columns".to_string(), Value::Integer(4), None, None),
                Input::new("rows".to_string(), Value::Integer(4), None, None),
                Input::new("offset".to_string(), Value::Decimal(0.5), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImagePatternBrick::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            // With no gap, all pixels should be white
            for y in 0..64 {
                for x in 0..64 {
                    let p = rgba.get_pixel(x, y)[0];
                    assert!(p > 200, "All should be white with no gap, got {} at ({},{})", p, x, y);
                }
            }
        }

        #[tokio::test]
        async fn test_brick_zero_offset() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("columns".to_string(), Value::Integer(4), None, None),
                Input::new("rows".to_string(), Value::Integer(4), None, None),
                Input::new("offset".to_string(), Value::Decimal(0.0), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternBrick::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_brick_1x1() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(1), None, None),
                Input::new("height".to_string(), Value::Integer(1), None, None),
                Input::new("columns".to_string(), Value::Integer(4), None, None),
                Input::new("rows".to_string(), Value::Integer(4), None, None),
                Input::new("offset".to_string(), Value::Decimal(0.5), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternBrick::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }
    }

    mod pattern_hexagonal_tests {
        use super::*;
        use crate::operations::images::patterns::hexagonal::OpImagePatternHexagonal;

        #[tokio::test]
        async fn test_hexagonal_settings() {
            let s = OpImagePatternHexagonal::settings();
            assert_eq!(s.name, "hexagonal");
            assert!(!s.description.is_empty());
            assert_eq!(OpImagePatternHexagonal::create_inputs().len(), 4);
            assert_eq!(OpImagePatternHexagonal::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_hexagonal_basic() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("scale".to_string(), Value::Decimal(10.0), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternHexagonal::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_hexagonal_has_gaps() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(128), None, None),
                Input::new("height".to_string(), Value::Integer(128), None, None),
                Input::new("scale".to_string(), Value::Decimal(8.0), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.1), None, None),
            ];
            let result = OpImagePatternHexagonal::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let mut has_dark = false;
            let mut has_light = false;
            for y in 0..128 {
                for x in 0..128 {
                    let p = rgba.get_pixel(x, y)[0];
                    if p < 50 { has_dark = true; }
                    if p > 200 { has_light = true; }
                }
            }
            assert!(has_dark, "Should have dark (gap) pixels");
            assert!(has_light, "Should have light (hex) pixels");
        }

        #[tokio::test]
        async fn test_hexagonal_no_gap() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("scale".to_string(), Value::Decimal(10.0), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImagePatternHexagonal::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_hexagonal_large_scale() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("scale".to_string(), Value::Decimal(64.0), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternHexagonal::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_hexagonal_1x1() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(1), None, None),
                Input::new("height".to_string(), Value::Integer(1), None, None),
                Input::new("scale".to_string(), Value::Decimal(10.0), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternHexagonal::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }
    }

    mod pattern_weave_tests {
        use super::*;
        use crate::operations::images::patterns::weave::OpImagePatternWeave;

        #[tokio::test]
        async fn test_weave_settings() {
            let s = OpImagePatternWeave::settings();
            assert_eq!(s.name, "weave");
            assert!(!s.description.is_empty());
            assert_eq!(OpImagePatternWeave::create_inputs().len(), 4);
            assert_eq!(OpImagePatternWeave::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_weave_basic() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("count".to_string(), Value::Integer(8), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternWeave::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_weave_has_two_tones() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(128), None, None),
                Input::new("height".to_string(), Value::Integer(128), None, None),
                Input::new("count".to_string(), Value::Integer(4), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternWeave::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let rgba = img.to_rgba8();
            let mut has_dark = false;
            let mut has_mid = false;
            let mut has_light = false;
            for y in 0..128 {
                for x in 0..128 {
                    let p = rgba.get_pixel(x, y)[0];
                    if p < 50 { has_dark = true; }
                    if p > 100 && p < 180 { has_mid = true; }
                    if p > 180 { has_light = true; }
                }
            }
            assert!(has_dark, "Should have dark (gap) pixels");
            assert!(has_mid || has_light, "Should have strand pixels");
        }

        #[tokio::test]
        async fn test_weave_no_gap() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("count".to_string(), Value::Integer(4), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.0), None, None),
            ];
            let result = OpImagePatternWeave::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_weave_1x1() {
            let mut inputs = vec![
                Input::new("width".to_string(), Value::Integer(1), None, None),
                Input::new("height".to_string(), Value::Integer(1), None, None),
                Input::new("count".to_string(), Value::Integer(4), None, None),
                Input::new("gap size".to_string(), Value::Decimal(0.05), None, None),
            ];
            let result = OpImagePatternWeave::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 1);
        }
    }

    mod pattern_tile_sampler_tests {
        use super::*;
        use crate::operations::images::patterns::tile_sampler::OpImagePatternTileSampler;
        use crate::operations::default_image;

        #[tokio::test]
        async fn test_tile_sampler_settings() {
            let s = OpImagePatternTileSampler::settings();
            assert_eq!(s.name, "tile sampler");
            assert!(!s.description.is_empty());
            assert_eq!(OpImagePatternTileSampler::create_inputs().len(), 10);
            assert_eq!(OpImagePatternTileSampler::create_outputs().len(), 1);
        }

        #[tokio::test]
        async fn test_tile_sampler_basic() {
            let mut inputs = vec![
                Input::new("pattern".to_string(), image_input(16, 16), None, None),
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("count x".to_string(), Value::Integer(4), None, None),
                Input::new("count y".to_string(), Value::Integer(4), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
                Input::new("scale random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("offset random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("seed".to_string(), Value::Integer(42), None, None),
            ];
            let result = OpImagePatternTileSampler::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
            assert_eq!(img.height(), 64);
        }

        #[tokio::test]
        async fn test_tile_sampler_with_randomization() {
            let mut inputs = vec![
                Input::new("pattern".to_string(), image_input(16, 16), None, None),
                Input::new("width".to_string(), Value::Integer(64), None, None),
                Input::new("height".to_string(), Value::Integer(64), None, None),
                Input::new("count x".to_string(), Value::Integer(4), None, None),
                Input::new("count y".to_string(), Value::Integer(4), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
                Input::new("scale random".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rotation random".to_string(), Value::Decimal(90.0), None, None),
                Input::new("offset random".to_string(), Value::Decimal(0.3), None, None),
                Input::new("seed".to_string(), Value::Integer(123), None, None),
            ];
            let result = OpImagePatternTileSampler::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 64);
        }

        #[tokio::test]
        async fn test_tile_sampler_deterministic() {
            // Same seed should produce same output
            let make_inputs = || vec![
                Input::new("pattern".to_string(), image_input(8, 8), None, None),
                Input::new("width".to_string(), Value::Integer(32), None, None),
                Input::new("height".to_string(), Value::Integer(32), None, None),
                Input::new("count x".to_string(), Value::Integer(2), None, None),
                Input::new("count y".to_string(), Value::Integer(2), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
                Input::new("scale random".to_string(), Value::Decimal(0.3), None, None),
                Input::new("rotation random".to_string(), Value::Decimal(45.0), None, None),
                Input::new("offset random".to_string(), Value::Decimal(0.2), None, None),
                Input::new("seed".to_string(), Value::Integer(99), None, None),
            ];
            let mut inputs1 = make_inputs();
            let mut inputs2 = make_inputs();
            let result1 = OpImagePatternTileSampler::run(&mut inputs1).await.unwrap();
            let result2 = OpImagePatternTileSampler::run(&mut inputs2).await.unwrap();
            let img1 = assert_image!(result1.responses[0].value);
            let img2 = assert_image!(result2.responses[0].value);
            let rgba1 = img1.to_rgba8();
            let rgba2 = img2.to_rgba8();
            for y in 0..32 {
                for x in 0..32 {
                    assert_eq!(rgba1.get_pixel(x, y), rgba2.get_pixel(x, y),
                        "Same seed should give same output at ({}, {})", x, y);
                }
            }
        }

        #[tokio::test]
        async fn test_tile_sampler_different_seeds() {
            let make_inputs = |seed: i32| vec![
                Input::new("pattern".to_string(), image_input(8, 8), None, None),
                Input::new("width".to_string(), Value::Integer(32), None, None),
                Input::new("height".to_string(), Value::Integer(32), None, None),
                Input::new("count x".to_string(), Value::Integer(2), None, None),
                Input::new("count y".to_string(), Value::Integer(2), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
                Input::new("scale random".to_string(), Value::Decimal(0.5), None, None),
                Input::new("rotation random".to_string(), Value::Decimal(180.0), None, None),
                Input::new("offset random".to_string(), Value::Decimal(0.5), None, None),
                Input::new("seed".to_string(), Value::Integer(seed), None, None),
            ];
            let mut inputs1 = make_inputs(1);
            let mut inputs2 = make_inputs(999);
            let result1 = OpImagePatternTileSampler::run(&mut inputs1).await.unwrap();
            let result2 = OpImagePatternTileSampler::run(&mut inputs2).await.unwrap();
            let img1 = assert_image!(result1.responses[0].value);
            let img2 = assert_image!(result2.responses[0].value);
            let rgba1 = img1.to_rgba8();
            let rgba2 = img2.to_rgba8();
            let mut different = 0;
            for y in 0..32 {
                for x in 0..32 {
                    if rgba1.get_pixel(x, y) != rgba2.get_pixel(x, y) {
                        different += 1;
                    }
                }
            }
            assert!(different > 0, "Different seeds should produce different outputs");
        }

        #[tokio::test]
        async fn test_tile_sampler_single_cell() {
            let mut inputs = vec![
                Input::new("pattern".to_string(), image_input(16, 16), None, None),
                Input::new("width".to_string(), Value::Integer(32), None, None),
                Input::new("height".to_string(), Value::Integer(32), None, None),
                Input::new("count x".to_string(), Value::Integer(1), None, None),
                Input::new("count y".to_string(), Value::Integer(1), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
                Input::new("scale random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("offset random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("seed".to_string(), Value::Integer(42), None, None),
            ];
            let result = OpImagePatternTileSampler::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
        }

        #[tokio::test]
        async fn test_tile_sampler_default_pattern() {
            // Default 1x1 white pattern
            let mut inputs = vec![
                Input::new("pattern".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
                Input::new("width".to_string(), Value::Integer(32), None, None),
                Input::new("height".to_string(), Value::Integer(32), None, None),
                Input::new("count x".to_string(), Value::Integer(4), None, None),
                Input::new("count y".to_string(), Value::Integer(4), None, None),
                Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
                Input::new("scale random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("rotation random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("offset random".to_string(), Value::Decimal(0.0), None, None),
                Input::new("seed".to_string(), Value::Integer(42), None, None),
            ];
            let result = OpImagePatternTileSampler::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 32);
        }
    }

    // ==================== PHASE 5: PBR ====================

    mod phase5_pbr {
        use super::*;
        use crate::operations::images::pbr::normal_from_height::OpImagePbrNormalFromHeight;
        use crate::operations::images::pbr::ao_from_height::OpImagePbrAoFromHeight;
        use crate::operations::images::pbr::curvature::OpImagePbrCurvature;

        // ==================== NORMAL FROM HEIGHT ====================

        #[tokio::test]
        async fn test_normal_from_height_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_normal_from_height_flat_surface() {
            // Uniform image should produce flat normals (0.5, 0.5, 1.0)
            let uniform = {
                let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([128, 128, 128, 255]));
                Arc::new(DynamicImage::ImageRgba8(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            // Flat surface: normal = (0, 0, 1) -> mapped to (0.5, 0.5, 1.0)
            assert!((p[0] - 0.5).abs() < 0.05, "Expected ~0.5 for x, got {}", p[0]);
            assert!((p[1] - 0.5).abs() < 0.05, "Expected ~0.5 for y, got {}", p[1]);
            assert!((p[2] - 1.0).abs() < 0.05, "Expected ~1.0 for z, got {}", p[2]);
        }

        #[tokio::test]
        async fn test_normal_from_height_high_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            ];
            let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_normal_from_height_1x1() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(1, 1), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
            assert_image!(result.responses[0].value);
        }

        #[tokio::test]
        async fn test_normal_from_height_settings() {
            let s = OpImagePbrNormalFromHeight::settings();
            assert_eq!(s.name, "normal from height");
            assert_eq!(OpImagePbrNormalFromHeight::create_inputs().len(), 2);
            assert_eq!(OpImagePbrNormalFromHeight::create_outputs().len(), 1);
        }

        // ==================== AO FROM HEIGHT ====================

        #[tokio::test]
        async fn test_ao_from_height_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("radius".to_string(), Value::Integer(4), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
                Input::new("samples".to_string(), Value::Integer(8), None, None),
            ];
            let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_ao_from_height_flat_surface() {
            // Uniform height -> no occlusion -> AO should be 1.0 (white)
            let uniform = {
                let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([128, 128, 128, 255]));
                Arc::new(DynamicImage::ImageRgba8(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
                Input::new("radius".to_string(), Value::Integer(4), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
                Input::new("samples".to_string(), Value::Integer(8), None, None),
            ];
            let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            assert!((p[0] - 1.0).abs() < 0.05, "Flat surface AO should be ~1.0, got {}", p[0]);
        }

        #[tokio::test]
        async fn test_ao_from_height_high_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("radius".to_string(), Value::Integer(8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
                Input::new("samples".to_string(), Value::Integer(16), None, None),
            ];
            let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            // All pixels should be in valid range
            for pixel in buf.pixels() {
                assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0, "AO out of range: {}", pixel[0]);
            }
        }

        #[tokio::test]
        async fn test_ao_from_height_settings() {
            let s = OpImagePbrAoFromHeight::settings();
            assert_eq!(s.name, "ao from height");
            assert_eq!(OpImagePbrAoFromHeight::create_inputs().len(), 4);
            assert_eq!(OpImagePbrAoFromHeight::create_outputs().len(), 1);
        }

        // ==================== CURVATURE ====================

        #[tokio::test]
        async fn test_curvature_basic() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImagePbrCurvature::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            assert_eq!(img.width(), 8);
            assert_eq!(img.height(), 8);
        }

        #[tokio::test]
        async fn test_curvature_flat_normals() {
            // Uniform flat normals (0.5, 0.5, 1.0) -> zero curvature -> output 0.5
            let flat = {
                let mut img = image::Rgba32FImage::new(8, 8);
                for pixel in img.pixels_mut() {
                    *pixel = image::Rgba([0.5, 0.5, 1.0, 1.0]);
                }
                Arc::new(DynamicImage::ImageRgba32F(img))
            };
            let mut inputs = vec![
                Input::new("image".to_string(), Value::DynamicImage { data: flat, change_id: get_id() }, None, None),
                Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpImagePbrCurvature::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            assert!((p[0] - 0.5).abs() < 0.05, "Flat normals should give ~0.5 curvature, got {}", p[0]);
        }

        #[tokio::test]
        async fn test_curvature_high_intensity() {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImagePbrCurvature::run(&mut inputs).await.unwrap();
            let img = assert_image!(result.responses[0].value);
            let buf = img.to_rgba32f();
            for pixel in buf.pixels() {
                assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0, "Curvature out of range: {}", pixel[0]);
            }
        }

        #[tokio::test]
        async fn test_curvature_settings() {
            let s = OpImagePbrCurvature::settings();
            assert_eq!(s.name, "curvature");
            assert_eq!(OpImagePbrCurvature::create_inputs().len(), 2);
            assert_eq!(OpImagePbrCurvature::create_outputs().len(), 1);
        }
    }
}
