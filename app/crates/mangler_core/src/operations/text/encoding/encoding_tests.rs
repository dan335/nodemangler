use super::{base64_encode, base64_decode};

#[test]
fn test_base64_encode_known_vectors() {
    // RFC 4648 test vectors
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
}

#[test]
fn test_base64_decode_roundtrip() {
    for s in ["", "f", "fo", "foo", "foob", "hello, world!", "\u{1F600} unicode"] {
        let enc = base64_encode(s.as_bytes());
        let dec = base64_decode(&enc).expect("valid base64");
        assert_eq!(dec, s.as_bytes());
    }
}

#[test]
fn test_base64_decode_ignores_whitespace() {
    assert_eq!(base64_decode("Zm9v\nYmFy").unwrap(), b"foobar");
}

#[test]
fn test_base64_decode_rejects_bad_chars() {
    assert!(base64_decode("****").is_none());
}
