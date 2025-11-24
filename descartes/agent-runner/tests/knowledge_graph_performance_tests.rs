/// Knowledge Graph Performance Tests
///
/// Performance benchmarks and stress tests for the knowledge graph system with focus on:
/// - Large codebase handling (10,000+ files)
/// - Graph traversal performance
/// - Search and query optimization
/// - Memory usage
/// - Incremental updates

#[cfg(test)]
mod performance_tests {
    use descartes_agent_runner::knowledge_graph::*;
    use descartes_agent_runner::knowledge_graph_overlay::*;
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    /// Helper to create a large test graph
    fn create_large_graph(num_nodes: usize, edges_per_node: usize) -> KnowledgeGraph {
        let mut graph = KnowledgeGraph::new();
        let mut node_ids = Vec::new();

        // Create nodes
        for i in 0..num_nodes {
            let node_type = match i % 4 {
                0 => KnowledgeNodeType::Function,
                1 => KnowledgeNodeType::Class,
                2 => KnowledgeNodeType::Method,
                _ => KnowledgeNodeType::Module,
            };

            let node = KnowledgeNode::new(
                node_type,
                format!("entity_{}", i),
                format!("module::entity_{}", i),
            );

            let node_id = graph.add_node(node);
            node_ids.push(node_id);
        }

        // Create edges
        for i in 0..num_nodes {
            for j in 1..=edges_per_node {
                let target_idx = (i + j) % num_nodes;
                let rel_type = match j % 3 {
                    0 => RelationshipType::Calls,
                    1 => RelationshipType::Uses,
                    _ => RelationshipType::DependsOn,
                };

                let edge = KnowledgeEdge::new(
                    node_ids[i].clone(),
                    node_ids[target_idx].clone(),
                    rel_type,
                );
                graph.add_edge(edge);
            }
        }

        graph
    }

    #[test]
    fn test_large_graph_creation_performance() {
        let start = Instant::now();
        let graph = create_large_graph(1000, 5);
        let duration = start.elapsed();

        println!("Created graph with 1000 nodes and ~5000 edges in {:?}", duration);

        assert_eq!(graph.nodes.len(), 1000);
        assert!(duration < Duration::from_secs(1), "Graph creation too slow");
    }

    #[test]
    fn test_very_large_graph_creation() {
        let start = Instant::now();
        let graph = create_large_graph(10000, 3);
        let duration = start.elapsed();

        println!(
            "Created graph with 10000 nodes and ~30000 edges in {:?}",
            duration
        );

        assert_eq!(graph.nodes.len(), 10000);
        // Should complete in reasonable time even for large graphs
        assert!(duration < Duration::from_secs(10));
    }

    #[test]
    fn test_node_lookup_performance() {
        let graph = create_large_graph(10000, 3);
        let node_ids: Vec<_> = graph.nodes.keys().take(100).cloned().collect();

        let start = Instant::now();
        for node_id in &node_ids {
            let _ = graph.get_node(node_id);
        }
        let duration = start.elapsed();

        println!("100 node lookups in {:?}", duration);
        assert!(
            duration < Duration::from_millis(10),
            "Node lookups too slow"
        );
    }

    #[test]
    fn test_name_based_lookup_performance() {
        let graph = create_large_graph(10000, 3);

        let start = Instant::now();
        for i in 0..100 {
            let name = format!("module::entity_{}", i);
            let _ = graph.get_node_by_name(&name);
        }
        let duration = start.elapsed();

        println!("100 name-based lookups in {:?}", duration);
        assert!(
            duration < Duration::from_millis(10),
            "Name lookups too slow"
        );
    }

    #[test]
    fn test_edge_traversal_performance() {
        let graph = create_large_graph(5000, 5);
        let node_ids: Vec<_> = graph.nodes.keys().take(100).cloned().collect();

        let start = Instant::now();
        let mut total_edges = 0;
        for node_id in &node_ids {
            let outgoing = graph.get_outgoing_edges(node_id);
            let incoming = graph.get_incoming_edges(node_id);
            total_edges += outgoing.len() + incoming.len();
        }
        let duration = start.elapsed();

        println!(
            "Traversed {} edges from 100 nodes in {:?}",
            total_edges, duration
        );
        assert!(
            duration < Duration::from_millis(50),
            "Edge traversal too slow"
        );
    }

