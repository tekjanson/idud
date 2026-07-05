//! Training module: discovering and analyzing repositories for training validation.

pub mod discovery;
pub mod predictor;

pub use discovery::{
    discover_training_repos, fetch_issue_and_linked_pr, RepoCandidate, IssueWithPR,
    RateLimitStatus,
};
pub use predictor::{predict_files_from_issue, PredictionRequest, PredictionResponse, TokenUsage};
