use nanoid::nanoid;

#[macro_use]
extern crate lazy_static;

pub mod nodes;
pub mod input;
pub mod output;
pub mod value;
pub mod graph;
pub mod node_attributes;


pub fn get_id() -> String {
    nanoid!()
}