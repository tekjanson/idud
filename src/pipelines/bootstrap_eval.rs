//! Phase 4: Continuous Bootstrap Evaluation
//! Pure topological analysis layer: zero-cost metric extraction from immutable petgraph
//!
//! TopologicalAnalyzer computes three deterministic metrics on the Contract Ledger's DiGraph:
//! 1. Cyclic Violations: Strongly Connected Components (SCCs) with vertex count > 1
//! 2. Orphan Rate: Ratio of fully isolated signatories (in-degree=0, out-degree=0)
//! 3. Contract Density: Overall coupling saturation (edges / (vertices * (vertices - 1)))
//!
//! DESIGN PRINCIPLES:
//! - Read-only: Accepts immutable references only, zero locking of graph
//! - Deterministic: Pure CPU math, no randomness or approximation
//! - Zero Dependencies: petgraph only (no external database/network crates)
//! - Lexicon: Uses StrictlyBinds/StrictlyBoundBy for directional coupling

use petgraph::algo;
use petgraph::prelude::*;

/// Bootstrap Evaluation Metrics: deterministic topological snapshot
#[derive(Debug, Clone)]
pub struct BootstrapMetrics {
    /// Cyclic violations: each Vec<NodeIndex> is a violating SCC cluster
    pub cyclic_violations: Vec<Vec<NodeIndex>>,
    /// Orphan rate: ratio of isolated nodes to total vertex count
    pub orphan_rate: f64,
    /// Contract density: edge saturation of the graph
    pub contract_density: f64,
    /// Total signatories in the graph
    pub total_vertices: usize,
    /// Total contracts (StrictlyBinds relationships) in the graph
    pub total_edges: usize,
}

impl Default for BootstrapMetrics {
    fn default() -> Self {
        Self {
            cyclic_violations: Vec::new(),
            orphan_rate: 0.0,
            contract_density: 0.0,
            total_vertices: 0,
            total_edges: 0,
        }
    }
}

/// TopologicalAnalyzer: pure, read-only metric extraction from the Contract Ledger's DiGraph
///
/// This analyzer runs continuously on the immutable graph, computing metrics with zero locking.
/// Each metric is derived deterministically from the graph structure.
pub struct TopologicalAnalyzer;

impl TopologicalAnalyzer {
    /// Analyze a graph for all bootstrap metrics
    ///
    /// # Arguments
    /// * `graph` - read-only reference to the DiGraph (immutable borrow)
    ///
    /// # Returns
    /// BootstrapMetrics with cyclic violations, orphan rate, and contract density
    pub fn analyze(graph: &DiGraph<(), ()>) -> BootstrapMetrics {
        let total_vertices = graph.node_count();
        let total_edges = graph.edge_count();

        let cyclic_violations = Self::detect_cyclic_violations(graph);
        let orphan_rate = Self::compute_orphan_rate(graph, total_vertices);
        let contract_density = Self::compute_contract_density(total_vertices, total_edges);

        BootstrapMetrics {
            cyclic_violations,
            orphan_rate,
            contract_density,
            total_vertices,
            total_edges,
        }
    }

    /// Detect Cyclic Violations using Tarjan's Strongly Connected Components algorithm
    ///
    /// A cyclic violation is any SCC with more than one vertex: a circular dependency loop.
    /// Uses petgraph::algo::tarjan_scc which operates in O(V + E) time.
    ///
    /// # Arguments
    /// * `graph` - read-only reference to the DiGraph
    ///
    /// # Returns
    /// Vec<Vec<NodeIndex>> where each Vec is a violating SCC cluster (cycles)
    fn detect_cyclic_violations(graph: &DiGraph<(), ()>) -> Vec<Vec<NodeIndex>> {
        let sccs = algo::tarjan_scc(graph);
        sccs.into_iter().filter(|scc| scc.len() > 1).collect()
    }

    /// Compute Orphan Rate: ratio of fully isolated signatories
    ///
    /// An orphan is a node where:
    /// - in_degree() == 0 (no one StrictlyBinds to it)
    /// - out_degree() == 0 (it StrictlyBinds to no one)
    ///
    /// Orphan rate = isolated_count / total_vertices
    ///
    /// # Arguments
    /// * `graph` - read-only reference to the DiGraph
    /// * `total_vertices` - pre-computed vertex count
    ///
    /// # Returns
    /// f64 in range [0.0, 1.0] representing the orphan ratio
    fn compute_orphan_rate(graph: &DiGraph<(), ()>, total_vertices: usize) -> f64 {
        if total_vertices == 0 {
            return 0.0;
        }

        let orphan_count = graph
            .node_indices()
            .filter(|&node| {
                graph
                    .neighbors_directed(node, petgraph::Direction::Incoming)
                    .count()
                    == 0
                    && graph
                        .neighbors_directed(node, petgraph::Direction::Outgoing)
                        .count()
                        == 0
            })
            .count();

        orphan_count as f64 / total_vertices as f64
    }

