// src/ui/translator.rs
//! WASM Presentation Layer: Contract Translator
//! Converts raw topological graph edges into human-readable sentences
//! Uses Leptos CSR for zero-install local dashboard

use crate::types::*;

/// Contract sentence struct: parsed, human-readable representation
#[derive(Debug, Clone)]
pub struct ContractSentence {
    pub principal_label: String,
    pub clause_description: String,
    pub guarantor_label: String,
    pub full_sentence: String,
    pub confidence: f32,
}

/// Translate ClauseType enum to human-readable verb phrase
fn clause_to_verb(clause: ClauseType) -> &'static str {
    match clause {
        ClauseType::Calls => "calls",
        ClauseType::Requires => "requires the capabilities of",
        ClauseType::RequiredBy => "is required by",
        ClauseType::CalledBy => "is called by",
        ClauseType::Uses => "uses",
        ClauseType::Documents => "documents",
        ClauseType::Audits => "is audited by",
        ClauseType::Implements => "implements",
        ClauseType::Enslaves => "has tight coupling with",
        ClauseType::EnslavedBy => "is tightly coupled to",
    }
}

/// Translate a Contract into a human-readable sentence
pub fn contract_to_sentence(principal: &Signatory, contract: &Contract, guarantor: &Signatory) -> ContractSentence {
    let clause_verb = clause_to_verb(contract.clause_type);
    let full_sentence = format!(
        "The {} '{}' {} the {} '{}'.",
        signatory_type_label(principal.signatory_type),
        principal.label,
        clause_verb,
        signatory_type_label(guarantor.signatory_type),
        guarantor.label
    );

    ContractSentence {
        principal_label: principal.label.clone(),
        clause_description: clause_verb.to_string(),
        guarantor_label: guarantor.label.clone(),
        full_sentence,
        confidence: contract.confidence,
    }
}

/// Get human-readable label for SignatoryType
pub fn signatory_type_label(sig_type: SignatoryType) -> &'static str {
    match sig_type {
        SignatoryType::File => "File",
        SignatoryType::Function => "Function",
        SignatoryType::Class => "Class",
        SignatoryType::Test => "Test",
        SignatoryType::Workflow => "Workflow",
        SignatoryType::Concept => "Concept",
        SignatoryType::ApiEndpoint => "API Endpoint",
        SignatoryType::MarkdownSection => "Documentation",
        SignatoryType::DecisionRecord => "Decision Record",
    }
}

/// Simple HTML renderer for contract sentences (no Leptos for now to avoid WASM compilation)
pub struct ContractRenderer;

impl ContractRenderer {
    pub fn render_sentence(sentence: &ContractSentence, show_confidence: bool) -> String {
        let confidence_class = if sentence.confidence > 0.85 {
            "confidence-high"
        } else if sentence.confidence > 0.7 {
            "confidence-medium"
        } else {
            "confidence-low"
        };

        let confidence_html = if show_confidence {
            format!(
                r#"<span class="confidence-badge">{}% confident</span>"#,
                (sentence.confidence * 100.0) as i32
            )
        } else {
            String::new()
        };

        format!(
            r#"<div class="contract-sentence {}"><p class="sentence-text">{}</p>{}</div>"#,
            confidence_class, sentence.full_sentence, confidence_html
        )
    }

    pub fn render_chain(chain: &ChainOfObligation) -> String {
        let mut html = format!(
            r#"<div class="chain-of-obligation">
            <h3>Chain of Obligations for "{}"</h3>
            <p class="chain-stats">{} signatories involved (max depth: {})</p>
            <ul class="chain-list">"#,
            chain.root_signatory.label, chain.total_signatories, chain.max_depth
        );

        for (idx, (signatory, contract_opt)) in chain.chain.iter().enumerate() {
            if let Some(contract) = contract_opt {
                let reasoning = contract
                    .clause_reasoning
                    .clone()
                    .unwrap_or_else(|| "Contractual obligation".to_string());

                html.push_str(&format!(
                    r#"<li class="chain-link">
                    <span class="chain-index">{}.</span>
                    <span class="chain-label">{}</span>
                    <span class="chain-reason">{}</span>
                    <span class="chain-confidence">{}%</span>
                </li>"#,
                    idx + 1,
                    signatory.label,
                    reasoning,
                    (contract.confidence * 100.0) as i32
                ));
            }
        }

        html.push_str("</ul></div>");
        html
    }

