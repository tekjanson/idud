//! Dependency analysis module
//!
//! Provides both deterministic AST-based analysis and AI-augmented dependency linking.
//! AST analysis uses regex to extract imports, calls, and type references.
//! AI analysis infers semantic dependencies that AST analysis misses.

pub mod ai_linker;
pub mod ast_analyzer;
pub mod contract_merger;
pub mod extractors;

pub use ai_linker::{AILinker, AILinkerConfig, AILinkerMetrics};
pub use ast_analyzer::{ASTAnalyzer, Dependency, DependencyAnalyzer};
pub use contract_merger::ContractMerger;
