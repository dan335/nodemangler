use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_random_gaussian_settings() {
    let s = OpNumberRandomGaussian::settings();
    assert_eq!(s.name, "random gaussian");
    assert_eq!(OpNumberRandomGaussian::create_inputs().len(), 3);
    assert_eq!(OpNumberRandomGaussian::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_random_gaussian_returns_decimal() {
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("mean".to_string(), Value::Decimal(0.0), None, None),
        Input::new("std dev".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberRandomGaussian::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.is_finite()),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_gaussian_mean_shift() {
    // A large mean with zero std should always return exactly the mean.
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("mean".to_string(), Value::Decimal(100.0), None, None),
        Input::new("std dev".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberRandomGaussian::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 100.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_gaussian_distribution_mean() {
    // Averaging many samples should land near the requested mean.
    let mut sum = 0.0f64;
    let n = 2000;
    for _ in 0..n {
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("mean".to_string(), Value::Decimal(5.0), None, None),
            Input::new("std dev".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberRandomGaussian::run(&mut inputs).await.unwrap();
        if let Value::Decimal(v) = &result.responses[0].value {
            sum += *v as f64;
        }
    }
    let avg = sum / n as f64;
    assert!((avg - 5.0).abs() < 0.5, "sample mean {avg} too far from 5.0");
}
