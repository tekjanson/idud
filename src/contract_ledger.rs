// src/contract_ledger.rs
//! The Contract Ledger: a pure topological index optimized for zero-traversal costs
//! Uses petgraph::DiGraph for O(1) neighbor lookups and deterministic path tracing
//! Signatories are indexed by UUID→NodeIndex for lightning-fast graph traversal

use crate::types::*;
use parking_lot::RwLock;
use petgraph::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
struct LedgerState {
    uuid_to_node: HashMap<String, NodeIndex>,
    node_data: HashMap<NodeIndex, Signatory>,
    edge_metadata: HashMap<(NodeIndex, NodeIndex), Contract>,
    graph: DiGraph<(), ()>,
}

/// Pure topological index: O(1) signatory UUID lookup, O(1) neighbor traversal
pub struct ContractLedger {
    state: Arc<RwLock<LedgerState>>,
}

impl ContractLedger {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(LedgerState {
                uuid_to_node: HashMap::new(),
                node_data: HashMap::new(),
                edge_metadata: HashMap::new(),
                graph: DiGraph::new(),
            })),
        }
    }

    /// O(1) signatory registration: insert into graph, map UUID→NodeIndex
    pub fn register_signatory(&self, signatory: Signatory) -> Result<String, String> {
        crate::schemas::ContractValidator::audit_signatory(&signatory)?;
        let signatory_id = signatory.id.clone();

        let mut state = self.state.write();

        // Add node to graph (always succeeds, returns NodeIndex)
        let node_idx = state.graph.add_node(());

        // Map UUID → NodeIndex and store signatory data under the same lock.
        state.uuid_to_node.insert(signatory_id.clone(), node_idx);
        state.node_data.insert(node_idx, signatory);

        Ok(signatory_id)
    }

    /// Draft a contract binding: O(1) edge insertion with metadata
    pub fn draft_contract(&self, contract: Contract) -> Result<String, String> {
        crate::schemas::ContractValidator::audit_contract(&contract)?;
        let contract_id = contract.id.clone();

        let mut state = self.state.write();

        // Lookup both signatories' NodeIndex.
        let principal_node = *state
            .uuid_to_node
            .get(&contract.principal_id)
            .ok_or_else(|| format!("Signatory {} not found", contract.principal_id))?;
        let guarantor_node = *state
            .uuid_to_node
            .get(&contract.guarantor_id)
            .ok_or_else(|| format!("Signatory {} not found", contract.guarantor_id))?;

        // Add edge to graph and store metadata under the same lock to avoid a
        // visibility gap between the two state updates.
        state.graph.add_edge(principal_node, guarantor_node, ());
        state
            .edge_metadata
            .insert((principal_node, guarantor_node), contract);

        Ok(contract_id)
    }

    /// O(1) signatory lookup: UUID → NodeIndex → Signatory
    pub fn get_signatory(&self, id: &str) -> Option<Signatory> {
        let state = self.state.read();
        let node_idx = state.uuid_to_node.get(id)?;
        state.node_data.get(node_idx).cloned()
    }

    /// O(k) contract lookup where k = out-degree of principal: use graph neighbors
    pub fn get_obligations(&self, principal_id: &str) -> Vec<Contract> {
        let state = self.state.read();
        let principal_node = match state.uuid_to_node.get(principal_id) {
            Some(n) => *n,
            None => return vec![],
        };

        let neighbors: Vec<NodeIndex> = state.graph.neighbors(principal_node).collect();

        neighbors
            .iter()
            .filter_map(|&neighbor_node| {
                state
                    .edge_metadata
                    .get(&(principal_node, neighbor_node))
                    .cloned()
            })
            .collect()
    }

    /// O(k) contract lookup where k = in-degree of guarantor: reverse neighbors
    pub fn get_guarantees(&self, guarantor_id: &str) -> Vec<Contract> {
        let state = self.state.read();
        let guarantor_node = match state.uuid_to_node.get(guarantor_id) {
            Some(n) => *n,
            None => return vec![],
        };

        let predecessors: Vec<NodeIndex> = state
            .graph
            .neighbors_directed(guarantor_node, Incoming)
            .collect();

        predecessors
            .iter()
            .filter_map(|&pred_node| {
                state
                    .edge_metadata
                    .get(&(pred_node, guarantor_node))
                    .cloned()
            })
            .collect()
    }

    /// Trace chain of obligation using BFS with O(1) neighbor lookups
    pub fn trace_chain_of_obligation(
        &self,
        start_signatory_id: &str,
        max_depth: usize,
    ) -> Option<ChainOfObligation> {
        let start_signatory = self.get_signatory(start_signatory_id)?;

        let state = self.state.read();
        let start_node = *state.uuid_to_node.get(start_signatory_id)?;

        let mut chain = vec![(start_signatory.clone(), None)];
        let mut visited = std::collections::HashSet::new();
        visited.insert(start_node);

        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start_node, 0usize));

        while let Some((current_node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // O(1) neighbor iteration: petgraph's neighbor_indices is linear in out-degree only
            for neighbor in state.graph.neighbors(current_node) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);

                    if let Some(signatory) = state.node_data.get(&neighbor) {
                        if let Some(contract) = state.edge_metadata.get(&(current_node, neighbor)) {
                            chain.push((signatory.clone(), Some(contract.clone())));
                            queue.push_back((neighbor, depth + 1));
                        }
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

    /// Audit for contract violations: signatories without audit coverage (O(n) scan over node_data)
    pub fn audit_contract_coverage(&self) -> ContractAuditReport {
        let mut audited_signatories = std::collections::HashSet::new();
        let state = self.state.read();

        // Find all signatories involved in audit contracts
        for ((_, _), contract) in state.edge_metadata.iter() {
            if contract.clause_type == ClauseType::Audits {
                audited_signatories.insert(contract.principal_id.clone());
                audited_signatories.insert(contract.guarantor_id.clone());
            }
        }

        // Find unaudited code signatories
        let unaudited: Vec<Signatory> = state
            .node_data
            .values()
            .filter(|signatory| {
                matches!(
                    signatory.signatory_type,
                    SignatoryType::Function | SignatoryType::Class
                ) && !audited_signatories.contains(&signatory.id)
            })
            .cloned()
            .collect();

        let total_signatories = state.node_data.len();
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

        let state = self.state.read();

        // Index signatories by type and label
        for signatory in state.node_data.values() {
            by_label.insert(signatory.label.clone(), signatory.id.clone());
            let type_name = format!("{:?}", signatory.signatory_type);
            by_type
                .entry(type_name)
                .or_default()
                .push(signatory.id.clone());
        }

        // Calculate in-degree (obligation count)
        for (_, contract) in state.edge_metadata.iter() {
            *in_degree.entry(contract.guarantor_id.clone()).or_insert(0) += 1;
        }

        let mut most_obligated: Vec<(String, usize)> = in_degree.into_iter().collect();
        most_obligated.sort_by(|a, b| b.1.cmp(&a.1));
        most_obligated.truncate(20);

        AIContractBrief {
            entity: entity.to_string(),
            generated_at: chrono::Utc::now(),
            signatory_count: state.node_data.len(),
            contract_count: state.edge_metadata.len(),
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
        let state = self.state.read();
        (state.node_data.len(), state.edge_metadata.len())
    }

    /// Get all signatories in the ledger
    pub fn get_all_signatories(&self) -> Vec<Signatory> {
        let state = self.state.read();
        state.node_data.values().cloned().collect()
    }

    /// Get all contracts in the ledger
    pub fn get_all_contracts(&self) -> Vec<Contract> {
        let state = self.state.read();
        state.edge_metadata.values().cloned().collect()
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
    fn draft_contract_keeps_graph_and_metadata_in_sync() {
        let ledger = ContractLedger::new();
        let principal = Signatory::new(
            SignatoryType::Function,
            "repo/principal.rs".to_string(),
            "principal".to_string(),
            "fn principal() {}".to_string(),
        );
        let guarantor = Signatory::new(
            SignatoryType::Function,
            "repo/guarantor.rs".to_string(),
            "guarantor".to_string(),
            "fn guarantor() {}".to_string(),
        );

        let principal_id = ledger.register_signatory(principal).unwrap();
        let guarantor_id = ledger.register_signatory(guarantor).unwrap();
        let contract = Contract::new(
            principal_id.clone(),
            guarantor_id.clone(),
            ClauseType::Uses,
            0.8,
            ContractSource::Deterministic,
        );

        let contract_id = ledger.draft_contract(contract).unwrap();
        let obligations = ledger.get_obligations(&principal_id);

        assert_eq!(contract_id, obligations[0].id);
        assert_eq!(obligations[0].principal_id, principal_id);
        assert_eq!(obligations[0].guarantor_id, guarantor_id);
    }

    #[test]
    fn test_draft_contract() {
        let ledger = ContractLedger::new();

        let s1 = Signatory::new(
            SignatoryType::Function,
            "uri1".to_string(),
            "func1".to_string(),
            "".to_string(),
        );
        let s1_id = s1.id.clone();

        let s2 = Signatory::new(
            SignatoryType::Function,
            "uri2".to_string(),
            "func2".to_string(),
            "".to_string(),
        );
        let s2_id = s2.id.clone();

        ledger.register_signatory(s1).unwrap();
        ledger.register_signatory(s2).unwrap();

        let contract = Contract::new(
            s1_id,
            s2_id,
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

        let contract = Contract::new(
            code_id,
            test_id,
            ClauseType::Audits,
            1.0,
            ContractSource::Deterministic,
        );
        ledger.draft_contract(contract).unwrap();

        let report = ledger.audit_contract_coverage();
        assert_eq!(report.unaudited.len(), 0);
    }
}
