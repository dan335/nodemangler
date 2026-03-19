use super::*;

#[tokio::test]
async fn test_e_settings() {
    let s = OpNumberInputE::settings();
    assert_eq!(s.name, "e");
    assert_eq!(OpNumberInputE::create_inputs().len(), 0);
    assert_eq!(OpNumberInputE::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_e_value() {
    let mut inputs = vec![];
    let result = OpNumberInputE::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::E).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
