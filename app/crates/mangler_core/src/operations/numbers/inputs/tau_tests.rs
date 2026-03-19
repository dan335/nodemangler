use super::*;

#[tokio::test]
async fn test_tau_settings() {
    let s = OpNumberInputTau::settings();
    assert_eq!(s.name, "tau");
    assert_eq!(OpNumberInputTau::create_inputs().len(), 0);
    assert_eq!(OpNumberInputTau::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_tau_value() {
    let mut inputs = vec![];
    let result = OpNumberInputTau::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::TAU).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
