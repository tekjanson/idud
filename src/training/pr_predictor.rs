//! PR file change predictor using Waymark dependency contracts
//!
//! Predicts which files change together based on dependency graph.
//! Core idea: files that are contractually bound (imports, calls, uses)
//! are likely to change together.

use crate::types::{ClauseType, Contract, Signatory};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// A file change prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePrediction {
    /// Files that were edited (input)
    pub changed_files: Vec<String>,
    /// Files predicted to also need changes
    pub predicted_files: Vec<String>,
    /// Confidence scores for each prediction (0-1)
    pub confidence_scores: HashMap<String, f32>,
    /// Graph edges traversed to make predictions
    pub traversal_depth: usize,
    /// Total files reachable from changed set
    pub total_reachable: usize,
    /// Computation time in ms
    pub compute_time_ms: u128,
}

/// Co-dependency graph built from Waymark contracts
#[derive(Debug, Clone)]
pub struct CoDependencyGraph {
    /// Map from file ID to all contracts where it's principal
    principal_contracts: HashMap<String, Vec<Contract>>,
    /// Map from file ID to all contracts where it's guarantor
    guarantor_contracts: HashMap<String, Vec<Contract>>,
    /// Map from file ID to source URI (reverse lookup)
    id_to_uri: HashMap<String, String>,
    /// Map from source URI to ID
    uri_to_id: HashMap<String, String>,
    /// Total signatories and contracts in graph
    pub total_signatories: usize,
    pub total_contracts: usize,
}

impl CoDependencyGraph {
    /// Build graph from signatories and contracts
    pub fn build(signatories: Vec<Signatory>, contracts: Vec<Contract>) -> Self {
        let mut principal_contracts = HashMap::new();
        let mut guarantor_contracts = HashMap::new();
        let mut id_to_uri = HashMap::new();
        let mut uri_to_id = HashMap::new();

        // Index signatories
        for sig in signatories.iter() {
            id_to_uri.insert(sig.id.clone(), sig.source_uri.clone());
            uri_to_id.insert(sig.source_uri.clone(), sig.id.clone());
        }

        // Index contracts by principal and guarantor
        for contract in contracts.iter() {
            principal_contracts
                .entry(contract.principal_id.clone())
                .or_insert_with(Vec::new)
                .push(contract.clone());

            guarantor_contracts
                .entry(contract.guarantor_id.clone())
                .or_insert_with(Vec::new)
                .push(contract.clone());
        }

        Self {
            principal_contracts,
            guarantor_contracts,
            id_to_uri,
            uri_to_id,
            total_signatories: signatories.len(),
            total_contracts: contracts.len(),
        }
    }

