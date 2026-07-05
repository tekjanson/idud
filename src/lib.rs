// src/lib.rs
pub mod contract_ledger;
pub mod pipelines;
pub mod schemas;
pub mod training_datalake;
pub mod types;
pub mod ui;
pub mod web_server;

pub use contract_ledger::ContractLedger;
pub use pipelines::{RepositoryIngestionConfig, RepositoryTraverser};
pub use schemas::{ContractFactory, ContractValidator, SignatoryFactory};
pub use training_datalake::{
    AggregatedMetrics, Checkpoint, IngestionStatus, PercentileMetrics, RepoMetadata,
    TimeWindow, TrainingDataLake, TrainingRun,
};
pub use types::*;
pub use ui::translator::*;
pub use web_server::{serve, WebServerConfig};
