import os

base = "D:/rust/nodemangler/crates/mangler/src/operations"

files_and_tests = {}

files_and_tests["numbers/arithmetic/decrement.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_decrement_settings() {
        let s = OpNumberMathDecrement::settings();
        assert_eq!(s.name, "decrement");
        assert_eq!(OpNumberMathDecrement::create_inputs().len(), 1);
        assert_eq!(OpNumberMathDecrement::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_decrement_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/min.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/max.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 7.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_equal() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(5.0), None, None),
            Input::new("b".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/clamp.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_below_min() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(-5.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_above_max() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(15.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/modulus.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/round.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_round_settings() {
        let s = OpNumberMathRound::settings();
        assert_eq!(s.name, "round");
        assert_eq!(OpNumberMathRound::create_inputs().len(), 1);
        assert_eq!(OpNumberMathRound::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_round_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.7), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_down() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.2), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/sign.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_sign_settings() {
        let s = OpNumberMathSign::settings();
        assert_eq!(s.name, "sign");
        assert_eq!(OpNumberMathSign::create_inputs().len(), 1);
        assert_eq!(OpNumberMathSign::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_sign_positive() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-5.0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/arithmetic/rand.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
"""

files_and_tests["numbers/algebra/abs.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_abs_settings() {
        let s = OpNumberMathAbs::settings();
        assert_eq!(s.name, "absolute value");
        assert_eq!(OpNumberMathAbs::create_inputs().len(), 1);
        assert_eq!(OpNumberMathAbs::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_abs_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-5.0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_positive() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/sqrt.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_sqrt_settings() {
        let s = OpNumberMathSqrt::settings();
        assert_eq!(s.name, "square root");
        assert_eq!(OpNumberMathSqrt::create_inputs().len(), 1);
        assert_eq!(OpNumberMathSqrt::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_sqrt_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(9.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/cbrt.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_cbrt_settings() {
        let s = OpNumberMathCbrt::settings();
        assert_eq!(s.name, "cube root");
        assert_eq!(OpNumberMathCbrt::create_inputs().len(), 1);
        assert_eq!(OpNumberMathCbrt::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_cbrt_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(27.0), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/nth_root.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_cube() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(8.0), None, None),
            Input::new("n".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/pow.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 8.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_zero_exponent() {
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(5.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_fractional() {
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(4.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/factorial.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_factorial_settings() {
        let s = OpNumberMathFactorial::settings();
        assert_eq!(s.name, "factorial");
        assert_eq!(OpNumberMathFactorial::create_inputs().len(), 1);
        assert_eq!(OpNumberMathFactorial::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_factorial_5() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(5), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 120),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_0() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_1() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/gcd.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 4),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_gcd_coprime() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(7), None, None),
            Input::new("b".to_string(), Value::Integer(13), None, None),
        ];
        let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_gcd_with_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(5), None, None),
            Input::new("b".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/lcm.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 12),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_with_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(5), None, None),
            Input::new("b".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/frac.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_frac_settings() {
        let s = OpNumberMathFrac::settings();
        assert_eq!(s.name, "frac");
        assert_eq!(OpNumberMathFrac::create_inputs().len(), 1);
        assert_eq!(OpNumberMathFrac::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_frac_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.14).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_whole_number() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/algebra/trunc.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_trunc_settings() {
        let s = OpNumberMathTrunc::settings();
        assert_eq!(s.name, "trunc");
        assert_eq!(OpNumberMathTrunc::create_inputs().len(), 1);
        assert_eq!(OpNumberMathTrunc::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_trunc_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
        let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trunc_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-3.7), None, None)];
        let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-3.0)).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/cast/to_decimal.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_to_decimal_settings() {
        let s = OpNumberCastToDecimal::settings();
        assert_eq!(s.name, "to decimal");
        assert_eq!(OpNumberCastToDecimal::create_inputs().len(), 1);
        assert_eq!(OpNumberCastToDecimal::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_to_decimal_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 42.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_passthrough() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.14).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/cast/to_integer.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_to_integer_settings() {
        let s = OpNumberCastToInteger::settings();
        assert_eq!(s.name, "to integer");
        assert_eq!(OpNumberCastToInteger::create_inputs().len(), 1);
        assert_eq!(OpNumberCastToInteger::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_to_integer_from_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.7), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_passthrough() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/logarithmic/log.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_log_base_2() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(8.0), None, None),
            Input::new("base".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
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
}
"""

files_and_tests["numbers/logarithmic/ln.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_ln_settings() {
        let s = OpNumberMathLn::settings();
        assert_eq!(s.name, "ln");
        assert_eq!(OpNumberMathLn::create_inputs().len(), 1);
        assert_eq!(OpNumberMathLn::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_ln_e() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::E), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_1() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_invalid() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await;
        assert!(result.is_err());
    }
}
"""

files_and_tests["numbers/logarithmic/exp.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_exp_settings() {
        let s = OpNumberMathExp::settings();
        assert_eq!(s.name, "exp");
        assert_eq!(OpNumberMathExp::create_inputs().len(), 1);
        assert_eq!(OpNumberMathExp::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_exp_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_one() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::E).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
"""

files_and_tests["numbers/random/random_integer.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_random_integer_settings() {
        let s = OpNumberRandomInteger::settings();
        assert_eq!(s.name, "random integer");
        assert_eq!(OpNumberRandomInteger::create_inputs().len(), 3);
        assert_eq!(OpNumberRandomInteger::create_outputs().len(), 1);
    }
}
"""

files_and_tests["numbers/random/random_decimal.rs"] = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

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
"""

for rel_path, test_block in files_and_tests.items():
    full_path = os.path.join(base, rel_path)
    with open(full_path, 'a') as f:
        f.write(test_block)
    print(f"Updated: {rel_path}")

print("Done!")
