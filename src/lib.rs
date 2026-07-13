// src/lib.rs
#![forbid(unsafe_code)]

pub mod analysis;
pub mod cli;
pub mod contract_ledger;
pub mod core;
pub mod pipelines;
pub mod schemas;
pub mod training;
pub mod training_datalake;
pub mod types;
pub mod ui;
pub mod web_server;

pub use analysis::{AILinker, AILinkerConfig, ASTAnalyzer, Dependency, DependencyAnalyzer};
pub use cli::{run as run_cli, Cli, CliCommand};
pub use contract_ledger::ContractLedger;
pub use core::{
    Database, GraphPointer, GraphPointerKind, SqliteDatabase, TopologyEdge, TopologyNode,
    TopologySnapshot, TreeSitterParser,
};
pub use pipelines::{RepositoryIngestionConfig, RepositoryTraverser};
pub use schemas::{ContractFactory, ContractValidator, SignatoryFactory};
pub use training::{
    batch_training_jobs, build_synthetic_understanding, calculate_aggregate_metrics,
    calculate_metrics_by_language, discover_training_repos, fetch_issue_and_linked_pr,
    predict_files_from_issue, validate_prediction, write_synthetic_understanding,
    write_training_result, CacheStats, DependencyHint, DirectorySummary, ExtensionSummary,
    IngestionLogEntry, IngestionMetrics, IngestionResults, IssueWithPR, JourneyCandidate,
    LanguageMetrics, PredictionRequest, PredictionResponse, RepoCandidate, RepoIngestionConfig,
    RepositoryEntry, RepositoryIngestionOrchestrator, RepositoryRegistry, SyntheticUnderstanding,
    TestSummary, TrainingCache, TrainingConfig, TrainingOrchestrator, TrainingResults,
    TrainingStatus, ValidationMetrics,
};
pub use training_datalake::{
    AggregatedMetrics, Checkpoint, IngestionStatus, PercentileMetrics, RepoMetadata, TimeWindow,
    TrainingDataLake, TrainingRun,
};
pub use types::*;
pub use ui::translator::*;
pub use web_server::{serve, WebServerConfig};
