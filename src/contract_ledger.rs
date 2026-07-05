// src/contract_ledger.rs
//! The Contract Ledger: an immutable, queryable registry of software contracts and bindings
//! Signatories: entities (files, functions, tests) that enter into contractual obligations
//! Contracts: discovered bindings between signatories

use crate::types::*;
use dashmap::DashMap;
use petgraph::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// The Contract Ledger: high-performance registry of signatories and contracts
pub struct ContractLedger {
    signatories: Arc<DashMap<String, Signatory>>,
    contracts: Arc<DashMap<String, Contract>>,
    graph: Arc<parking_lot::Mutex<StableDiGraph<String, String>>>,
}

impl ContractLedger {
    pub fn new() -> Self {
        Self {
            signatories: Arc::new(DashMap::new()),
            contracts: Arc::new(DashMap::new()),
            graph: Arc::new(parking_lot::Mutex::new(StableDiGraph::new())),
        }
    }

    /// Register a signatory in the ledger
    pub fn register_signatory(&self, signatory: Signatory) -> Result<String, String> {
        crate::schemas::ContractValidator::audit_signatory(&signatory)?;
        let signatory_id = signatory.id.clone();
        self.signatories.insert(signatory_id.clone(), signatory);
        Ok(signatory_id)
    }

    /// Draft a contract binding between signatories
    pub fn draft_contract(&self, contract: Contract) -> Result<String, String> {
        crate::schemas::ContractValidator::audit_contract(&contract)?;
        let contract_id = contract.id.clone();
        self.contracts.insert(contract_id.clone(), contract);
        Ok(contract_id)
    }

    /// Retrieve a signatory by ID
    pub fn get_signatory(&self, id: &str) -> Option<Signatory> {
        self.signatories.get(id).map(|r| r.clone())
    }

    /// Retrieve a contract by ID
    pub fn get_contract(&self, id: &str) -> Option<Contract> {
        self.contracts.get(id).map(|r| r.clone())
    }

    /// Find all contracts where a signatory is the principal
    pub fn get_obligations(&self, principal_id: &str) -> Vec<Contract> {
        self.contracts
            .iter()
            .filter(|entry| entry.value().principal_id == principal_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Find all contracts where a signatory is the guarantor
    pub fn get_guarantees(&self, guarantor_id: &str) -> Vec<Contract> {
        self.contracts
            .iter()
            .filter(|entry| entry.value().guarantor_id == guarantor_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Trace a chain of obligation: follow contractual bindings
    pub fn trace_chain_of_obligation(
        &self,
        start_signatory_id: &str,
        max_depth: usize,
    ) -> Option<ChainOfObligation> {
        let start_signatory = self.get_signatory(start_signatory_id)?;
        let mut chain = vec![(start_signatory.clone(), None)];
        let mut visited = std::collections::HashSet::new();
        visited.insert(start_signatory_id.to_string());

        let mut queue = vec![(start_signatory_id.to_string(), 0)];

        while let Some((current_id, depth)) = queue.pop() {
            if depth >= max_depth {
                continue;
            }

            for contract in self.get_obligations(&current_id) {
                if !visited.contains(&contract.guarantor_id) {
                    visited.insert(contract.guarantor_id.clone());
                    if let Some(target_signatory) = self.get_signatory(&contract.guarantor_id) {
                        chain.push((target_signatory, Some(contract.clone())));
                        queue.push((contract.guarantor_id, depth + 1));
                    }
                }
            }
        }

        Some(ChainOfObligation {
            root_signatory: start_signatory,
            chain,
            max_depth,
            total_signatories: visited.len(),
        })
    }

    /// Audit for contract violations: signatories without audit coverage
    pub fn audit_contract_coverage(&self) -> ContractAuditReport {
        let mut audited_signatories = std::collections::HashSet::new();

        // Find all signatories involved in audit contracts
        for entry in self.contracts.iter() {
            let contract = entry.value();
            if contract.clause_type == ClauseType::Audits {
                audited_signatories.insert(contract.principal_id.clone());
                audited_signatories.insert(contract.guarantor_id.clone());
            }
        }

        // Find unaudited code signatories
        let unaudited: Vec<Signatory> = self
            .signatories
            .iter()
            .filter(|entry| {
                let signatory = entry.value();
                matches!(
                    signatory.signatory_type,
                    SignatoryType::Function | SignatoryType::Class
                ) && !audited_signatories.contains(&signatory.id)
            })
            .map(|entry| entry.value().clone())
            .collect();

        let total_signatories = self.signatories.len();
        let audit_coverage_percent = if total_signatories > 0 {
            ((audited_signatories.len() as f32) / (total_signatories as f32)) * 100.0
        } else {
            0.0
        };

        let violations = unaudited
            .iter()
            .map(|s| (s.clone(), "No audit contract found".to_string()))
            .collect();

        ContractAuditReport {
            audited_signatories: audited_signatories.len(),
            unaudited,
            audit_coverage_percent,
            violations,
        }
    }

    /// Generate AI Contract Brief: compressed snapshot for LLM context
    pub fn generate_contract_brief(&self, entity: &str) -> AIContractBrief {
        let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
        let mut by_label: HashMap<String, String> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Index signatories by type and label
        for entry in self.signatories.iter() {
            let signatory = entry.value();
            by_label.insert(signatory.label.clone(), signatory.id.clone());
            let type_name = format!("{:?}", signatory.signatory_type);
            by_type
                .entry(type_name)
                .or_insert_with(Vec::new)
                .push(signatory.id.clone());
        }

        // Calculate in-degree (obligation count)
        for entry in self.contracts.iter() {
            let contract = entry.value();
            *in_degree.entry(contract.guarantor_id.clone()).or_insert(0) += 1;
        }

        let mut most_obligated: Vec<(String, usize)> = in_degree.into_iter().collect();
        most_obligated.sort_by(|a, b| b.1.cmp(&a.1));
        most_obligated.truncate(20);

        AIContractBrief {
            entity: entity.to_string(),
            generated_at: chrono::Utc::now(),
            signatory_count: self.signatories.len(),
            contract_count: self.contracts.len(),
            conceptual_contracts: vec![],
            workflow_bindings: vec![],
            ledger_index: LedgerIndex {
                by_type,
                by_label,
                most_obligated,
            },
        }
    }

    /// Statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.signatories.len(), self.contracts.len())
    }
}

impl Default for ContractLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_signatory() {
        let ledger = ContractLedger::new();
        let signatory = Signatory::new(
            SignatoryType::Function,
            "repo/func.ts".to_string(),
            "myFunc".to_string(),
            "fn myFunc(){}".to_string(),
        );
        let id = signatory.id.clone();
        let result = ledger.register_signatory(signatory);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), id);
        assert!(ledger.get_signatory(&id).is_some());
    }

