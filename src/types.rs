// src/types.rs
//! Contract Ledger types: an immutable ledger of software contracts and bindings.
//! Pure topological index: stores contractual pointers, not logic or knowledge.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignatoryType {
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
pub enum ClauseType {
    /// This signatory implements the contract
    Implements,
    /// This signatory is audited by another
    Audits,
    /// This signatory requires another to function
    Requires,
    /// This signatory is required by another
    RequiredBy,
    /// This signatory calls another
    Calls,
    /// This signatory is called by another
    CalledBy,
    /// This signatory documents another
    Documents,
    /// This signatory uses another
    Uses,
    /// This signatory enslaves another (high coupling)
    Enslaves,
    /// This signatory is enslaved by another
    EnslavedBy,
}

/// Signatory: a contractual entity in the ledger
/// Every component (file, function, test) that enters into contractual obligations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signatory {
    pub id: String,
    pub signatory_type: SignatoryType,
    /// Pirate Bay link: precise location in repo/branch/line
    pub source_uri: String,
    pub label: String,
    /// Raw snippet for contract discovery
    pub snippet: String,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Signatory {
    pub fn new(
        signatory_type: SignatoryType,
        source_uri: String,
        label: String,
        snippet: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            signatory_type,
            source_uri,
            label,
            snippet,
            registered_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Contract: a binding between two Signatories
/// Represents contractual obligations discovered during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    /// Principal: the signatory with obligations
    pub principal_id: String,
    /// Guarantor: the signatory that the principal binds to
    pub guarantor_id: String,
    pub clause_type: ClauseType,
    /// Confidence score: AI certainty about this binding (0-1)
    pub confidence: f32,
    pub discovered_by: ContractSource,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub clause_reasoning: Option<String>,
    pub evidential_proofs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractSource {
    /// Binding discovered through deterministic parsing
    Deterministic,
    /// Binding inferred through AI analysis
    AiInferred,
}

impl Contract {
    pub fn new(
        principal_id: String,
        guarantor_id: String,
        clause_type: ClauseType,
        confidence: f32,
        discovered_by: ContractSource,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            principal_id,
            guarantor_id,
            clause_type,
            confidence: confidence.max(0.0).min(1.0),
            discovered_by,
            discovered_at: chrono::Utc::now(),
            clause_reasoning: None,
            evidential_proofs: vec![],
        }
    }

    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.clause_reasoning = Some(reasoning);
        self
    }

    pub fn with_proof(mut self, proof: String) -> Self {
        self.evidential_proofs.push(proof);
        self
    }
}

/// Chain of Obligation: a path through contractual bindings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfObligation {
    pub root_signatory: Signatory,
    pub chain: Vec<(Signatory, Option<Contract>)>,
    pub max_depth: usize,
    pub total_signatories: usize,
}

/// Contract Audit Report: identifies gaps and violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAuditReport {
    pub audited_signatories: usize,
    pub unaudited: Vec<Signatory>,
    pub audit_coverage_percent: f32,
    pub violations: Vec<(Signatory, String)>,
}

/// AI Contract Brief: compressed, queryable ledger snapshot
/// Loaded into AI context to avoid token waste during traversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIContractBrief {
    pub entity: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub signatory_count: usize,
    pub contract_count: usize,
    pub conceptual_contracts: Vec<ContractualConcept>,
    pub workflow_bindings: Vec<WorkflowBinding>,
    pub ledger_index: LedgerIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractualConcept {
    pub id: String,
    pub name: String,
    pub principal_obligations: Vec<String>,
    pub guarantor_obligations: Vec<String>,
    pub audited_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBinding {
    pub name: String,
    pub signatories: Vec<String>,
    pub critical_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerIndex {
    pub by_type: HashMap<String, Vec<String>>,
    pub by_label: HashMap<String, String>,
    pub most_obligated: Vec<(String, usize)>,
}
