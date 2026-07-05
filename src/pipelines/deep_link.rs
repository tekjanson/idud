// src/pipelines/deep_link.rs
//! PHASE 3.2: The Deep Link
//! AI prompts for inferring contracts between signatories
//! Uses LLM to discover bindings missed by deterministic parsing

use crate::types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMDiscoveryRequest {
    pub signatory: Signatory,
    pub context_signatories: Vec<Signatory>,
    pub max_contracts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredContract {
    pub guarantor_id: String,
    pub clause_type: ClauseType,
    pub confidence: f32,
    pub clause_reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMDiscoveryResponse {
    pub principal_id: String,
    pub contracts: Vec<InferredContract>,
}

/// System prompt for contract discovery
fn system_prompt() -> String {
    r#"You are an expert code analyst tasked with discovering contractual bindings between code elements.

Given a principal signatory and contextual information about other signatories, identify likely clauses (binding obligations) between them.

Return ONLY valid JSON with this structure:
{
  "contracts": [
    {
      "guarantor_id": "signatory-xyz",
      "clause_type": "Calls|Requires|Uses|Enslaves",
      "confidence": 0.85,
      "clause_reasoning": "Why this binding exists"
    }
  ]
}

Clause Types:
- Calls: Principal directly invokes guarantor
- Requires: Principal depends on guarantor to function
- Uses: Principal utilizes guarantor capability
- Enslaves: Principal changes force guarantor changes (high coupling)

Guidelines:
- confidence: 0.0-1.0 (how certain the binding exists)
- Only return bindings with confidence > 0.7
- Each binding is a contractual obligation"#
        .to_string()
}

/// Build discovery prompt for a signatory
fn discovery_prompt(principal: &Signatory, context: &[Signatory]) -> String {
    let mut prompt = format!(
        "Discover contractual bindings for this principal signatory.\n\n\
         PRINCIPAL:\n\
         Type: {:?}\n\
         Name: {}\n\
         Location: {}\n\
         Code:\n{}\n\n\
         CONTEXT (other signatories in the repository):\n",
        principal.signatory_type, principal.label, principal.source_uri, principal.snippet
    );

    for (i, signatory) in context.iter().enumerate() {
        prompt.push_str(&format!(
            "\n[{}] Type: {:?}, Name: {}\nCode:\n{}\n",
            signatory.id, signatory.signatory_type, signatory.label, 
            signatory.snippet.lines().take(3).collect::<Vec<_>>().join("\n")
        ));
        if i >= 10 {
            prompt.push_str("\n(... and more signatories in context)");
            break;
        }
    }

    prompt.push_str("\n\nIdentify contract clauses from PRINCIPAL to CONTEXT signatories. Return JSON only.");
    prompt
}

/// LLM interface for contract discovery
pub struct ContractDiscoveryEngine {
    enable_mock: bool,
}

impl ContractDiscoveryEngine {
    pub fn new(enable_mock: bool) -> Self {
        Self { enable_mock }
    }

    /// Discover contracts for a signatory using LLM
    pub async fn discover_contracts(
        &self,
        request: LLMDiscoveryRequest,
    ) -> Result<LLMDiscoveryResponse> {
        if self.enable_mock {
            return self.mock_discovery(request).await;
        }

        // TODO: Call actual LLM API
        // 1. Build prompt using discovery_prompt()
        // 2. Call OpenAI/Ollama with system_prompt()
        // 3. Parse JSON response
        // 4. Validate confidence scores
        // 5. Return inferred contracts

        self.mock_discovery(request).await
    }

    /// Mock discovery for testing (deterministic for UAT)
    async fn mock_discovery(&self, request: LLMDiscoveryRequest) -> Result<LLMDiscoveryResponse> {
        let mut contracts = Vec::new();

        // Heuristic: functions often bind to functions with complementary names
        if request.signatory.signatory_type == SignatoryType::Function {
            for context in &request.context_signatories {
                if context.signatory_type == SignatoryType::Function {
                    let principal_name = request.signatory.label.to_lowercase();
                    let context_name = context.label.to_lowercase();

                    if (principal_name.contains("fetch") && context_name.contains("parse"))
                        || (principal_name.contains("validate") && context_name.contains("clean"))
                    {
                        contracts.push(InferredContract {
                            guarantor_id: context.id.clone(),
                            clause_type: ClauseType::Calls,
                            confidence: 0.75,
                            clause_reasoning: format!(
                                "Name pattern: '{}' calls '{}'",
                                request.signatory.label, context.label
                            ),
                        });
                    }
                }
            }
        }

        // Tests bind to the code they audit
        if request.signatory.signatory_type == SignatoryType::Test {
            for context in &request.context_signatories {
                if context.signatory_type == SignatoryType::Function {
                    let test_name = request.signatory.label.to_lowercase();
                    let func_name = context.label.to_lowercase();

                    if test_name.contains(&func_name) || func_name.contains(&test_name) {
                        contracts.push(InferredContract {
                            guarantor_id: context.id.clone(),
                            clause_type: ClauseType::Audits,
                            confidence: 0.95,
                            clause_reasoning: format!("Test '{}' audits '{}'", request.signatory.label, context.label),
                        });
                    }
                }
            }
        }

        Ok(LLMDiscoveryResponse {
            principal_id: request.signatory.id,
            contracts: contracts.into_iter().take(request.max_contracts).collect(),
        })
    }
}

/// Batch contract discovery engine
pub struct BatchDiscoveryEngine {
    engine: ContractDiscoveryEngine,
    batch_size: usize,
}

impl BatchDiscoveryEngine {
    pub fn new(engine: ContractDiscoveryEngine, batch_size: usize) -> Self {
        Self { engine, batch_size }
    }

    /// Discover contracts for signatories in batch
    pub async fn discover_batch(&self, signatories: Vec<Signatory>, context: Vec<Signatory>) -> Result<Vec<InferredContract>> {
        let mut all_contracts = Vec::new();

        for batch in signatories.chunks(self.batch_size) {
            let mut batch_futures = Vec::new();

            for signatory in batch {
                let engine = ContractDiscoveryEngine::new(true); // Use mock for now
                let request = LLMDiscoveryRequest {
                    signatory: signatory.clone(),
                    context_signatories: context.clone(),
                    max_contracts: 5,
                };

                batch_futures.push(tokio::spawn(async move {
                    engine.discover_contracts(request).await
                }));
            }

            // Await all requests in this batch
            for future in batch_futures {
                if let Ok(Ok(response)) = future.await {
                    all_contracts.extend(response.contracts);
                }
            }
        }

        Ok(all_contracts)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SignatoryType;

    #[tokio::test]
    async fn test_mock_discovery() {
        let engine = ContractDiscoveryEngine::new(true);
        let principal = Signatory::new(
            SignatoryType::Function,
            "uri".to_string(),
            "fetchData".to_string(),
            "fn fetchData(){}".to_string(),
        );

        let context = vec![
            Signatory::new(
                SignatoryType::Function,
                "uri".to_string(),
                "parseData".to_string(),
                "fn parseData(){}".to_string(),
            ),
        ];

        let request = LLMDiscoveryRequest {
            signatory: principal,
            context_signatories: context,
            max_contracts: 5,
        };

        let result = engine.discover_contracts(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.contracts.is_empty());
    }
}
