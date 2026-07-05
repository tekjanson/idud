// src/link_tree.rs
//! The Link Tree: a high-performance graph storage for concept mapping

use crate::types::*;
use dashmap::DashMap;
use petgraph::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// The Link Tree: a concurrent, queryable graph of Nodes and Edges
pub struct LinkTree {
    nodes: Arc<DashMap<String, Node>>,
    edges: Arc<DashMap<String, Edge>>,
    graph: Arc<parking_lot::Mutex<StableDiGraph<String, String>>>,
}

impl LinkTree {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            edges: Arc::new(DashMap::new()),
            graph: Arc::new(parking_lot::Mutex::new(StableDiGraph::new())),
        }
    }

    /// Add a node to the tree
    pub fn add_node(&self, node: Node) -> Result<String, String> {
        crate::schemas::SchemaValidator::validate_node(&node)?;
        let node_id = node.id.clone();
        self.nodes.insert(node_id.clone(), node);
        Ok(node_id)
    }

    /// Add an edge to the tree
    pub fn add_edge(&self, edge: Edge) -> Result<String, String> {
        crate::schemas::SchemaValidator::validate_edge(&edge)?;
        let edge_id = edge.id.clone();
        self.edges.insert(edge_id.clone(), edge);
        Ok(edge_id)
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<Node> {
        self.nodes.get(id).map(|r| r.clone())
    }

    /// Get an edge by ID
    pub fn get_edge(&self, id: &str) -> Option<Edge> {
        self.edges.get(id).map(|r| r.clone())
    }

    /// Find all edges from a source node
    pub fn edges_from(&self, source_id: &str) -> Vec<Edge> {
        self.edges
            .iter()
            .filter(|entry| entry.value().source_node_id == source_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Find all edges to a target node
    pub fn edges_to(&self, target_id: &str) -> Vec<Edge> {
        self.edges
            .iter()
            .filter(|entry| entry.value().target_node_id == target_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Traverse from a node up to a max depth
    pub fn traverse_from(&self, start_node_id: &str, max_depth: usize) -> Option<TraversalResult> {
        let start_node = self.get_node(start_node_id)?;
        let mut path = vec![(start_node.clone(), None)];
        let mut visited = std::collections::HashSet::new();
        visited.insert(start_node_id.to_string());

        let mut queue = vec![(start_node_id.to_string(), 0)];

        while let Some((current_id, depth)) = queue.pop() {
            if depth >= max_depth {
                continue;
            }

            for edge in self.edges_from(&current_id) {
                if !visited.contains(&edge.target_node_id) {
                    visited.insert(edge.target_node_id.clone());
                    if let Some(target_node) = self.get_node(&edge.target_node_id) {
                        path.push((target_node, Some(edge.clone())));
                        queue.push((edge.target_node_id, depth + 1));
                    }
                }
            }
        }

        Some(TraversalResult {
            start_node,
            path,
            max_depth,
            total_nodes: visited.len(),
        })
    }

    /// Find all untested code nodes
    pub fn find_untested(&self) -> CoverageReport {
        let mut tested_nodes = std::collections::HashSet::new();

        // Find all nodes connected to test nodes
        for entry in self.edges.iter() {
            let edge = entry.value();
            if edge.edge_type == EdgeType::IsTestedBy || edge.edge_type == EdgeType::Tests {
                tested_nodes.insert(edge.source_node_id.clone());
                tested_nodes.insert(edge.target_node_id.clone());
            }
        }

        // Find untested code nodes
        let untested: Vec<Node> = self
            .nodes
            .iter()
            .filter(|entry| {
                let node = entry.value();
                matches!(node.node_type, NodeType::Function | NodeType::Class)
                    && !tested_nodes.contains(&node.id)
            })
            .map(|entry| entry.value().clone())
            .collect();

        let total_nodes = self.nodes.len();
        let coverage_percent = if total_nodes > 0 {
            ((tested_nodes.len() as f32) / (total_nodes as f32)) * 100.0
        } else {
            0.0
        };

        let gaps = untested
            .iter()
            .map(|n| (n.clone(), "No test found".to_string()))
            .collect();

        CoverageReport {
            tested_nodes: tested_nodes.len(),
            untested,
            coverage_percent,
            gaps,
        }
    }

    /// Generate the AI Cheat Sheet
    pub fn generate_cheat_sheet(&self, entity: &str) -> AICheatSheet {
        let mut concepts = Vec::new();
        let mut workflows = Vec::new();
        let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
        let mut by_label: HashMap<String, String> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Index nodes by type and label
        for entry in self.nodes.iter() {
            let node = entry.value();
            by_label.insert(node.label.clone(), node.id.clone());
            let type_name = format!("{:?}", node.node_type);
            by_type.entry(type_name).or_insert_with(Vec::new).push(node.id.clone());
        }

        // Calculate in-degree (dependency count)
        for entry in self.edges.iter() {
            let edge = entry.value();
            *in_degree.entry(edge.target_node_id.clone()).or_insert(0) += 1;
        }

        let mut top_dependencies: Vec<(String, usize)> = in_degree
            .into_iter()
            .collect();
        top_dependencies.sort_by(|a, b| b.1.cmp(&a.1));
        top_dependencies.truncate(20);

        AICheatSheet {
            entity: entity.to_string(),
            generated_at: chrono::Utc::now(),
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            concepts,
            workflows,
            index: GraphIndex {
                by_type,
                by_label,
                top_dependencies,
            },
        }
    }

    /// Statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.nodes.len(), self.edges.len())
    }
}

impl Default for LinkTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let tree = LinkTree::new();
        let node = Node::new(
            NodeType::Function,
            "repo/func.ts".to_string(),
            "myFunc".to_string(),
            "fn myFunc(){}".to_string(),
        );
        let id = node.id.clone();
        let result = tree.add_node(node);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), id);
        assert!(tree.get_node(&id).is_some());
    }

    #[test]
    fn test_add_edge() {
        let tree = LinkTree::new();
        let edge = Edge::new(
            "node1".to_string(),
            "node2".to_string(),
            EdgeType::DependsOn,
            1.0,
            EdgeSource::Deterministic,
        );
        let result = tree.add_edge(edge);
        assert!(result.is_ok());
    }

    #[test]
    fn test_traverse() {
        let tree = LinkTree::new();

        // Create nodes
        let n1 = Node::new(
            NodeType::Workflow,
            "uri1".to_string(),
            "wf1".to_string(),
            "".to_string(),
        );
        let n1_id = n1.id.clone();
        let n2 = Node::new(
            NodeType::Function,
            "uri2".to_string(),
            "func1".to_string(),
            "".to_string(),
        );
        let n2_id = n2.id.clone();

        tree.add_node(n1).unwrap();
        tree.add_node(n2).unwrap();

        // Create edge
        let edge = Edge::new(n1_id.clone(), n2_id.clone(), EdgeType::Calls, 1.0, EdgeSource::Deterministic);
        tree.add_edge(edge).unwrap();

        // Traverse
        let result = tree.traverse_from(&n1_id, 2);
        assert!(result.is_some());
        let traversal = result.unwrap();
        assert_eq!(traversal.total_nodes, 2);
    }

    #[test]
    fn test_coverage() {
        let tree = LinkTree::new();

        let code = Node::new(
            NodeType::Function,
            "uri".to_string(),
            "func".to_string(),
            "".to_string(),
        );
        let code_id = code.id.clone();

        let test = Node::new(
            NodeType::Test,
            "uri".to_string(),
            "test".to_string(),
            "".to_string(),
        );
        let test_id = test.id.clone();

        tree.add_node(code).unwrap();
        tree.add_node(test).unwrap();

        let edge = Edge::new(code_id, test_id, EdgeType::IsTestedBy, 1.0, EdgeSource::Deterministic);
        tree.add_edge(edge).unwrap();

        let report = tree.find_untested();
        assert_eq!(report.untested.len(), 0);
    }
}