    #[test]
    fn test_draft_contract() {
        let ledger = ContractLedger::new();
        let contract = Contract::new(
            "principal1".to_string(),
            "guarantor1".to_string(),
            ClauseType::Requires,
            1.0,
            ContractSource::Deterministic,
        );
        let result = ledger.draft_contract(contract);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_chain_of_obligation() {
        let ledger = ContractLedger::new();

        let s1 = Signatory::new(
            SignatoryType::Workflow,
            "uri1".to_string(),
            "wf1".to_string(),
            "".to_string(),
        );
        let s1_id = s1.id.clone();

        let s2 = Signatory::new(
            SignatoryType::Function,
            "uri2".to_string(),
            "func1".to_string(),
            "".to_string(),
        );
        let s2_id = s2.id.clone();

        ledger.register_signatory(s1).unwrap();
        ledger.register_signatory(s2).unwrap();

        let contract = Contract::new(
            s1_id.clone(),
            s2_id.clone(),
            ClauseType::Calls,
            1.0,
            ContractSource::Deterministic,
        );
        ledger.draft_contract(contract).unwrap();

        let result = ledger.trace_chain_of_obligation(&s1_id, 2);
        assert!(result.is_some());
        let chain = result.unwrap();
        assert_eq!(chain.total_signatories, 2);
    }

    #[test]
    fn test_audit_coverage() {
        let ledger = ContractLedger::new();

        let code = Signatory::new(
            SignatoryType::Function,
            "uri".to_string(),
            "func".to_string(),
            "".to_string(),
        );
        let code_id = code.id.clone();

        let test = Signatory::new(
            SignatoryType::Test,
            "uri".to_string(),
            "test".to_string(),
            "".to_string(),
        );
        let test_id = test.id.clone();

        ledger.register_signatory(code).unwrap();
        ledger.register_signatory(test).unwrap();

        let contract = Contract::new(code_id, test_id, ClauseType::Audits, 1.0, ContractSource::Deterministic);
        ledger.draft_contract(contract).unwrap();

        let report = ledger.audit_contract_coverage();
        assert_eq!(report.unaudited.len(), 0);
    }
}

