use super::*;
use std::path::PathBuf;

#[test]
fn test_image_format_from_path_common_extensions() {
    assert_eq!(image_format_from_path(&PathBuf::from("out.png")), Ok(ImageFormat::Png));
    assert_eq!(image_format_from_path(&PathBuf::from("out.jpg")), Ok(ImageFormat::Jpeg));
    assert_eq!(image_format_from_path(&PathBuf::from("out.jpeg")), Ok(ImageFormat::Jpeg));
    assert_eq!(image_format_from_path(&PathBuf::from("out.bmp")), Ok(ImageFormat::Bmp));
    assert_eq!(image_format_from_path(&PathBuf::from("out.avif")), Ok(ImageFormat::Avif));
    assert_eq!(image_format_from_path(&PathBuf::from("out.exr")), Ok(ImageFormat::OpenExr));
    assert_eq!(image_format_from_path(&PathBuf::from("out.hdr")), Ok(ImageFormat::Hdr));
    assert_eq!(image_format_from_path(&PathBuf::from("out.ff")), Ok(ImageFormat::Farbfeld));
}

#[test]
fn test_image_format_from_path_is_case_insensitive() {
    assert_eq!(image_format_from_path(&PathBuf::from("OUT.PNG")), Ok(ImageFormat::Png));
    assert_eq!(image_format_from_path(&PathBuf::from("out.Jpg")), Ok(ImageFormat::Jpeg));
}

#[test]
fn test_image_format_from_path_with_dotted_stem() {
    // A dot in the stem must not be mistaken for the extension boundary; the
    // real extension (last component) still drives format selection.
    assert_eq!(image_format_from_path(&PathBuf::from("render.v2.png")), Ok(ImageFormat::Png));
}

#[test]
fn test_image_format_from_path_missing_extension_errors() {
    let err = image_format_from_path(&PathBuf::from("out")).unwrap_err();
    assert!(err.contains("no extension"), "error should mention missing extension: {}", err);
    assert!(err.contains("png"), "error should list supported extensions: {}", err);
}

#[test]
fn test_image_format_from_path_unknown_extension_errors() {
    let err = image_format_from_path(&PathBuf::from("out.dds")).unwrap_err();
    assert!(err.contains("dds"), "error should mention the bad extension: {}", err);
    assert!(err.contains("png"), "error should list supported extensions: {}", err);
}
