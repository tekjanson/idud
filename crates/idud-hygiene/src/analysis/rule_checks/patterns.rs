use anyhow::{Context, Result};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::helpers::strip_test_modules;
use super::super::paths::relative_path;
use crate::manifest::PatternSpec;

pub fn check_forbid_pattern(
    repo_root: &Path,
    id: &str,
    pattern: &PatternSpec,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for pattern_text in pattern.as_vec() {
        let regex = Regex::new(&pattern_text)
            .with_context(|| format!("invalid regex for rule {id}: {pattern_text}"))?;
        for file in files {
            let content = fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let sanitized = strip_test_modules(&content);
            if regex.is_match(&sanitized) {
                violations.push(format!(
                    "[{id}] {pattern_text} matched {}",
                    relative_path(repo_root, file)
                ));
            }
        }
    }
    Ok(violations)
}

pub fn check_require_pattern(
    repo_root: &Path,
    id: &str,
    pattern: &PatternSpec,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for pattern_text in pattern.as_vec() {
        let regex = Regex::new(&pattern_text)
            .with_context(|| format!("invalid regex for rule {id}: {pattern_text}"))?;
        for file in files {
            let content = fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            if !regex.is_match(&content) {
                violations.push(format!(
                    "[{id}] missing pattern `{pattern_text}` in {}",
                    relative_path(repo_root, file)
                ));
            }
        }
    }
    Ok(violations)
}
