use crate::value::Value;

pub trait Node {
    fn run(&mut self);
    fn set_intput_value(&mut self, index: usize, value: Value);
    fn print_output(&self) -> String;
}