    #[test]
    fn test_neighbor_lookup_performance() {
        let graph = create_large_graph(5000, 5);
        let node_ids: Vec<_> = graph.nodes.keys().take(100).cloned().collect();

        let start = Instant::now();
        let mut total_neighbors = 0;
        for node_id in &node_ids {
            let neighbors = graph.get_neighbors(node_id);
            total_neighbors += neighbors.len();
        }
        let duration = start.elapsed();

        println!(
            "Found {} total neighbors for 100 nodes in {:?}",
            total_neighbors, duration
        );
        assert!(
            duration < Duration::from_millis(100),
            "Neighbor lookup too slow"
        );
    }

    #[test]
    fn test_path_finding_performance() {
        let graph = create_large_graph(1000, 3);
        let node_ids: Vec<_> = graph.nodes.keys().take(10).cloned().collect();

        let start = Instant::now();
        let mut paths_found = 0;
        for i in 0..node_ids.len() - 1 {
            if let Some(_path) = graph.find_path(&node_ids[i], &node_ids[i + 1]) {
                paths_found += 1;
            }
        }
        let duration = start.elapsed();

        println!(
            "Found {} paths in graph with 1000 nodes in {:?}",
            paths_found, duration
        );
        assert!(
            duration < Duration::from_millis(500),
            "Path finding too slow"
        );
    }

    #[test]
    fn test_type_filtering_performance() {
        let graph = create_large_graph(10000, 3);

        let start = Instant::now();
        let functions = graph.get_nodes_by_type(KnowledgeNodeType::Function);
        let classes = graph.get_nodes_by_type(KnowledgeNodeType::Class);
        let methods = graph.get_nodes_by_type(KnowledgeNodeType::Method);
        let duration = start.elapsed();

        println!(
            "Filtered by 3 types in graph with 10000 nodes in {:?}",
            duration
        );
        assert!(functions.len() > 0);
        assert!(classes.len() > 0);
        assert!(methods.len() > 0);
        assert!(
            duration < Duration::from_millis(10),
            "Type filtering too slow"
        );
    }

    #[test]
    fn test_search_performance() {
        let graph = create_large_graph(10000, 3);

        let start = Instant::now();
        let results = graph.find_nodes(|node| node.name.contains("entity_1"));
        let duration = start.elapsed();

        println!(
            "Searched 10000 nodes and found {} results in {:?}",
            results.len(),
            duration
        );
        assert!(results.len() > 0);
        assert!(
            duration < Duration::from_millis(50),
            "Search too slow"
        );
    }

    #[test]
    fn test_statistics_computation_performance() {
        let graph = create_large_graph(10000, 5);

        let start = Instant::now();
        let stats = graph.stats();
        let duration = start.elapsed();

        println!("Computed stats for large graph in {:?}", duration);
        println!("Stats: {:?}", stats);

        assert_eq!(stats.total_nodes, 10000);
        assert!(stats.total_edges > 0);
        assert!(
            duration < Duration::from_millis(100),
            "Stats computation too slow"
        );
    }

