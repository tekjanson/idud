use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::helpers::{count_nesting_depth, count_parameter_violations, strip_test_modules};
use super::super::paths::relative_path;

pub fn check_max_file_lines(
    repo_root: &Path,
    id: &str,
    max_lines: &usize,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let line_count = content.lines().count();
        if line_count > *max_lines {
            violations.push(format!(
                "[{id}] {} has {line_count} lines (max {max_lines})",
                relative_path(repo_root, file)
            ));
        }
    }
    Ok(violations)
}

pub fn check_max_parameters(
    repo_root: &Path,
    id: &str,
    max_parameters: &usize,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let sanitized = strip_test_modules(&content);
        for violation in count_parameter_violations(&sanitized, *max_parameters) {
            violations.push(format!(
                "[{id}] {violation} in {}",
                relative_path(repo_root, file)
            ));
        }
    }
    Ok(violations)
}

pub fn check_max_nesting_depth(
    repo_root: &Path,
    id: &str,
    max_depth: &usize,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let sanitized = strip_test_modules(&content);
        let nesting_depth = count_nesting_depth(&sanitized);
        if nesting_depth > *max_depth {
            violations.push(format!(
                "[{id}] {} has nesting depth {nesting_depth} (max {max_depth})",
                relative_path(repo_root, file)
            ));
        }
    }
    Ok(violations)
}
