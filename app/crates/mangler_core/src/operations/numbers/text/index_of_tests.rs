use super::*;
use crate::input::Input;
use crate::value::Value;

fn run_on(text: &str, sub: &str) -> i32 {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text(text.to_string()), None, None),
        Input::new("substring".to_string(), Value::Text(sub.to_string()), None, None),
    ];
    let rt = tokio::runtime::Runtime::new().unwrap();
    let r = rt.block_on(OpNumberTextIndexOf::run(&mut inputs)).unwrap();
    match r.responses[0].value { Value::Integer(n) => n, ref o => panic!("expected Integer, got {:?}", o) }
}

#[tokio::test]
async fn test_index_of_settings() {
    let s = OpNumberTextIndexOf::settings();
    assert_eq!(s.name, "index of");
    assert_eq!(OpNumberTextIndexOf::create_inputs().len(), 2);
}

#[test]
fn test_index_of_values() {
    assert_eq!(run_on("hello world", "world"), 6);
    assert_eq!(run_on("hello", "z"), -1);
    assert_eq!(run_on("anything", ""), 0);
    // Char index, not byte offset: "é" is 2 bytes, so "x" sits at char index 1.
    assert_eq!(run_on("éx", "x"), 1);
}
