// src/pipelines/mod.rs
pub mod broad_sweep;
pub mod deep_link;
pub mod embedding;

pub use broad_sweep::{RepositoryIngestionConfig, RepositoryTraverser, IngestionResult};
pub use deep_link::{
    BatchDiscoveryEngine, ContractDiscoveryEngine, InferenceClient, LLMDiscoveryRequest, 
    OllamaDispatcher, OllamaRequest, OllamaResponse, InferredContract, LLMDiscoveryResponse,
};
