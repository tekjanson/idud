use anyhow::{Context, Result};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::helpers::{collect_named_entities, strip_test_modules};
use super::super::paths::relative_path;

pub fn check_require_dependency(
    repo_root: &Path,
    id: &str,
    pattern: &str,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let sanitized = strip_test_modules(&content);
        let regex = Regex::new(&regex::escape(pattern))
            .with_context(|| format!("invalid dependency pattern for rule {id}: {pattern}"))?;
        if !regex.is_match(&sanitized) {
            violations.push(format!(
                "[{id}] missing dependency `{pattern}` in {}",
                relative_path(repo_root, file)
            ));
        }
    }
    Ok(violations)
}

pub fn check_forbid_dependency(
    repo_root: &Path,
    id: &str,
    pattern: &str,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let sanitized = strip_test_modules(&content);
        let regex = Regex::new(&regex::escape(pattern))
            .with_context(|| format!("invalid dependency pattern for rule {id}: {pattern}"))?;
        if regex.is_match(&sanitized) {
            violations.push(format!(
                "[{id}] forbidden dependency `{pattern}` matched {}",
                relative_path(repo_root, file)
            ));
        }
    }
    Ok(violations)
}

pub fn check_require_naming(
    repo_root: &Path,
    id: &str,
    target: &str,
    pattern: &str,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    let regex = Regex::new(pattern)
        .with_context(|| format!("invalid naming pattern for rule {id}: {pattern}"))?;
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let sanitized = strip_test_modules(&content);
        let names = collect_named_entities(&sanitized, target);
        if names.is_empty() {
            continue;
        }
        for name in names {
            if !regex.is_match(&name) {
                violations.push(format!(
                    "[{id}] {target} `{name}` in {} did not match naming pattern `{pattern}`",
                    relative_path(repo_root, file)
                ));
            }
        }
    }
    Ok(violations)
}
