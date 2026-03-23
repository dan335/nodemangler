//! Performance benchmarks for core graph operations.
//!
//! These tests measure execution time for common graph patterns (single node,
//! chained nodes, fan-out) comparing cold runs, cached runs, and runs with
//! changed inputs. Run with `cargo test -- --nocapture` to see timing output.

#[cfg(test)]
mod perf_tests {
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::mpsc;

    use crate::{
        float_image::FloatImage,
        get_id, graph::Graph, operations::Operation, value::Value, AddNodeType,
        GraphChangedMessage, NodeChangedMessage,
    };

    /// Create a test FloatImage of the given dimensions (4-channel RGBA gradient).
    fn make_test_image(width: u32, height: u32) -> FloatImage {
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                data.push((x % 256) as f32 / 255.0); // r
                data.push((y % 256) as f32 / 255.0); // g
                data.push(128.0 / 255.0);             // b
                data.push(1.0);                        // a
            }
        }
        FloatImage::from_raw(width, height, 4, data).expect("data length matches")
    }

    fn make_image_value(width: u32, height: u32) -> Value {
        Value::Image {
            data: Arc::new(make_test_image(width, height)),
            change_id: get_id(),
        }
    }

    fn create_test_graph() -> Graph {
        let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(32);
        let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(32);
        Graph::new(get_id(), tx_node_changed, tx_graph_changed, false).unwrap()
    }

    // ---------------------------------------------------------------
    // Simple test: measure how long it takes to clone a Value::Image
    // ---------------------------------------------------------------
    #[test]
    fn perf_image_value_clone() {
        let sizes: [(u32, u32); 3] = [(256, 256), (1024, 1024), (2048, 2048)];

        for (w, h) in sizes {
            let value = make_image_value(w, h);
            let iterations = 100;

            let start = Instant::now();
            for _ in 0..iterations {
                let _cloned = value.clone();
            }
            let elapsed = start.elapsed();

            let per_clone_us = elapsed.as_micros() as f64 / iterations as f64;
            println!(
                "PERF clone {}x{} image: {:.0}us per clone ({} clones in {:.1}ms)",
                w, h, per_clone_us, iterations, elapsed.as_secs_f64() * 1000.0
            );
        }
    }

    // ---------------------------------------------------------------
    // Single blur node: cold run (actually computes) vs cached run
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn perf_single_node_image_pass() {
        let mut graph = create_test_graph();

        let blur_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpImageAdjustmentBlur),
                glam::Vec2::ZERO, true, None,
            )
            .await;

        let image_value = make_image_value(1024, 1024);
        graph.set_input(blur_id.clone(), 0, image_value);
        graph.set_input(blur_id.clone(), 1, Value::Decimal(0.5));

        // Cold run (first execution)
        let start = Instant::now();
        graph.run().await;
        let cold_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Cached run (same inputs)
        graph.set_input(blur_id.clone(), 1, Value::Decimal(0.5));
        let start = Instant::now();
        graph.run().await;
        let cached_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Changed input (forces re-execution)
        let iterations = 3;
        let start = Instant::now();
        for i in 0..iterations {
            graph.set_input(blur_id.clone(), 1, Value::Decimal(0.5 + i as f32 * 0.01));
            graph.run().await;
        }
        let changed_ms = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        println!(
            "PERF single blur (1024x1024): cold={:.1}ms, cached={:.2}ms, changed={:.1}ms",
            cold_ms, cached_ms, changed_ms
        );
    }

    // ---------------------------------------------------------------
    // 5-node chain: cold vs cached vs changed
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn perf_chained_image_nodes() {
        let mut graph = create_test_graph();

        let input_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpImageAdjustmentBlur),
                glam::Vec2::new(0.0, 0.0), true, None,
            )
            .await;

        let mut prev_id = input_id.clone();

        for i in 1..=4 {
            let blur_id = graph
                .add_node(
                    get_id(),
                    AddNodeType::Operation(Operation::OpImageAdjustmentBlur),
                    glam::Vec2::new(i as f32 * 200.0, 0.0), true, None,
                )
                .await;

            graph
                .add_connection(blur_id.clone(), 0, prev_id.clone(), 0)
                .await;

            graph.set_input(blur_id.clone(), 1, Value::Decimal(0.1));
            prev_id = blur_id;
        }

        let image_value = make_image_value(512, 512);
        graph.set_input(input_id.clone(), 0, image_value);
        graph.set_input(input_id.clone(), 1, Value::Decimal(0.1));

        // Cold
        let start = Instant::now();
        graph.run().await;
        let cold_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Cached
        graph.set_input(input_id.clone(), 1, Value::Decimal(0.1));
        let start = Instant::now();
        graph.run().await;
        let cached_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Changed
        let iterations = 3;
        let start = Instant::now();
        for i in 0..iterations {
            graph.set_input(input_id.clone(), 1, Value::Decimal(0.1 + i as f32 * 0.01));
            graph.run().await;
        }
        let changed_ms = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        println!(
            "PERF 5-node chain (512x512): cold={:.1}ms, cached={:.2}ms, changed={:.1}ms",
            cold_ms, cached_ms, changed_ms
        );
    }

    // ---------------------------------------------------------------
    // Fan-out 1->3: cold vs cached vs changed
    // Tests parallel execution of independent downstream nodes
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn perf_fanout_image_nodes() {
        let mut graph = create_test_graph();

        let source_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpImageAdjustmentBlur),
                glam::Vec2::new(0.0, 0.0), true, None,
            )
            .await;

        graph.set_input(source_id.clone(), 0, make_image_value(1024, 1024));
        graph.set_input(source_id.clone(), 1, Value::Decimal(0.1));

        for i in 0..3 {
            let blur_id = graph
                .add_node(
                    get_id(),
                    AddNodeType::Operation(Operation::OpImageAdjustmentBlur),
                    glam::Vec2::new(200.0, i as f32 * 100.0), true, None,
                )
                .await;

            graph
                .add_connection(blur_id.clone(), 0, source_id.clone(), 0)
                .await;
            graph.set_input(blur_id.clone(), 1, Value::Decimal(0.1));
        }

        // Cold
        let start = Instant::now();
        graph.run().await;
        let cold_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Cached
        graph.set_input(source_id.clone(), 1, Value::Decimal(0.1));
        let start = Instant::now();
        graph.run().await;
        let cached_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Changed
        let iterations = 3;
        let start = Instant::now();
        for i in 0..iterations {
            graph.set_input(source_id.clone(), 1, Value::Decimal(0.1 + i as f32 * 0.01));
            graph.run().await;
        }
        let changed_ms = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        println!(
            "PERF fan-out 1->3 (1024x1024): cold={:.1}ms, cached={:.2}ms, changed={:.1}ms",
            cold_ms, cached_ms, changed_ms
        );
    }
}
