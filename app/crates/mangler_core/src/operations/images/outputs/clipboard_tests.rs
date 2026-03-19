use super::*;

#[tokio::test]
async fn test_clipboard_output_settings() {
    let s = OpImageOutputClipboard::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageOutputClipboard::create_inputs().is_empty());
    assert_eq!(OpImageOutputClipboard::create_outputs().len(), 0);
}