    /// Compute Contract Density: overall coupling saturation
    ///
    /// Contract density measures how tightly coupled the graph is.
    /// Formula: total_edges / (total_vertices * (total_vertices - 1))
    ///
    /// - A fully connected graph (complete graph) has density = 1.0
    /// - A tree (no cycles) has density approaching 0.0
    /// - Safely handles graphs with < 2 vertices (returns 0.0)
    ///
    /// # Arguments
    /// * `total_vertices` - total number of nodes in the graph
    /// * `total_edges` - total number of edges (StrictlyBinds relationships)
    ///
    /// # Returns
    /// f64 representing the coupling density
    fn compute_contract_density(total_vertices: usize, total_edges: usize) -> f64 {
        if total_vertices < 2 {
            return 0.0;
        }

        let max_possible_edges = total_vertices * (total_vertices - 1);
        total_edges as f64 / max_possible_edges as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph: DiGraph<(), ()> = DiGraph::new();
        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 0);
        assert_eq!(metrics.total_edges, 0);
        assert_eq!(metrics.orphan_rate, 0.0);
        assert_eq!(metrics.contract_density, 0.0);
        assert!(metrics.cyclic_violations.is_empty());
    }

    #[test]
    fn test_single_vertex_no_orphan() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let _n1 = graph.add_node(());

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 1);
        assert_eq!(metrics.total_edges, 0);
        assert_eq!(metrics.orphan_rate, 1.0); // Single node is an orphan
        assert_eq!(metrics.contract_density, 0.0); // Density undefined for < 2 vertices
        assert!(metrics.cyclic_violations.is_empty());
    }

    #[test]
    fn test_two_vertices_no_edge() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let _n1 = graph.add_node(());
        let _n2 = graph.add_node(());

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 2);
        assert_eq!(metrics.total_edges, 0);
        assert_eq!(metrics.orphan_rate, 1.0); // Both nodes are orphans
        assert_eq!(metrics.contract_density, 0.0); // 0 / (2 * 1) = 0
        assert!(metrics.cyclic_violations.is_empty());
    }

    #[test]
    fn test_linear_chain_no_cycles() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        let n3 = graph.add_node(());

        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n3, ());

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 3);
        assert_eq!(metrics.total_edges, 2);
        assert_eq!(metrics.orphan_rate, 0.0); // No orphans (all have degree >= 1)
        assert_eq!(metrics.contract_density, 2.0 / 6.0); // 2 / (3 * 2)
        assert!(metrics.cyclic_violations.is_empty());
    }

    #[test]
    fn test_simple_cycle() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());

        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n1, ()); // Creates cycle

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 2);
        assert_eq!(metrics.total_edges, 2);
        assert_eq!(metrics.orphan_rate, 0.0);
        assert_eq!(metrics.contract_density, 1.0); // 2 / (2 * 1) = 1.0
        assert_eq!(metrics.cyclic_violations.len(), 1); // One SCC of size 2
        assert_eq!(metrics.cyclic_violations[0].len(), 2);
    }

    #[test]
    fn test_multiple_cycles() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        // First cycle: n1 <-> n2
        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        graph.add_edge(n1, n2, ());
        graph.add_edge(n2, n1, ());

        // Second cycle: n3 -> n4 -> n5 -> n3
        let n3 = graph.add_node(());
        let n4 = graph.add_node(());
        let n5 = graph.add_node(());
        graph.add_edge(n3, n4, ());
        graph.add_edge(n4, n5, ());
        graph.add_edge(n5, n3, ());

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 5);
        assert_eq!(metrics.total_edges, 5);
        assert_eq!(metrics.cyclic_violations.len(), 2);
        assert_eq!(metrics.cyclic_violations[0].len(), 2);
        assert_eq!(metrics.cyclic_violations[1].len(), 3);
    }

    #[test]
    fn test_mixed_orphans_and_edges() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let _orphan1 = graph.add_node(()); // Isolated
        let _orphan2 = graph.add_node(()); // Isolated

        let n1 = graph.add_node(());
        let n2 = graph.add_node(());
        graph.add_edge(n1, n2, ());

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 4);
        assert_eq!(metrics.total_edges, 1);
        assert_eq!(metrics.orphan_rate, 0.5); // 2 orphans out of 4
        assert_eq!(metrics.contract_density, 1.0 / 12.0); // 1 / (4 * 3)
    }

    #[test]
    fn test_complete_graph() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let nodes: Vec<_> = (0..3).map(|_| graph.add_node(())).collect();

        // Add edges from every node to every other node
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    graph.add_edge(nodes[i], nodes[j], ());
                }
            }
        }

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 3);
        assert_eq!(metrics.total_edges, 6); // 3 * (3 - 1) = 6
        assert_eq!(metrics.orphan_rate, 0.0);
        assert_eq!(metrics.contract_density, 1.0); // Complete graph: 6 / 6 = 1.0
    }

    #[test]
    fn test_self_loop_not_cycle_violation() {
        let mut graph: DiGraph<(), ()> = DiGraph::new();
        let n1 = graph.add_node(());
        graph.add_edge(n1, n1, ()); // Self-loop

        let metrics = TopologicalAnalyzer::analyze(&graph);

        assert_eq!(metrics.total_vertices, 1);
        assert_eq!(metrics.total_edges, 1);
        // Self-loop creates an SCC of size 1, so no cycle violations (size > 1)
        assert!(metrics.cyclic_violations.is_empty());
    }
}
