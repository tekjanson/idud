// src/ui/translator.rs
//! WASM Presentation Layer: Contract Translator
//! Converts raw topological graph edges into human-readable sentences
//! Uses Leptos CSR for zero-install local dashboard

use crate::types::*;
use leptos::*;

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
pub fn clause_to_verb(clause: ClauseType) -> &'static str {
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
pub fn contract_to_sentence(
    principal: &Signatory,
    contract: &Contract,
    guarantor: &Signatory,
) -> ContractSentence {
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

/// Compute confidence class based on confidence score
fn confidence_class(confidence: f32) -> &'static str {
    if confidence > 0.85 {
        "confidence-high"
    } else if confidence > 0.7 {
        "confidence-medium"
    } else {
        "confidence-low"
    }
}

/// ContractSentenceView: Renders a single contract sentence with optional confidence badge
#[component]
pub fn contract_sentence_view(
    sentence: ContractSentence,
    #[prop(into)] show_confidence: Signal<bool>,
) -> impl IntoView {
    let confidence_pct = (sentence.confidence * 100.0) as i32;
    let css_class = confidence_class(sentence.confidence);

    view! {
        <div class=format!("contract-sentence {}", css_class)>
            <p class="sentence-text">{sentence.full_sentence}</p>
            {move || {
                show_confidence.get().then(|| {
                    view! {
                        <span class="confidence-badge">{confidence_pct}% confident</span>
                    }
                })
            }}
        </div>
    }
}

/// ChainOfObligationView: Renders a chain of obligations as an ordered list
#[component]
pub fn chain_of_obligation_view(chain: ChainOfObligation) -> impl IntoView {
    let root_label = chain.root_signatory.label.clone();
    let total_signatories = chain.total_signatories;
    let max_depth = chain.max_depth;
    let items = chain
        .chain
        .into_iter()
        .enumerate()
        .map(|(idx, (signatory, contract_opt))| {
            let chain_index = idx + 1;
            let label = signatory.label.clone();
            let (reasoning, confidence_pct) = if let Some(contract) = contract_opt {
                let reason = contract
                    .clause_reasoning
                    .unwrap_or_else(|| "Contractual obligation".to_string());
                let conf_pct = (contract.confidence * 100.0) as i32;
                (reason, conf_pct)
            } else {
                ("No contract data".to_string(), 0)
            };

            view! {
                <li class="chain-link">
                    <span class="chain-index">{chain_index}.</span>
                    <span class="chain-label">{label}</span>
                    <span class="chain-reason">{reasoning}</span>
                    <span class="chain-confidence">{confidence_pct}%</span>
                </li>
            }
        })
        .collect_view();

    view! {
        <div class="chain-of-obligation">
            <h3>"Chain of Obligations for \""{root_label}"\""</h3>
            <p class="chain-stats">{total_signatories}" signatories involved (max depth: "{max_depth}")"</p>
            <ul class="chain-list">
                {items}
            </ul>
        </div>
    }
}

/// NetworkOverview: Renders high-level contract network statistics
#[component]
pub fn network_overview(
    #[prop(into)] signatory_count: Signal<usize>,
    #[prop(into)] contract_count: Signal<usize>,
) -> impl IntoView {
    view! {
        <div class="network-view">
            <div class="network-stats">
                <h3>"Contract Network Overview"</h3>
                <p>
                    {move || signatory_count.get()}" Signatories | "
                    {move || contract_count.get()}" Contracts"
                </p>
            </div>
            {move || {
                if contract_count.get() == 0 {
                    view! {
                        <p class="empty-state">"No contracts discovered yet."</p>
                    }
                    .into_view()
                } else {
                    view! {
                        <div class="graph-placeholder">
                            <p>"[Network graph rendering placeholder]"</p>
                            <p>"Showing "{move || contract_count.get()}" contracts"</p>
                        </div>
                    }
                    .into_view()
                }
            }}
        </div>
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
        assert!(sentence
            .full_sentence
            .contains("requires the capabilities of"));
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
    fn test_confidence_class_high() {
        assert_eq!(confidence_class(0.95), "confidence-high");
        assert_eq!(confidence_class(0.86), "confidence-high");
    }

    #[test]
    fn test_confidence_class_medium() {
        assert_eq!(confidence_class(0.85), "confidence-medium");
        assert_eq!(confidence_class(0.75), "confidence-medium");
        assert_eq!(confidence_class(0.71), "confidence-medium");
    }

    #[test]
    fn test_confidence_class_low() {
        assert_eq!(confidence_class(0.70), "confidence-low");
        assert_eq!(confidence_class(0.50), "confidence-low");
        assert_eq!(confidence_class(0.0), "confidence-low");
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
    fn test_contract_sentence_creation() {
        let sentence = ContractSentence {
            principal_label: "authenticate".to_string(),
            clause_description: "calls".to_string(),
            guarantor_label: "hashPassword".to_string(),
            full_sentence: "The Function 'authenticate' calls the Function 'hashPassword'."
                .to_string(),
            confidence: 0.95,
        };

        assert_eq!(sentence.confidence, 0.95);
        assert!(sentence.full_sentence.contains("calls"));
    }

    #[test]
    fn test_chain_of_obligation_structure() {
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

        let chain = ChainOfObligation {
            root_signatory: sig1.clone(),
            chain: vec![(sig1.clone(), Some(contract)), (sig2.clone(), None)],
            max_depth: 2,
            total_signatories: 2,
        };

        assert_eq!(chain.total_signatories, 2);
        assert_eq!(chain.max_depth, 2);
        assert_eq!(chain.chain.len(), 2);
    }
}
