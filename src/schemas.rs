// src/schemas.rs
//! Contract Ledger schema factories and validators

use crate::types::*;
use serde_json::json;

/// Signatory Factory: registers entities into the contract ledger
pub struct SignatoryFactory;

impl SignatoryFactory {
    pub fn register_file(
        repo_uri: &str,
        file_path: &str,
        branch: &str,
    ) -> Signatory {
        let source_uri = format!("{}/blob/{}/{}", repo_uri, branch, file_path);
        Signatory::new(
            SignatoryType::File,
            source_uri,
            file_path.to_string(),
            format!("File: {}", file_path),
        )
        .with_metadata("filePath".to_string(), json!(file_path))
        .with_metadata("repoUri".to_string(), json!(repo_uri))
        .with_metadata("branch".to_string(), json!(branch))
    }

    pub fn register_function(
        repo_uri: &str,
        file_path: &str,
        function_name: &str,
        snippet: String,
        line_start: usize,
        line_end: usize,
        branch: &str,
    ) -> Signatory {
        let source_uri = format!(
            "{}/blob/{}/{}#L{}-L{}",
            repo_uri, branch, file_path, line_start, line_end
        );
        Signatory::new(
            SignatoryType::Function,
            source_uri,
            function_name.to_string(),
            snippet,
        )
        .with_metadata("filePath".to_string(), json!(file_path))
        .with_metadata("functionName".to_string(), json!(function_name))
        .with_metadata("lineStart".to_string(), json!(line_start))
        .with_metadata("lineEnd".to_string(), json!(line_end))
    }

    pub fn register_test(
        repo_uri: &str,
        test_file_path: &str,
        test_name: &str,
        snippet: String,
        line_start: usize,
        line_end: usize,
        branch: &str,
    ) -> Signatory {
        let source_uri = format!(
            "{}/blob/{}/{}#L{}-L{}",
            repo_uri, branch, test_file_path, line_start, line_end
        );
        Signatory::new(
            SignatoryType::Test,
            source_uri,
            format!("{} ({})", test_name, test_file_path),
            snippet,
        )
        .with_metadata("testFile".to_string(), json!(test_file_path))
        .with_metadata("testName".to_string(), json!(test_name))
    }

    pub fn register_api_endpoint(
        repo_uri: &str,
        file_path: &str,
        method: &str,
        path: &str,
        snippet: String,
        line_start: usize,
        branch: &str,
    ) -> Signatory {
        let source_uri = format!(
            "{}/blob/{}/{}#L{}",
            repo_uri, branch, file_path, line_start
        );
        Signatory::new(
            SignatoryType::ApiEndpoint,
            source_uri,
            format!("{} {}", method, path),
            snippet,
        )
        .with_metadata("method".to_string(), json!(method))
        .with_metadata("path".to_string(), json!(path))
    }

    pub fn register_documentation(
        doc_uri: &str,
        section: &str,
        heading: &str,
        snippet: String,
    ) -> Signatory {
        let source_uri = format!("{}#{}", doc_uri, section);
        Signatory::new(
            SignatoryType::MarkdownSection,
            source_uri,
            heading.to_string(),
            snippet,
        )
        .with_metadata("docUri".to_string(), json!(doc_uri))
        .with_metadata("section".to_string(), json!(section))
    }

    pub fn register_concept(name: &str, description: String) -> Signatory {
        Signatory::new(
            SignatoryType::Concept,
            "synthetic://concept".to_string(),
            name.to_string(),
            description,
        )
        .with_metadata("name".to_string(), json!(name))
    }
}

/// Contract Factory: drafts binding clauses between signatories
pub struct ContractFactory;

impl ContractFactory {
    /// Principal requires guarantor to function
    pub fn requires_clause(
        principal_id: String,
        guarantor_id: String,
        confidence: f32,
        source: ContractSource,
        reasoning: Option<String>,
    ) -> Contract {
        let mut contract = Contract::new(
            principal_id,
            guarantor_id,
            ClauseType::Requires,
            confidence,
            source,
        );
        if let Some(r) = reasoning {
            contract = contract.with_reasoning(r);
        }
        contract
    }

    /// Principal audits guarantor
    pub fn audits_clause(principal_id: String, guarantor_id: String, confidence: f32) -> Contract {
        Contract::new(
            principal_id,
            guarantor_id,
            ClauseType::Audits,
            confidence,
            ContractSource::Deterministic,
        )
    }

    /// Principal calls guarantor
    pub fn calls_clause(
        principal_id: String,
        guarantor_id: String,
        confidence: f32,
        source: ContractSource,
    ) -> Contract {
        Contract::new(
            principal_id,
            guarantor_id,
            ClauseType::Calls,
            confidence,
            source,
        )
    }

    /// Principal enslaves guarantor (high coupling)
    pub fn enslaves_clause(
        principal_id: String,
        guarantor_id: String,
        confidence: f32,
    ) -> Contract {
        Contract::new(
            principal_id,
            guarantor_id,
            ClauseType::Enslaves,
            confidence,
            ContractSource::AiInferred,
        )
        .with_reasoning("Binding: changes to principal force changes to guarantor".to_string())
    }

    /// Principal documents guarantor
    pub fn documents_clause(principal_id: String, guarantor_id: String) -> Contract {
        Contract::new(
            principal_id,
            guarantor_id,
            ClauseType::Documents,
            1.0,
            ContractSource::Deterministic,
        )
    }

    /// Principal uses guarantor capability
    pub fn uses_clause(
        principal_id: String,
        guarantor_id: String,
        confidence: f32,
        source: ContractSource,
    ) -> Contract {
        Contract::new(
            principal_id,
            guarantor_id,
            ClauseType::Uses,
            confidence,
            source,
        )
    }
}

/// Contract validator: audit binding schema
pub struct ContractValidator;

impl ContractValidator {
    pub fn audit_signatory(signatory: &Signatory) -> Result<(), String> {
        if signatory.id.is_empty() {
            return Err("Signatory ID cannot be empty".to_string());
        }
        if signatory.source_uri.is_empty() {
            return Err("Signatory source_uri cannot be empty".to_string());
        }
        if signatory.label.is_empty() {
            return Err("Signatory label cannot be empty".to_string());
        }
        Ok(())
    }

    pub fn audit_contract(contract: &Contract) -> Result<(), String> {
        if contract.id.is_empty() {
            return Err("Contract ID cannot be empty".to_string());
        }
        if contract.principal_id.is_empty() {
            return Err("Contract principal_id cannot be empty".to_string());
        }
        if contract.guarantor_id.is_empty() {
            return Err("Contract guarantor_id cannot be empty".to_string());
        }
        if contract.confidence < 0.0 || contract.confidence > 1.0 {
            return Err("Contract confidence must be between 0 and 1".to_string());
        }
        Ok(())
    }
}
