use super::*;

#[tokio::test]
async fn test_phi_settings() {
    let s = OpNumberInputPhi::settings();
    assert_eq!(s.name, "phi");
    assert_eq!(OpNumberInputPhi::create_inputs().len(), 0);
    assert_eq!(OpNumberInputPhi::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_phi_value() {
    let mut inputs = vec![];
    let result = OpNumberInputPhi::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.618_034).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
