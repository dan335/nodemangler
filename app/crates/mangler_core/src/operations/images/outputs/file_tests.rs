use super::*;

#[tokio::test]
async fn test_file_output_settings() {
    let s = OpImageOutputFile::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageOutputFile::create_inputs().is_empty());
    assert!(!OpImageOutputFile::create_outputs().is_empty());
}

#[tokio::test]
async fn test_file_output_exact_settings() {
    let s = OpImageOutputFile::settings();
    assert_eq!(s.name, "image to file");
    assert_eq!(OpImageOutputFile::create_inputs().len(), 4);
    assert_eq!(OpImageOutputFile::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_file_output_nonexistent_folder_returns_error() {
    use image::DynamicImage;
    use std::sync::Arc;
    use crate::get_id;

    let imgbuf = image::RgbaImage::new(4, 4);
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("file name".to_string(), Value::Text("test_output".to_string()), None, None),
        Input::new("folder".to_string(), Value::Path(std::path::PathBuf::from("/this/path/does/not/exist/at/all")), None, None),
        Input::new("image format".to_string(), Value::ImageType(image::ImageFormat::Png), None, None),
    ];
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "saving to nonexistent folder should fail");
}
