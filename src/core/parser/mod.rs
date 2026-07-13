use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use petgraph::stable_graph::{NodeIndex, StableGraph};
use tree_sitter::{Node, Parser};

use crate::core::pointers::GraphPointer;

/// A lightweight wrapper around tree-sitter that turns Rust syntax into a petgraph topology.
#[derive(Debug, Clone)]
pub struct TreeSitterParser {
    language: tree_sitter::Language,
}

impl TreeSitterParser {
    /// Create a parser configured for Rust sources.
    pub fn new() -> Result<Self> {
        Ok(Self {
            language: tree_sitter_rust::LANGUAGE.into(),
        })
    }

    /// Parse a source string and build a graph of structural nodes and parent/child edges.
    pub fn ingest_source(
        &self,
        source: &str,
        path: impl AsRef<Path>,
    ) -> Result<StableGraph<GraphNode, GraphEdge>> {
        let path = path.as_ref();
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .context("failed to configure tree-sitter for Rust")?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("tree-sitter failed to parse source"))?;

        let mut graph = StableGraph::new();
        let file_pointer = GraphPointer::file(path);
        let file_node = graph.add_node(GraphNode::from_pointer(file_pointer, "file", path.to_path_buf()));

        let root = tree.root_node();
        self.walk_node(&root, source, path, file_node, &mut graph, None)?;

        Ok(graph)
    }

    fn walk_node(
        &self,
        node: &Node,
        source: &str,
        path: &Path,
        parent: NodeIndex,
        graph: &mut StableGraph<GraphNode, GraphEdge>,
        parent_hash: Option<String>,
    ) -> Result<()> {
        let structural_hash = self.hash_node(node, source, parent_hash.as_deref())?;
        let pointer = GraphPointer::from_digest(path, node.kind(), &structural_hash);
        let node_index = graph.add_node(GraphNode::new(
            pointer,
            node.kind().to_string(),
            path.to_path_buf(),
            Some(node.start_byte()),
            Some(node.end_byte()),
        ));
        graph.add_edge(parent, node_index, GraphEdge::Contains);

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "comment" {
                continue;
            }

            self.walk_node(&child, source, path, node_index, graph, Some(structural_hash.clone()))?;
        }

        Ok(())
    }

    fn hash_node(&self, node: &Node, source: &str, parent_digest: Option<&str>) -> Result<String> {
        let mut pieces = Vec::new();
        pieces.push(node.kind().to_string());

        if let Some(parent_digest) = parent_digest {
            pieces.push(parent_digest.to_string());
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "comment" {
                continue;
            }

            let child_digest = self.hash_node(&child, source, Some(&pieces.join("::")))?;
            pieces.push(child_digest);
        }

        if node.child_count() == 0 {
            let trimmed = node
                .utf8_text(source.as_bytes())
                .unwrap_or_default()
                .trim()
                .to_string();
            if !trimmed.is_empty() {
                pieces.push(trimmed);
            }
        }

        let seed = pieces.join("::");
        Ok(blake3::hash(seed.as_bytes()).to_hex().to_string())
    }
}

/// A graph node derived from a tree-sitter AST node.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub pointer: GraphPointer,
    pub kind: String,
    pub source_path: PathBuf,
    pub start_byte: Option<usize>,
    pub end_byte: Option<usize>,
}

impl GraphNode {
    pub fn new(
        pointer: GraphPointer,
        kind: String,
        source_path: PathBuf,
        start_byte: Option<usize>,
        end_byte: Option<usize>,
    ) -> Self {
        Self {
            pointer,
            kind,
            source_path,
            start_byte,
            end_byte,
        }
    }

    pub fn from_pointer(pointer: GraphPointer, kind: impl Into<String>, source_path: PathBuf) -> Self {
        Self {
            pointer: pointer.clone(),
            kind: kind.into(),
            source_path,
            start_byte: None,
            end_byte: None,
        }
    }
}

/// An edge between two structural AST nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEdge {
    Contains,
    References,
}

pub use tree_sitter::Tree as TreeSitterTree;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_source_into_a_graph() {
        let parser = TreeSitterParser::new().unwrap();
        let graph = parser.ingest_source("fn main() { println!(\"hi\"); }", "src/lib.rs").unwrap();

        assert!(graph.node_count() > 1);
        assert!(graph.edge_count() > 0);
    }
}