    /// Get all contracts where a file is the principal (has obligations to)
    pub fn get_principal_contracts(&self, file_id: &str) -> Vec<&Contract> {
        self.principal_contracts
            .get(file_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get all contracts where a file is the guarantor (is obligated to by)
    pub fn get_guarantor_contracts(&self, file_id: &str) -> Vec<&Contract> {
        self.guarantor_contracts
            .get(file_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Convert source URI to internal ID
    pub fn uri_to_id(&self, uri: &str) -> Option<String> {
        self.uri_to_id.get(uri).cloned()
    }

    /// Convert internal ID to source URI
    pub fn id_to_uri(&self, id: &str) -> Option<String> {
        self.id_to_uri.get(id).cloned()
    }
}

/// Predicts file changes using dependency graph
pub struct PRPredictor {
    graph: CoDependencyGraph,
}

impl PRPredictor {
    pub fn new(graph: CoDependencyGraph) -> Self {
        Self { graph }
    }

    /// Predict files that change together
    ///
    /// Strategy:
    /// 1. For each changed file, find all contract obligations
    /// 2. Traverse contracts to connected files
    /// 3. Rank by connection strength (confidence + type)
    /// 4. Return top predictions
    pub fn predict(&self, changed_files: Vec<String>, max_predictions: usize) -> FilePrediction {
        let start = Instant::now();

        // Convert URIs to IDs
        let changed_ids: Vec<String> = changed_files
            .iter()
            .filter_map(|uri| self.graph.uri_to_id(uri))
            .collect();

        let mut all_reachable = HashSet::new();
        let mut predicted_with_scores: HashMap<String, f32> = HashMap::new();
        let mut traversal_depth = 0;

        // BFS traversal to find related files
        let mut queue = changed_ids.clone();
        let mut visited: HashSet<String> = HashSet::from_iter(changed_ids.clone());
        let mut depth = 0;

        while !queue.is_empty() && depth < 3 {
            let mut next_queue = Vec::new();
            let _queue_len = queue.len();

            for file_id in queue.iter() {
                // Get all contracts (both as principal and guarantor)
                let principal = self.graph.get_principal_contracts(file_id);
                let guarantor = self.graph.get_guarantor_contracts(file_id);

                // Process principal contracts (files this file depends on)
                for contract in principal {
                    if !visited.contains(&contract.guarantor_id) {
                        visited.insert(contract.guarantor_id.clone());
                        all_reachable.insert(contract.guarantor_id.clone());
                        next_queue.push(contract.guarantor_id.clone());

                        // Score based on confidence and clause type
                        let score = self.score_contract(contract, depth);
                        predicted_with_scores
                            .entry(contract.guarantor_id.clone())
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }

                // Process guarantor contracts (files that depend on this file)
                for contract in guarantor {
                    if !visited.contains(&contract.principal_id) {
                        visited.insert(contract.principal_id.clone());
                        all_reachable.insert(contract.principal_id.clone());
                        next_queue.push(contract.principal_id.clone());

                        // Score based on confidence and clause type
                        let score = self.score_contract(contract, depth);
                        predicted_with_scores
                            .entry(contract.principal_id.clone())
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }
            }

            queue = next_queue;
            depth += 1;
            traversal_depth = depth;
        }

        // Convert IDs back to URIs
        let mut predicted_files: Vec<(String, f32)> = predicted_with_scores
            .iter()
            .filter_map(|(id, score)| self.graph.id_to_uri(id).map(|uri| (uri, *score)))
            .collect();

        // Sort by score descending
        predicted_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to max predictions
        let top_predictions: Vec<String> = predicted_files
            .iter()
            .take(max_predictions)
            .map(|(uri, _)| uri.clone())
            .collect();

        let confidence_map: HashMap<String, f32> = predicted_files
            .iter()
            .map(|(uri, score)| (uri.clone(), *score))
            .collect();

        FilePrediction {
            changed_files,
            predicted_files: top_predictions,
            confidence_scores: confidence_map,
            traversal_depth,
            total_reachable: all_reachable.len(),
            compute_time_ms: start.elapsed().as_millis(),
        }
    }

    /// Score a contract based on clause type and confidence
    fn score_contract(&self, contract: &Contract, depth: usize) -> f32 {
        let base_score = contract.confidence;

        // Boost score for high-coupling relationships
        let clause_multiplier = match contract.clause_type {
            ClauseType::Requires | ClauseType::RequiredBy => 1.2,
            ClauseType::Calls | ClauseType::CalledBy => 1.1,
            ClauseType::Implements => 1.3,
            ClauseType::Uses => 1.0,
            ClauseType::Enslaves | ClauseType::EnslavedBy => 1.5,
            _ => 0.9,
        };

        // Reduce score with depth (far away files less likely to change)
        let depth_factor = 1.0 / (1.0 + (depth as f32) * 0.3);

        (base_score * clause_multiplier * depth_factor).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContractSource, SignatoryType};
    use chrono::Utc;

    fn create_test_signatory(id: &str, source_uri: &str) -> Signatory {
        Signatory {
            id: id.to_string(),
            signatory_type: SignatoryType::File,
            source_uri: source_uri.to_string(),
            label: source_uri.to_string(),
            snippet: "".to_string(),
            registered_at: Utc::now(),
            metadata: Default::default(),
        }
    }

    fn create_test_contract(
        principal_id: &str,
        guarantor_id: &str,
        clause_type: ClauseType,
        confidence: f32,
    ) -> Contract {
        Contract {
            id: uuid::Uuid::new_v4().to_string(),
            principal_id: principal_id.to_string(),
            guarantor_id: guarantor_id.to_string(),
            clause_type,
            confidence,
            discovered_by: ContractSource::Deterministic,
            discovered_at: Utc::now(),
            clause_reasoning: None,
            evidential_proofs: vec![],
        }
    }

    #[test]
    fn test_graph_build() {
        let signatories = vec![
            create_test_signatory("1", "src/main.rs"),
            create_test_signatory("2", "src/lib.rs"),
            create_test_signatory("3", "src/utils.rs"),
        ];

        let contracts = vec![
            create_test_contract("1", "2", ClauseType::Requires, 0.9),
            create_test_contract("2", "3", ClauseType::Uses, 0.8),
        ];

        let graph = CoDependencyGraph::build(signatories, contracts);
        assert_eq!(graph.total_signatories, 3);
        assert_eq!(graph.total_contracts, 2);
    }

    #[test]
    fn test_prediction_simple_chain() {
        let signatories = vec![
            create_test_signatory("1", "src/main.rs"),
            create_test_signatory("2", "src/lib.rs"),
            create_test_signatory("3", "src/utils.rs"),
        ];

        let contracts = vec![
            create_test_contract("1", "2", ClauseType::Requires, 0.9),
            create_test_contract("2", "3", ClauseType::Uses, 0.8),
        ];

        let graph = CoDependencyGraph::build(signatories, contracts);
        let predictor = PRPredictor::new(graph);

        let prediction = predictor.predict(vec!["src/main.rs".to_string()], 10);

        // Should predict src/lib.rs (direct dependency)
        assert!(prediction
            .predicted_files
            .contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn test_prediction_empty_changes() {
        let signatories = vec![
            create_test_signatory("1", "src/main.rs"),
            create_test_signatory("2", "src/lib.rs"),
        ];

        let contracts = vec![create_test_contract("1", "2", ClauseType::Requires, 0.9)];

        let graph = CoDependencyGraph::build(signatories, contracts);
        let predictor = PRPredictor::new(graph);

        let prediction = predictor.predict(vec![], 10);
        assert!(prediction.predicted_files.is_empty());
    }

    #[test]
    fn test_prediction_scoring() {
        let signatories = vec![
            create_test_signatory("1", "src/main.rs"),
            create_test_signatory("2", "src/lib.rs"),
            create_test_signatory("3", "src/utils.rs"),
        ];

        let contracts = vec![
            // High confidence Requires
            create_test_contract("1", "2", ClauseType::Requires, 0.95),
            // Lower confidence Uses
            create_test_contract("1", "3", ClauseType::Uses, 0.6),
        ];

        let graph = CoDependencyGraph::build(signatories, contracts);
        let predictor = PRPredictor::new(graph);

        let prediction = predictor.predict(vec!["src/main.rs".to_string()], 10);

        // src/lib.rs should have higher score than src/utils.rs
        let lib_score = prediction
            .confidence_scores
            .get("src/lib.rs")
            .unwrap_or(&0.0);
        let utils_score = prediction
            .confidence_scores
            .get("src/utils.rs")
            .unwrap_or(&0.0);
        assert!(lib_score > utils_score);
    }
}
