use super::*;
use std::os::unix::ffi::OsStrExt;

#[test]
fn nautilus_style_list_with_crlf_and_escapes() {
    let list = "file:///home/dan/Pictures/my%20photo.png\r\nfile:///tmp/a.jpg\r\n";
    assert_eq!(
        parse_uri_list(list),
        vec![PathBuf::from("/home/dan/Pictures/my photo.png"), PathBuf::from("/tmp/a.jpg")]
    );
}

#[test]
fn comments_blank_lines_and_non_file_uris_are_skipped() {
    let list = "# dragged from somewhere\n\nhttps://example.com/x.png\nfile:///x.png\n";
    assert_eq!(parse_uri_list(list), vec![PathBuf::from("/x.png")]);
}

#[test]
fn localhost_and_bare_forms_are_local_but_other_hosts_are_not() {
    assert_eq!(parse_uri_list("file://localhost/a.png"), vec![PathBuf::from("/a.png")]);
    assert_eq!(parse_uri_list("file:/b.png"), vec![PathBuf::from("/b.png")]);
    assert!(parse_uri_list("file://otherhost/c.png").is_empty());
    assert!(parse_uri_list("file:relative.png").is_empty());
}

#[test]
fn utf8_and_non_utf8_names_decode_to_their_bytes() {
    let paths = parse_uri_list("file:///p/caf%C3%A9.png\nfile:///p/%FF.png");
    assert_eq!(paths[0], PathBuf::from("/p/café.png"));
    assert_eq!(paths[1].as_os_str().as_bytes(), b"/p/\xFF.png");
}

#[test]
fn malformed_escapes_are_kept_literally() {
    assert_eq!(parse_uri_list("file:///a%2.png\nfile:///b%"), vec![
        PathBuf::from("/a%2.png"),
        PathBuf::from("/b%"),
    ]);
}
