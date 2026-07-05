//! Text encoding/decoding operations (Base64, URL percent-encoding).
//!
//! These nodes transform `Text` to `Text`. The Base64 codec below is a small
//! self-contained implementation (the crate has no base64 dependency) shared by
//! the `base64 encode`/`base64 decode` nodes here and the `data uri` image→text
//! node in `text/image/`.

/// Standard Base64 encode of a byte string to text.
pub mod base64_encode;
/// Standard Base64 decode of text back to a (lossy-UTF8) string.
pub mod base64_decode;
/// Percent-encodes text for safe use in URLs.
pub mod url_encode;
/// Decodes percent-encoded (`%XX`) text.
pub mod url_decode;

/// Standard Base64 alphabet (RFC 4648), padded with `=`.
const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes bytes to a standard, `=`-padded Base64 string.
pub(crate) fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { B64_ALPHABET[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64_ALPHABET[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Decodes a standard Base64 string to bytes. Whitespace and `=` padding are
/// ignored. Returns `None` if a non-alphabet character is encountered.
pub(crate) fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn sextet(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = sextet(c)?;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
