use super::*;
use crate::input::Input;
use crate::value::Value;

fn run_on(s: &str) -> i32 {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text(s.to_string()), None, None)];
    let rt = tokio::runtime::Runtime::new().unwrap();
    let r = rt.block_on(OpNumberTextWordCount::run(&mut inputs)).unwrap();
    match r.responses[0].value { Value::Integer(n) => n, ref o => panic!("expected Integer, got {:?}", o) }
}

#[tokio::test]
async fn test_word_count_settings() {
    let s = OpNumberTextWordCount::settings();
    assert_eq!(s.name, "word count");
    assert_eq!(OpNumberTextWordCount::create_outputs().len(), 1);
}

#[test]
fn test_word_count_values() {
    assert_eq!(run_on(""), 0);
    assert_eq!(run_on("   "), 0);
    assert_eq!(run_on("hello"), 1);
    assert_eq!(run_on("hello world"), 2);
    assert_eq!(run_on("  the   quick  brown fox  "), 4);
}
