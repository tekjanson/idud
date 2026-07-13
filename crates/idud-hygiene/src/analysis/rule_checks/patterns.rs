use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::manifest::PatternSpec;

use super::ast_patterns::{check_forbid_pattern_ast, check_require_pattern_ast};

pub fn check_forbid_pattern(
    repo_root: &Path,
    id: &str,
    pattern: &PatternSpec,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    check_forbid_pattern_ast(repo_root, id, pattern, files)
}

pub fn check_require_pattern(
    repo_root: &Path,
    id: &str,
    pattern: &PatternSpec,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    check_require_pattern_ast(repo_root, id, pattern, files)
}
