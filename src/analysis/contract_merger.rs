//! Contract Merger: combines AST and AI dependency analysis results
//! 
//! Merges dependencies from multiple analysis sources, deduplicates contracts,
//! and assigns confidence scores based on analysis method reliability.

use crate::types::{Contract, ContractSource, Signatory, ClauseType};
use crate::analysis::ast_analyzer::Dependency;
use std::collections::{HashMap, HashSet};

pub struct ContractMerger;

impl ContractMerger {
    /// Merge dependencies from AST and AI analysis into deduplicated contracts
    /// 
    /// Strategy:
    /// - AST deps get higher confidence (0.90-0.95)
    /// - AI contracts get lower confidence (0.40-0.70)
    /// - If same (from_uri, to_uri) pair exists in both, keep AST version
    /// - Convert Dependency → Contract using signatory URIs
    /// 
    /// # Arguments
    /// * `ast_deps` - Dependencies from AST analysis
    /// * `ai_contracts` - Contracts from AILinker
    /// * `signatories` - Signatories to map URIs to IDs
    /// 
    /// # Returns
    /// Vec of deduplicated Contracts
    pub fn merge_dependencies(
        ast_deps: Vec<Dependency>,
        ai_contracts: Vec<Contract>,
        signatories: &[Signatory],
    ) -> Result<Vec<Contract>, String> {
        // Build a mapping from source_uri to signatory ID
        let uri_to_id: HashMap<String, String> = signatories
            .iter()
            .map(|s| (s.source_uri.clone(), s.id.clone()))
            .collect();

        // Collect AST dependencies as a set for deduplication
        let mut ast_set: HashSet<(String, String)> = HashSet::new();
        let mut contracts_by_pair: HashMap<(String, String), Contract> = HashMap::new();

        // Process AST dependencies first (higher confidence, take precedence)
        for dep in ast_deps {
            if let (Some(from_id), Some(to_id)) = (uri_to_id.get(&dep.from_uri), uri_to_id.get(&dep.to_uri)) {
                let pair_key = (from_id.clone(), to_id.clone());
                ast_set.insert(pair_key.clone());

                // Map dependency type to ClauseType
                let clause_type = match dep.dep_type.as_str() {
                    "import" => ClauseType::Requires,
                    "call" => ClauseType::Calls,
                    "inherit" => ClauseType::Enslaves,
                    "type_ref" => ClauseType::Uses,
                    _ => ClauseType::Uses,
                };

                // Assign high confidence to AST deps
                let confidence = 0.90 + (dep.confidence * 0.05).min(0.05);
                
                let contract = Contract::new(
                    from_id.clone(),
                    to_id.clone(),
                    clause_type,
                    confidence,
                    ContractSource::Deterministic,
                )
                .with_reasoning(format!("AST analysis: {} dependency", dep.dep_type));

                contracts_by_pair.insert(pair_key, contract);
            }
        }

        // Process AI contracts (lower confidence, skip if already in AST)
        for ai_contract in ai_contracts {
            let pair_key = (ai_contract.principal_id.clone(), ai_contract.guarantor_id.clone());

            // Skip if this pair already exists in AST analysis
            if ast_set.contains(&pair_key) {
                continue;
            }

            // Lower the confidence for AI contracts
            let mut ai_contract = ai_contract;
            ai_contract.confidence = (ai_contract.confidence * 0.7).max(0.40).min(0.70);

            contracts_by_pair.insert(pair_key, ai_contract);
        }

        // Return deduplicated contracts
        Ok(contracts_by_pair.into_values().collect())
    }

    /// Count duplicate pairs before merging
    pub fn count_duplicates_from_deps(ast_deps: &[Dependency], ai_deps: &[Dependency]) -> usize {
        let ast_pairs: HashSet<_> = ast_deps
            .iter()
            .map(|d| (d.from_uri.clone(), d.to_uri.clone()))
            .collect();

        ai_deps
            .iter()
            .filter(|d| ast_pairs.contains(&(d.from_uri.clone(), d.to_uri.clone())))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_deduplicates_ast_priority() {
        let ast_deps = vec![
            Dependency::new(
                "file://a.rs".to_string(),
                "file://b.rs".to_string(),
                "import".to_string(),
                0.95,
            ),
        ];

        let ai_contracts = vec![];

        let signatories = vec![
            Signatory::new(
                crate::types::SignatoryType::File,
                "file://a.rs".to_string(),
                "a.rs".to_string(),
                "content".to_string(),
            ),
            Signatory::new(
                crate::types::SignatoryType::File,
                "file://b.rs".to_string(),
                "b.rs".to_string(),
                "content".to_string(),
            ),
        ];

        let contracts = ContractMerger::merge_dependencies(ast_deps, ai_contracts, &signatories)
            .expect("merge should succeed");

        assert_eq!(contracts.len(), 1);
        
        let contract = &contracts[0];
        assert_eq!(contract.discovered_by, ContractSource::Deterministic);
        assert!(contract.confidence >= 0.90);
        assert!(contract.confidence <= 0.95);
    }

    #[test]
    fn test_merge_includes_unique_ai_contracts() {
        let ast_deps = vec![
            Dependency::new(
                "file://a.rs".to_string(),
                "file://b.rs".to_string(),
                "import".to_string(),
                0.95,
            ),
        ];

        let ai_contracts = vec![
            Contract::new(
                "sig_b".to_string(),
                "sig_c".to_string(),
                ClauseType::Uses,
                0.6,
                ContractSource::AiInferred,
            ),
        ];

        let mut signatories = vec![
            Signatory::new(
                crate::types::SignatoryType::File,
                "file://a.rs".to_string(),
                "a.rs".to_string(),
                "content".to_string(),
            ),
            Signatory::new(
                crate::types::SignatoryType::File,
                "file://b.rs".to_string(),
                "b.rs".to_string(),
                "content".to_string(),
            ),
            Signatory::new(
                crate::types::SignatoryType::File,
                "file://c.rs".to_string(),
                "c.rs".to_string(),
                "content".to_string(),
            ),
        ];

        // Manually set IDs to match
        signatories[1].id = "sig_b".to_string();
        signatories[2].id = "sig_c".to_string();

        let contracts = ContractMerger::merge_dependencies(ast_deps, ai_contracts, &signatories)
            .expect("merge should succeed");

        assert_eq!(contracts.len(), 2);

        let ai_contract = contracts
            .iter()
            .find(|c| c.discovered_by == ContractSource::AiInferred)
            .expect("should find AI contract");

        assert!(ai_contract.confidence <= 0.70);
    }
}
