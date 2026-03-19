use super::*;
use crate::input::Input;
use crate::value::Value;

fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
    vals.iter()
        .enumerate()
        .map(|(i, v)| Input::new(format!("v{}",  i), Value::Decimal(*v), None, None))
        .collect()
}

#[tokio::test]
async fn test_xyz_input() {
    let mut inputs = decimal_inputs(&[0.5, 0.2, 0.1, 1.0]);
    let result = OpColorInputXyz::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_xyz_settings() {
    let s = OpColorInputXyz::settings();
    assert_eq!(s.name, "xyz");
    assert_eq!(OpColorInputXyz::create_inputs().len(), 4);
    assert_eq!(OpColorInputXyz::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_xyz_zero_alpha() {
    let mut inputs = decimal_inputs(&[0.5, 0.2, 0.1, 0.0]);
    let result = OpColorInputXyz::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (_, _, _, a) = c.to_srgb_float();
            assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_xyz_produces_color() {
    // Various XYZ values should produce a Color without panicking
    for (x, y, z) in [(0.0f32, 0.0f32, 0.0f32), (0.5, 0.2, 0.1), (0.95, 1.0, 1.09)] {
        let mut inputs = decimal_inputs(&[x, y, z, 1.0]);
        let result = OpColorInputXyz::run(&mut inputs).await;
        assert!(result.is_ok(), "xyz ({},{},{}) failed: {:?}", x, y, z, result.err());
    }
}

#[tokio::test]
async fn test_xyz_zero_is_black() {
    // X=Y=Z=0 should give black
    let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 1.0]);
    let result = OpColorInputXyz::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.02, "black R should be ~0, got {}", r);
            assert!(g.abs() < 0.02, "black G should be ~0, got {}", g);
            assert!(b.abs() < 0.02, "black B should be ~0, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}
