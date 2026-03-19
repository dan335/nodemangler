use super::*;

#[tokio::test]
async fn test_file_input_settings() {
    let s = OpImageInputFile::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageInputFile::create_inputs().is_empty());
    assert!(!OpImageInputFile::create_outputs().is_empty());
}

#[tokio::test]
async fn test_file_input_exact_settings() {
    let s = OpImageInputFile::settings();
    assert_eq!(s.name, "from file");
    assert_eq!(OpImageInputFile::create_inputs().len(), 1);
    assert_eq!(OpImageInputFile::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_file_input_nonexistent_path_returns_error() {
    use crate::input::Input;
    let mut inputs = vec![
        Input::new("path".to_string(), Value::Path(PathBuf::from("/this/does/not/exist.png")), None, None),
    ];
    let result = OpImageInputFile::run(&mut inputs).await;
    assert!(result.is_err(), "loading from nonexistent path should fail");
}
