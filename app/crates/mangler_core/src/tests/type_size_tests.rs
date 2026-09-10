//! Guards on the size of the types the engine allocates in bulk.
//!
//! Rust sizes an enum to its largest variant, so one oversized variant taxes
//! every value of the type — including the overwhelmingly common small ones.
//! Two variants used to do exactly that:
//!
//! * `NodeType::Subgraph` held a `Graph` inline, so every plain operation node
//!   carried a subgraph's worth of bytes, and `GraphChangedMessage` (sized by
//!   the variant that holds a `Node`) inherited it.
//! * `InputSettings::Path` held a `Vec`, a `PathBuf` and two `String`s inline,
//!   so every `Input` in the graph paid for a file picker that six operations
//!   in the crate use.
//!
//! Both are boxed now. These assertions are upper bounds with headroom, not
//! exact matches — adding an ordinary field should not fail them, but
//! *unboxing* one of those payloads (or adding a new large inline variant)
//! would, which is the regression worth catching. `Node` is cloned per
//! `LoadedNode` message and stored by value in the graph's map; `Input` is
//! allocated in bulk by `create_inputs()` on every node creation and every
//! graph load.

#[cfg(test)]
mod type_sizes {
    use std::mem::size_of;

    use crate::{
        input::{Input, InputSettings},
        node::Node,
        node_type::NodeType,
        GraphChangedMessage,
    };

    #[test]
    fn node_type_keeps_its_subgraph_payload_boxed() {
        let size = size_of::<NodeType>();
        assert!(
            size <= 64,
            "NodeType is {size} bytes; it should stay small (the subgraph's Graph is boxed). \
             Every Node pays this, subgraph or not."
        );
    }

    #[test]
    fn input_settings_keeps_its_path_payload_boxed() {
        let size = size_of::<InputSettings>();
        assert!(
            size <= 48,
            "InputSettings is {size} bytes; it should stay small (PathSettings is boxed). \
             Every Input pays this, and Slider/DragValue outnumber Path by ~200:1."
        );
    }

    #[test]
    fn bulk_allocated_types_stay_modest() {
        let node = size_of::<Node>();
        assert!(node <= 384, "Node is {node} bytes; expected to stay under 384");

        let input = size_of::<Input>();
        assert!(input <= 384, "Input is {input} bytes; expected to stay under 384");
    }

    /// Every message on the graph channel reserves the size of the largest
    /// variant, including a bare `RemovedNode { node_id }`.
    #[test]
    fn graph_messages_stay_modest() {
        let size = size_of::<GraphChangedMessage>();
        assert!(
            size <= 384,
            "GraphChangedMessage is {size} bytes; every message on the channel reserves this much"
        );
    }
}
