use mangler::graph::Graph;
use mangler::nodes::*;
use mangler::value::Value;

fn main() {
    let mut graph = Graph::new();
    let id = add::Add::new(&mut graph);

    if let Some(node) = graph.nodes.get_mut(&id) {
        node.set_intput_value(0, Value::Decimal { value: 5.0 });
    }

    graph.run();

    if let Some(v) = graph.nodes.get(&id) {
        println!("Hello, world! {:?}", v.print_output());
    } 
}
