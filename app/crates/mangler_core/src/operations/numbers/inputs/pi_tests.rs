use super::*;

#[tokio::test]
async fn test_pi_settings() {
    let s = OpNumberInputPi::settings();
    assert_eq!(s.name, "pi");
    assert_eq!(OpNumberInputPi::create_inputs().len(), 0);
    assert_eq!(OpNumberInputPi::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_pi_value() {
    let mut inputs = vec![];
    let result = OpNumberInputPi::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::PI).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
