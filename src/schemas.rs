// src/schemas.rs
//! Link Tree schema factories and validators

use crate::types::*;
use serde_json::json;

/// Node Factory: creates universally typed nodes from any source.
pub struct NodeFactory;

impl NodeFactory {
    /// Create a FILE node from filesystem path
    pub fn create_file_node(
        repo_uri: &str,
        file_path: &str,
        branch: &str,
    ) -> Node {
        let source_uri = format!("{}/blob/{}/{}", repo_uri, branch, file_path);
        Node::new(
            NodeType::File,
            source_uri,
            file_path.to_string(),
            format!("File: {}", file_path),
        )
        .with_metadata("filePath".to_string(), json!(file_path))
        .with_metadata("repoUri".to_string(), json!(repo_uri))
        .with_metadata("branch".to_string(), json!(branch))
    }

    /// Create a FUNCTION node from code analysis
    pub fn create_function_node(
        repo_uri: &str,
        file_path: &str,
        function_name: &str,
        snippet: String,
        line_start: usize,
        line_end: usize,
        branch: &str,
    ) -> Node {
        let source_uri = format!(
            "{}/blob/{}/{}#L{}-L{}",
            repo_uri, branch, file_path, line_start, line_end
        );
        Node::new(
            NodeType::Function,
            source_uri,
            function_name.to_string(),
            snippet,
        )
        .with_metadata("filePath".to_string(), json!(file_path))
        .with_metadata("functionName".to_string(), json!(function_name))
        .with_metadata("lineStart".to_string(), json!(line_start))
        .with_metadata("lineEnd".to_string(), json!(line_end))
    }

    /// Create a TEST node from test file
    pub fn create_test_node(
        repo_uri: &str,
        test_file_path: &str,
        test_name: &str,
        snippet: String,
        line_start: usize,
        line_end: usize,
        branch: &str,
    ) -> Node {
        let source_uri = format!(
            "{}/blob/{}/{}#L{}-L{}",
            repo_uri, branch, test_file_path, line_start, line_end
        );
        Node::new(
            NodeType::Test,
            source_uri,
            format!("{} ({})", test_name, test_file_path),
            snippet,
        )
        .with_metadata("testFile".to_string(), json!(test_file_path))
        .with_metadata("testName".to_string(), json!(test_name))
    }

    /// Create an API_ENDPOINT node from route definition
    pub fn create_api_endpoint_node(
        repo_uri: &str,
        file_path: &str,
        method: &str,
        path: &str,
        snippet: String,
        line_start: usize,
        branch: &str,
    ) -> Node {
        let source_uri = format!(
            "{}/blob/{}/{}#L{}",
            repo_uri, branch, file_path, line_start
        );
        Node::new(
            NodeType::ApiEndpoint,
            source_uri,
            format!("{} {}", method, path),
            snippet,
        )
        .with_metadata("method".to_string(), json!(method))
        .with_metadata("path".to_string(), json!(path))
    }

    /// Create a MARKDOWN_SECTION node from documentation
    pub fn create_doc_node(
        doc_uri: &str,
        section: &str,
        heading: &str,
        snippet: String,
    ) -> Node {
        let source_uri = format!("{}#{}", doc_uri, section);
        Node::new(
            NodeType::MarkdownSection,
            source_uri,
            heading.to_string(),
            snippet,
        )
        .with_metadata("docUri".to_string(), json!(doc_uri))
        .with_metadata("section".to_string(), json!(section))
    }

    /// Create a CONCEPT node (high-level abstraction)
    pub fn create_concept_node(name: &str, description: String) -> Node {
        Node::new(
            NodeType::Concept,
            "synthetic://concept".to_string(),
            name.to_string(),
            description,
        )
        .with_metadata("name".to_string(), json!(name))
    }
}

/// Edge Factory: creates universally typed edges capturing relationships.
pub struct EdgeFactory;

impl EdgeFactory {
    /// Create a DEPENDS_ON edge: A requires B to function
    pub fn create_dependency_edge(
        source_node_id: String,
        target_node_id: String,
        confidence: f32,
        source: EdgeSource,
        reasoning: Option<String>,
    ) -> Edge {
        let mut edge = Edge::new(
            source_node_id,
            target_node_id,
            EdgeType::DependsOn,
            confidence,
            source,
        );
        if let Some(r) = reasoning {
            edge = edge.with_reasoning(r);
        }
        edge
    }

    /// Create an IS_TESTED_BY edge: A is covered by test B
    pub fn create_test_edge(code_node_id: String, test_node_id: String, confidence: f32) -> Edge {
        Edge::new(
            code_node_id,
            test_node_id,
            EdgeType::IsTestedBy,
            confidence,
            EdgeSource::Deterministic,
        )
    }

    /// Create a CALLS edge: A invokes B
    pub fn create_call_edge(
        caller_node_id: String,
        callee_node_id: String,
        confidence: f32,
        source: EdgeSource,
    ) -> Edge {
        Edge::new(
            caller_node_id,
            callee_node_id,
            EdgeType::Calls,
            confidence,
            source,
        )
    }

    /// Create an ENSLAVES edge: A's changes require B to change (high coupling)
    pub fn create_enslavement_edge(
        concept_node_id: String,
        enslaved_concept_id: String,
        confidence: f32,
    ) -> Edge {
        Edge::new(
            concept_node_id,
            enslaved_concept_id,
            EdgeType::Enslaves,
            confidence,
            EdgeSource::AiInferred,
        )
        .with_reasoning(
            "High coupling: changes to source concept force changes to target".to_string(),
        )
    }

    /// Create a DOCUMENTS edge: Doc A explains implementation detail B
    pub fn create_documentation_edge(
        doc_node_id: String,
        implementation_node_id: String,
    ) -> Edge {
        Edge::new(
            doc_node_id,
            implementation_node_id,
            EdgeType::Documents,
            1.0,
            EdgeSource::Deterministic,
        )
    }

    /// Create a USES edge: A utilizes capability B
    pub fn create_usage_edge(
        consumer_node_id: String,
        provider_id: String,
        confidence: f32,
        source: EdgeSource,
    ) -> Edge {
        Edge::new(
            consumer_node_id,
            provider_id,
            EdgeType::Uses,
            confidence,
            source,
        )
    }
}

/// Schema validation: ensure all Nodes and Edges conform to expected structure
pub struct SchemaValidator;

impl SchemaValidator {
    pub fn validate_node(node: &Node) -> Result<(), String> {
        if node.id.is_empty() {
            return Err("Node ID cannot be empty".to_string());
        }
        if node.source_uri.is_empty() {
            return Err("Node source_uri cannot be empty".to_string());
        }
        if node.label.is_empty() {
            return Err("Node label cannot be empty".to_string());
        }
        Ok(())
    }

    pub fn validate_edge(edge: &Edge) -> Result<(), String> {
        if edge.id.is_empty() {
            return Err("Edge ID cannot be empty".to_string());
        }
        if edge.source_node_id.is_empty() {
            return Err("Edge source_node_id cannot be empty".to_string());
        }
        if edge.target_node_id.is_empty() {
            return Err("Edge target_node_id cannot be empty".to_string());
        }
        if edge.confidence_score < 0.0 || edge.confidence_score > 1.0 {
            return Err("Edge confidence_score must be between 0 and 1".to_string());
        }
        Ok(())
    }
}