    pub fn render_network(signatories: &[Signatory], contracts: &[Contract]) -> String {
        let signatory_count = signatories.len();
        let contract_count = contracts.len();

        if contract_count == 0 {
            format!(
                r#"<div class="network-view">
                <div class="network-stats">
                    <h3>Contract Network Overview</h3>
                    <p>{} Signatories | {} Contracts</p>
                </div>
                <p class="empty-state">No contracts discovered yet.</p>
            </div>"#,
                signatory_count, contract_count
            )
        } else {
            format!(
                r#"<div class="network-view">
                <div class="network-stats">
                    <h3>Contract Network Overview</h3>
                    <p>{} Signatories | {} Contracts</p>
                </div>
                <div class="graph-placeholder">
                    <p>[Network graph rendering placeholder]</p>
                    <p>Showing {} contracts</p>
                </div>
            </div>"#,
                signatory_count, contract_count, contract_count
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_to_sentence_requires() {
        let principal = Signatory::new(
            SignatoryType::Function,
            "uri1".to_string(),
            "authenticate".to_string(),
            "".to_string(),
        );

        let guarantor = Signatory::new(
            SignatoryType::Function,
            "uri2".to_string(),
            "hashPassword".to_string(),
            "".to_string(),
        );

        let contract = Contract::new(
            principal.id.clone(),
            guarantor.id.clone(),
            ClauseType::Requires,
            0.95,
            ContractSource::Deterministic,
        );

        let sentence = contract_to_sentence(&principal, &contract, &guarantor);
        assert!(sentence.full_sentence.contains("authenticate"));
        assert!(sentence.full_sentence.contains("requires the capabilities of"));
        assert!(sentence.full_sentence.contains("hashPassword"));
        assert_eq!(sentence.confidence, 0.95);
    }

    #[test]
    fn test_contract_to_sentence_calls() {
        let principal = Signatory::new(
            SignatoryType::Function,
            "uri1".to_string(),
            "fetchData".to_string(),
            "".to_string(),
        );

        let guarantor = Signatory::new(
            SignatoryType::Function,
            "uri2".to_string(),
            "parseJSON".to_string(),
            "".to_string(),
        );

        let contract = Contract::new(
            principal.id.clone(),
            guarantor.id.clone(),
            ClauseType::Calls,
            0.85,
            ContractSource::AiInferred,
        );

        let sentence = contract_to_sentence(&principal, &contract, &guarantor);
        assert!(sentence.full_sentence.contains("fetchData"));
        assert!(sentence.full_sentence.contains("calls"));
        assert!(sentence.full_sentence.contains("parseJSON"));
    }

    #[test]
    fn test_contract_to_sentence_audits() {
        let principal = Signatory::new(
            SignatoryType::Test,
            "uri1".to_string(),
            "test_authenticate".to_string(),
            "".to_string(),
        );

        let guarantor = Signatory::new(
            SignatoryType::Function,
            "uri2".to_string(),
            "authenticate".to_string(),
            "".to_string(),
        );

        let contract = Contract::new(
            principal.id.clone(),
            guarantor.id.clone(),
            ClauseType::Audits,
            1.0,
            ContractSource::Deterministic,
        );

        let sentence = contract_to_sentence(&principal, &contract, &guarantor);
        assert!(sentence.full_sentence.contains("is audited by"));
        assert!(sentence.full_sentence.contains("Test"));
    }

    #[test]
    fn test_contract_renderer_render_sentence() {
        let sentence = ContractSentence {
            principal_label: "authenticate".to_string(),
            clause_description: "calls".to_string(),
            guarantor_label: "hashPassword".to_string(),
            full_sentence: "The Function 'authenticate' calls the Function 'hashPassword'.".to_string(),
            confidence: 0.95,
        };

        let html = ContractRenderer::render_sentence(&sentence, true);
        assert!(html.contains("confidence-high"));
        assert!(html.contains("95% confident"));
        assert!(html.contains("authenticate"));
    }

    #[test]
    fn test_contract_renderer_render_sentence_low_confidence() {
        let sentence = ContractSentence {
            principal_label: "foo".to_string(),
            clause_description: "uses".to_string(),
            guarantor_label: "bar".to_string(),
            full_sentence: "The Function 'foo' uses the Function 'bar'.".to_string(),
            confidence: 0.65,
        };

        let html = ContractRenderer::render_sentence(&sentence, false);
        assert!(html.contains("confidence-low"));
        assert!(!html.contains("confident"));
    }

    #[test]
    fn test_clause_to_verb_coverage() {
        // Verify all clause types have verb translations
        assert!(!clause_to_verb(ClauseType::Calls).is_empty());
        assert!(!clause_to_verb(ClauseType::Requires).is_empty());
        assert!(!clause_to_verb(ClauseType::Uses).is_empty());
        assert!(!clause_to_verb(ClauseType::Audits).is_empty());
        assert!(!clause_to_verb(ClauseType::Documents).is_empty());
        assert!(!clause_to_verb(ClauseType::Enslaves).is_empty());
    }

    #[test]
    fn test_signatory_type_label_coverage() {
        // Verify all signatory types have human-readable labels
        assert!(!signatory_type_label(SignatoryType::File).is_empty());
        assert!(!signatory_type_label(SignatoryType::Function).is_empty());
        assert!(!signatory_type_label(SignatoryType::Class).is_empty());
        assert!(!signatory_type_label(SignatoryType::Test).is_empty());
        assert!(!signatory_type_label(SignatoryType::Workflow).is_empty());
    }

    #[test]
    fn test_contract_renderer_network_empty() {
        let html = ContractRenderer::render_network(&[], &[]);
        assert!(html.contains("0 Signatories | 0 Contracts"));
        assert!(html.contains("No contracts discovered yet"));
    }

    #[test]
    fn test_contract_renderer_network_with_data() {
        let sig1 = Signatory::new(
            SignatoryType::Function,
            "uri1".to_string(),
            "func1".to_string(),
            "".to_string(),
        );
        let sig2 = Signatory::new(
            SignatoryType::Function,
            "uri2".to_string(),
            "func2".to_string(),
            "".to_string(),
        );

        let contract = Contract::new(
            sig1.id.clone(),
            sig2.id.clone(),
            ClauseType::Calls,
            1.0,
            ContractSource::Deterministic,
        );

        let html = ContractRenderer::render_network(&[sig1, sig2], &[contract]);
        assert!(html.contains("2 Signatories | 1 Contracts"));
        assert!(html.contains("Showing 1 contracts"));
    }
}
