// src/types.rs
//! Universal graph types for the Link Tree architecture.
//! Pure index model: stores pointers, not knowledge.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    File,
    Function,
    Class,
    Test,
    Workflow,
    Concept,
    ApiEndpoint,
    MarkdownSection,
    DecisionRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    Implements,
    Tests,
    DependsOn,
    IsTestedBy,
    Calls,
    CalledBy,
    Documents,
    Uses,
    Enslaves,
    EnslavedBy,
}

/// Universal Node: the fundamental unit in the Link Tree.
/// Every entity (code, documentation, test) is a Node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    /// Pirate Bay link back to the exact GitHub repo/branch/line
    pub source_uri: String,
    pub label: String,
    /// Raw text snippet for LLM analysis
    pub snippet: String,
    pub extracted: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Node {
    pub fn new(
        node_type: NodeType,
        source_uri: String,
        label: String,
        snippet: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            node_type,
            source_uri,
            label,
            snippet,
            extracted: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Universal Edge: a link between two Nodes.
/// Represents discovered relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub edge_type: EdgeType,
    /// 0-1: how sure the AI/parser is about this link
    pub confidence_score: f32,
    pub source: EdgeSource,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub reasoning: Option<String>,
    pub proofs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeSource {
    Deterministic,
    AiInferred,
}

impl Edge {
    pub fn new(
        source_node_id: String,
        target_node_id: String,
        edge_type: EdgeType,
        confidence_score: f32,
        source: EdgeSource,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_node_id,
            target_node_id,
            edge_type,
            confidence_score: confidence_score.max(0.0).min(1.0),
            source,
            discovered_at: chrono::Utc::now(),
            reasoning: None,
            proofs: vec![],
        }
    }

    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = Some(reasoning);
        self
    }

    pub fn with_proof(mut self, proof: String) -> Self {
        self.proofs.push(proof);
        self
    }
}

/// Graph Query Result: a traversal through the tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub start_node: Node,
    pub path: Vec<(Node, Option<Edge>)>,
    pub max_depth: usize,
    pub total_nodes: usize,
}

/// Coverage Report: identifies gaps in the Link Tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub tested_nodes: usize,
    pub untested: Vec<Node>,
    pub coverage_percent: f32,
    pub gaps: Vec<(Node, String)>,
}

/// AI Cheat Sheet: compressed, queryable snapshot of the Link Tree.
/// Loaded into AI context to avoid token waste during traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICheatSheet {
    pub entity: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub node_count: usize,
    pub edge_count: usize,
    pub concepts: Vec<ConceptEntry>,
    pub workflows: Vec<WorkflowEntry>,
    pub index: GraphIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptEntry {
    pub id: String,
    pub name: String,
    pub depends_on: Vec<String>,
    pub tested_by: Vec<String>,
    pub documents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub name: String,
    pub concepts: Vec<String>,
    pub critical_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphIndex {
    pub by_type: HashMap<String, Vec<String>>,
    pub by_label: HashMap<String, String>,
    pub top_dependencies: Vec<(String, usize)>,
}
