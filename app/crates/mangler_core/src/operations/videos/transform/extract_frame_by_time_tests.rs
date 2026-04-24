use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs() -> Vec<Input> {
    OpExtractFrameByTime::create_inputs()
}

#[test]
fn test_apply_render_time_writes_time_directly() {
    let mut inputs = make_inputs();
    OpExtractFrameByTime::apply_render_time(&mut inputs, 1.75);
    match &inputs[1].value {
        Value::Decimal(t) => assert!((t - 1.75).abs() < 1e-6),
        other => panic!("expected Decimal(1.75), got {:?}", other),
    }
}

#[test]
fn test_apply_render_time_updates_on_each_call() {
    // Render loop calls this once per frame; verify later calls overwrite
    // earlier values rather than additively drifting.
    let mut inputs = make_inputs();
    OpExtractFrameByTime::apply_render_time(&mut inputs, 0.5);
    OpExtractFrameByTime::apply_render_time(&mut inputs, 3.0);
    OpExtractFrameByTime::apply_render_time(&mut inputs, 0.1);
    match &inputs[1].value {
        Value::Decimal(t) => assert!((t - 0.1).abs() < 1e-6),
        other => panic!("expected Decimal(0.1), got {:?}", other),
    }
}
