use super::*;
use crate::input::Input;
use crate::value::Value;

fn run_on(s: &str) -> i32 {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text(s.to_string()), None, None)];
    let rt = tokio::runtime::Runtime::new().unwrap();
    let r = rt.block_on(OpNumberTextByteLength::run(&mut inputs)).unwrap();
    match r.responses[0].value { Value::Integer(n) => n, ref o => panic!("expected Integer, got {:?}", o) }
}

#[tokio::test]
async fn test_byte_length_settings() {
    let s = OpNumberTextByteLength::settings();
    assert_eq!(s.name, "byte length");
    assert_eq!(OpNumberTextByteLength::create_outputs().len(), 1);
}

#[test]
fn test_byte_length_values() {
    assert_eq!(run_on(""), 0);
    assert_eq!(run_on("abc"), 3);
    // "é" is 2 bytes in UTF-8 but a single character.
    assert_eq!(run_on("é"), 2);
    // Emoji is 4 bytes in UTF-8.
    assert_eq!(run_on("😀"), 4);
}
