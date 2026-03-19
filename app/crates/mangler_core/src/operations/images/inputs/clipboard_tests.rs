use super::*;

#[tokio::test]
async fn test_clipboard_input_settings() {
    let s = OpImageInputClipboard::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageInputClipboard::create_inputs().is_empty());
    assert!(!OpImageInputClipboard::create_outputs().is_empty());
}
