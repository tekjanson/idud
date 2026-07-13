use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use super::paths::{relative_path, resolve_manifest_files, resolve_path};
use super::rules::collect_rule_violations;
use crate::manifest::load_golden_pattern;

#[derive(Debug, Serialize)]
pub struct RuleReport {
    pub id: String,
    pub passed: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ManifestReport {
    pub name: String,
    pub description: String,
    pub path: String,
    pub rules: Vec<RuleReport>,
    pub passed: bool,
    pub passed_rules: usize,
    pub failed_rules: usize,
}

pub fn enforce_golden_pattern(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<String>> {
    let repo_root = repo_root.as_ref();
    let manifest_path = resolve_path(repo_root, manifest_path.as_ref())?;
    let pattern = load_golden_pattern(&manifest_path)?;
    let mut violations = Vec::new();
    for rule in &pattern.rules {
        let files = super::paths::discover_files(repo_root, &rule.include());
        violations.extend(collect_rule_violations(repo_root, rule, &files)?);
    }
    Ok(violations)
}

pub fn report_golden_pattern(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<RuleReport>> {
    let repo_root = repo_root.as_ref();
    let manifest_path = resolve_path(repo_root, manifest_path.as_ref())?;
    let pattern = load_golden_pattern(&manifest_path)?;
    let mut reports = Vec::new();
    for rule in &pattern.rules {
        let files = super::paths::discover_files(repo_root, &rule.include());
        let violations = collect_rule_violations(repo_root, rule, &files)?;
        reports.push(RuleReport {
            id: rule.id().to_string(),
            passed: violations.is_empty(),
            violations,
        });
    }
    Ok(reports)
}

pub fn report_golden_manifests(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<ManifestReport>> {
    let repo_root = repo_root.as_ref();
    let manifest_files = resolve_manifest_files(repo_root, manifest_path.as_ref())?;
    let mut reports = Vec::new();
    for manifest_file in manifest_files {
        let pattern = load_golden_pattern(&manifest_file)?;
        let rule_reports = report_golden_pattern(repo_root, &manifest_file)?;
        let passed_rules = rule_reports.iter().filter(|report| report.passed).count();
        let failed_rules = rule_reports.len().saturating_sub(passed_rules);
        reports.push(ManifestReport {
            name: pattern.name,
            description: pattern.description,
            path: relative_path(repo_root, &manifest_file),
            passed: failed_rules == 0,
            passed_rules,
            failed_rules,
            rules: rule_reports,
        });
    }
    Ok(reports)
}

pub fn enforce_golden_manifests(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<String>> {
    let repo_root = repo_root.as_ref();
    let manifest_files = resolve_manifest_files(repo_root, manifest_path.as_ref())?;
    let mut violations = Vec::new();
    for manifest_file in manifest_files {
        let pattern = load_golden_pattern(&manifest_file)?;
        for rule in &pattern.rules {
            let files = super::paths::discover_files(repo_root, &rule.include());
            violations.extend(collect_rule_violations(repo_root, rule, &files)?);
        }
    }
    Ok(violations)
}
