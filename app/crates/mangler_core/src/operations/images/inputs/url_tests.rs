use super::*;

#[tokio::test]
async fn test_url_input_settings() {
    let s = OpImageInputUrl::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageInputUrl::create_inputs().is_empty());
    assert!(!OpImageInputUrl::create_outputs().is_empty());
}

#[tokio::test]
async fn test_url_input_exact_settings() {
    let s = OpImageInputUrl::settings();
    assert_eq!(s.name, "from url");
    assert_eq!(OpImageInputUrl::create_inputs().len(), 1);
    assert_eq!(OpImageInputUrl::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_url_input_invalid_url_returns_error() {
    use crate::input::Input;
    let mut inputs = vec![
        Input::new("url".to_string(), Value::Text("not_a_valid_url".to_string()), None, None),
    ];
    let result = OpImageInputUrl::run(&mut inputs).await;
    assert!(result.is_err(), "invalid url should return error");
}