    #[test]
    fn test_concurrent_read_performance() {
        use std::sync::Arc;
        use std::thread;

        let graph = Arc::new(create_large_graph(5000, 5));
        let num_threads = 4;
        let lookups_per_thread = 1000;

        let start = Instant::now();
        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let graph_clone = Arc::clone(&graph);
                thread::spawn(move || {
                    for i in 0..lookups_per_thread {
                        let idx = (thread_id * lookups_per_thread + i) % 5000;
                        let name = format!("module::entity_{}", idx);
                        let _ = graph_clone.get_node_by_name(&name);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        println!(
            "{} concurrent lookups across {} threads in {:?}",
            num_threads * lookups_per_thread,
            num_threads,
            duration
        );
        assert!(
            duration < Duration::from_secs(1),
            "Concurrent reads too slow"
        );
    }

    #[test]
    fn test_memory_usage_estimation() {
        let graph = create_large_graph(10000, 5);

        // Rough memory estimation
        let nodes_size = graph.nodes.len() * std::mem::size_of::<KnowledgeNode>();
        let edges_size = graph.edges.len() * std::mem::size_of::<KnowledgeEdge>();
        let total_mb = (nodes_size + edges_size) as f64 / 1_000_000.0;

        println!(
            "Estimated memory usage: {:.2} MB for 10000 nodes and {} edges",
            total_mb,
            graph.edges.len()
        );

        // Sanity check - should not use excessive memory
        assert!(total_mb < 500.0, "Memory usage too high");
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use descartes_agent_runner::knowledge_graph::*;

    #[test]
    fn test_stress_massive_graph() {
        // Create a very large graph to test limits
        let start = Instant::now();
        let mut graph = KnowledgeGraph::new();

        // Add 50,000 nodes
        let mut node_ids = Vec::new();
        for i in 0..50_000 {
            let node = KnowledgeNode::new(
                KnowledgeNodeType::Function,
                format!("func_{}", i),
                format!("module::func_{}", i),
            );
            let node_id = graph.add_node(node);
            node_ids.push(node_id);
        }

        // Add edges (sparse graph to avoid explosion)
        for i in 0..50_000 {
            if i + 1 < 50_000 {
                let edge = KnowledgeEdge::new(
                    node_ids[i].clone(),
                    node_ids[i + 1].clone(),
                    RelationshipType::Calls,
                );
                graph.add_edge(edge);
            }
        }

        let duration = start.elapsed();

        println!(
            "Created massive graph with 50000 nodes in {:?}",
            duration
        );
        assert_eq!(graph.nodes.len(), 50_000);
        // Should handle large graphs
        assert!(duration < Duration::from_secs(30));
    }

    #[test]
    fn test_stress_deep_traversal() {
        // Create a deep chain for testing deep traversals
        let mut graph = KnowledgeGraph::new();
        let chain_length = 1000;

        let mut prev_id = None;
        for i in 0..chain_length {
            let node = KnowledgeNode::new(
                KnowledgeNodeType::Function,
                format!("step_{}", i),
                format!("chain::step_{}", i),
            );
            let node_id = graph.add_node(node);

            if let Some(prev) = prev_id {
                let edge =
                    KnowledgeEdge::new(prev, node_id.clone(), RelationshipType::Calls);
                graph.add_edge(edge);
            }
            prev_id = Some(node_id);
        }

        // Test path finding on deep chain
        let first_id = graph
            .get_node_by_name("chain::step_0")
            .unwrap()
            .node_id
            .clone();
        let last_id = graph
            .get_node_by_name(&format!("chain::step_{}", chain_length - 1))
            .unwrap()
            .node_id
            .clone();

        let start = Instant::now();
        let path = graph.find_path(&first_id, &last_id);
        let duration = start.elapsed();

        println!(
            "Found path of length {} in deep chain in {:?}",
            path.as_ref().map(|p| p.len()).unwrap_or(0),
            duration
        );
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), chain_length);
        assert!(duration < Duration::from_secs(1));
    }

    #[test]
    fn test_stress_highly_connected_graph() {
        // Create a dense graph where each node connects to many others
        let mut graph = KnowledgeGraph::new();
        let num_nodes = 500;
        let connections_per_node = 50;

        let mut node_ids = Vec::new();
        for i in 0..num_nodes {
            let node = KnowledgeNode::new(
                KnowledgeNodeType::Function,
                format!("node_{}", i),
                format!("module::node_{}", i),
            );
            let node_id = graph.add_node(node);
            node_ids.push(node_id);
        }

        // Create many edges
        for i in 0..num_nodes {
            for j in 0..connections_per_node {
                let target = (i + j + 1) % num_nodes;
                let edge = KnowledgeEdge::new(
                    node_ids[i].clone(),
                    node_ids[target].clone(),
                    RelationshipType::Calls,
                );
                graph.add_edge(edge);
            }
        }

        println!(
            "Created highly connected graph: {} nodes, {} edges",
            graph.nodes.len(),
            graph.edges.len()
        );

        // Test neighbor lookups
        let start = Instant::now();
        let neighbors = graph.get_neighbors(&node_ids[0]);
        let duration = start.elapsed();

        println!(
            "Found {} neighbors in highly connected graph in {:?}",
            neighbors.len(),
            duration
        );
        assert!(neighbors.len() >= connections_per_node);
        assert!(duration < Duration::from_millis(10));
    }
}

#[cfg(test)]
mod scalability_tests {
    use super::*;
    use descartes_agent_runner::knowledge_graph::*;

    #[test]
    fn test_scalability_linear_growth() {
        // Test that performance scales linearly with graph size
        let sizes = vec![100, 500, 1000, 5000];
        let mut timings = Vec::new();

        for size in &sizes {
            let start = Instant::now();
            let graph = create_large_graph(*size, 3);
            let duration = start.elapsed();
            timings.push(duration);

            println!("Size {}: {:?}", size, duration);
        }

        // Check that growth is roughly linear (not exponential)
        // Time for 5000 should be less than 100x the time for 100
        let ratio = timings[3].as_millis() as f64 / timings[0].as_millis() as f64;
        println!("Growth ratio (5000/100): {:.2}x", ratio);
        assert!(ratio < 100.0, "Performance degradation too high");
    }

    #[test]
    fn test_search_scalability() {
        let sizes = vec![1000, 5000, 10000];

        for size in sizes {
            let graph = create_large_graph(size, 3);

            let start = Instant::now();
            let _results = graph.find_nodes(|node| node.name.contains("_1"));
            let duration = start.elapsed();

            println!("Search in {} nodes: {:?}", size, duration);
            assert!(duration < Duration::from_millis(100));
        }
    }

    #[test]
    fn test_incremental_update_performance() {
        let mut graph = create_large_graph(10000, 3);
        let initial_count = graph.nodes.len();

        // Simulate incremental updates
        let start = Instant::now();
        for i in 0..100 {
            let node = KnowledgeNode::new(
                KnowledgeNodeType::Function,
                format!("new_func_{}", i),
                format!("module::new_func_{}", i),
            );
            graph.add_node(node);
        }
        let duration = start.elapsed();

        println!("Added 100 nodes to large graph in {:?}", duration);
        assert_eq!(graph.nodes.len(), initial_count + 100);
        assert!(
            duration < Duration::from_millis(100),
            "Incremental updates too slow"
        );
    }
}

#[cfg(test)]
mod optimization_tests {
    use super::*;
    use descartes_agent_runner::knowledge_graph::*;

    #[test]
    fn test_index_effectiveness() {
        let graph = create_large_graph(10000, 3);

        // Test that indices make lookups fast
        let start = Instant::now();
        for i in 0..1000 {
            let name = format!("module::entity_{}", i);
            let _ = graph.get_node_by_name(&name);
        }
        let duration = start.elapsed();

        println!("1000 indexed lookups in {:?}", duration);
        // With proper indexing, this should be very fast
        assert!(duration < Duration::from_millis(50));
    }

    #[test]
    fn test_type_index_effectiveness() {
        let graph = create_large_graph(10000, 3);

        // Multiple type filters should be fast
        let start = Instant::now();
        for _ in 0..100 {
            let _ = graph.get_nodes_by_type(KnowledgeNodeType::Function);
            let _ = graph.get_nodes_by_type(KnowledgeNodeType::Class);
        }
        let duration = start.elapsed();

        println!("200 type filter operations in {:?}", duration);
        assert!(duration < Duration::from_millis(50));
    }

    #[test]
    fn test_edge_index_effectiveness() {
        let graph = create_large_graph(10000, 5);
        let node_ids: Vec<_> = graph.nodes.keys().take(1000).cloned().collect();

        // Edge lookups should be fast with indexing
        let start = Instant::now();
        for node_id in &node_ids {
            let _ = graph.get_outgoing_edges(node_id);
            let _ = graph.get_incoming_edges(node_id);
        }
        let duration = start.elapsed();

        println!("2000 edge lookups in {:?}", duration);
        assert!(duration < Duration::from_millis(100));
    }
}
