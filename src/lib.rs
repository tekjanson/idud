// src/lib.rs
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

pub use analysis::{AILinker, AILinkerConfig, ASTAnalyzer, Dependency};
pub use cli::{run as run_cli, Cli, CliCommand};
pub use contract_ledger::ContractLedger;
pub use core::{Database, GraphPointer, GraphPointerKind, SqliteDatabase, TopologyEdge, TopologyNode, TopologySnapshot, TreeSitterParser};
pub use pipelines::{RepositoryIngestionConfig, RepositoryTraverser};
pub use schemas::{ContractFactory, ContractValidator, SignatoryFactory};
pub use training::{
    build_synthetic_understanding, discover_training_repos, fetch_issue_and_linked_pr, RepoCandidate, IssueWithPR,
    predict_files_from_issue, PredictionRequest, PredictionResponse, TrainingOrchestrator,
    TrainingConfig, TrainingResults, batch_training_jobs, TrainingStatus,
    validate_prediction, write_training_result, calculate_aggregate_metrics,
    calculate_metrics_by_language, ValidationMetrics, LanguageMetrics, TrainingCache, CacheStats,
    RepositoryIngestionOrchestrator, RepoIngestionConfig, RepositoryRegistry,
    RepositoryEntry, IngestionMetrics, IngestionResults, IngestionLogEntry,
    SyntheticUnderstanding, DependencyHint, DirectorySummary, ExtensionSummary,
    write_synthetic_understanding,
};
pub use training_datalake::{
    AggregatedMetrics, Checkpoint, IngestionStatus, PercentileMetrics, RepoMetadata,
    TimeWindow, TrainingDataLake, TrainingRun,
};
pub use types::*;
pub use ui::translator::*;
pub use web_server::{serve, WebServerConfig};
