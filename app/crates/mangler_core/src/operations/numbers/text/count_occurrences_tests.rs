use super::*;
use crate::input::Input;
use crate::value::Value;

fn run_on(text: &str, sub: &str) -> i32 {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text(text.to_string()), None, None),
        Input::new("substring".to_string(), Value::Text(sub.to_string()), None, None),
    ];
    let rt = tokio::runtime::Runtime::new().unwrap();
    let r = rt.block_on(OpNumberTextCountOccurrences::run(&mut inputs)).unwrap();
    match r.responses[0].value { Value::Integer(n) => n, ref o => panic!("expected Integer, got {:?}", o) }
}

#[tokio::test]
async fn test_count_occurrences_settings() {
    let s = OpNumberTextCountOccurrences::settings();
    assert_eq!(s.name, "count occurrences");
    assert_eq!(OpNumberTextCountOccurrences::create_inputs().len(), 2);
}

#[test]
fn test_count_occurrences_values() {
    assert_eq!(run_on("banana", "a"), 3);
    // Non-overlapping: "aaaa" contains "aa" twice.
    assert_eq!(run_on("aaaa", "aa"), 2);
    assert_eq!(run_on("hello", "z"), 0);
    assert_eq!(run_on("anything", ""), 0);
}
