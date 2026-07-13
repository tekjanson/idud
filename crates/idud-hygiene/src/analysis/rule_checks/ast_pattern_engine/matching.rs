use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::visit::Visit;

use super::super::super::helpers::strip_test_modules;
use super::super::super::paths::relative_path;
use super::collector::{SemanticToken, SemanticTokenCollector};
use super::pattern_parsing::{
    normalize_pattern, parse_call_pattern, parse_method_pattern, split_pattern_alternatives,
};
use super::token_matching::{
    match_identifiers, matches_attribute_pattern, matches_call_pattern, matches_path_pattern,
    matches_return_type_pattern,
};
use crate::manifest::PatternSpec;

pub fn check_forbid_pattern_ast(
    repo_root: &Path,
    id: &str,
    pattern: &PatternSpec,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for pattern_text in pattern.as_vec() {
        for file in files {
            let content = fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let sanitized = strip_test_modules(&content);
            let parsed = syn::parse_file(&sanitized)
                .with_context(|| format!("failed to parse {} as Rust", file.display()))?;
            if pattern_matches(&parsed, &sanitized, &pattern_text) {
                violations.push(format!(
                    "[{id}] {pattern_text} matched {}",
                    relative_path(repo_root, file)
                ));
            }
        }
    }
    Ok(violations)
}

pub fn check_require_pattern_ast(
    repo_root: &Path,
    id: &str,
    pattern: &PatternSpec,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for pattern_text in pattern.as_vec() {
        for file in files {
            let content = fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let parsed = syn::parse_file(&content)
                .with_context(|| format!("failed to parse {} as Rust", file.display()))?;
            if !pattern_matches(&parsed, &content, &pattern_text) {
                violations.push(format!(
                    "[{id}] missing pattern `{pattern_text}` in {}",
                    relative_path(repo_root, file)
                ));
            }
        }
    }
    Ok(violations)
}

fn pattern_matches(file: &syn::File, content: &str, pattern_text: &str) -> bool {
    let mut collector = SemanticTokenCollector::default();
    collector.visit_file(file);
    let normalized = normalize_pattern(pattern_text);
    let alternatives = split_pattern_alternatives(&normalized);

    alternatives
        .iter()
        .any(|alternative| pattern_matches_single(&collector, alternative))
        || alternatives
            .iter()
            .any(|alternative| literal_pattern_matches(content, alternative))
}

fn literal_pattern_matches(content: &str, pattern: &str) -> bool {
    let normalized = normalize_pattern(pattern);
    let candidates = split_pattern_alternatives(&normalized)
        .into_iter()
        .map(|candidate| candidate.trim())
        .filter(|candidate| !candidate.is_empty())
        .collect::<Vec<_>>();

    candidates.iter().any(|candidate| {
        let search_text = candidate
            .trim_start_matches("->")
            .trim()
            .trim_end_matches('<')
            .trim_end_matches(')')
            .trim_end_matches('(')
            .trim();
        content.contains(search_text)
    })
}

fn pattern_matches_single(collector: &SemanticTokenCollector, pattern: &str) -> bool {
    if let Some((receiver, candidates)) = parse_method_pattern(pattern) {
        return collector.tokens.iter().any(|token| match token {
            SemanticToken::MethodCall {
                receiver: actual,
                method,
            } => {
                actual.as_deref() == Some(receiver)
                    && candidates.iter().any(|candidate| *candidate == method)
            }
            _ => false,
        });
    }

    if let Some((path, args)) = parse_call_pattern(pattern) {
        return collector
            .tokens
            .iter()
            .any(|token| matches_call_pattern(token, path, &args));
    }

    if pattern.starts_with("->") {
        return collector
            .tokens
            .iter()
            .any(|token| matches_return_type_pattern(token, pattern));
    }

    if pattern.contains("::") {
        return collector
            .tokens
            .iter()
            .any(|token| matches_path_pattern(token, pattern));
    }

    if pattern.starts_with("#[") || pattern.starts_with("#![") {
        return collector
            .tokens
            .iter()
            .any(|token| matches_attribute_pattern(token, pattern));
    }

    collector
        .tokens
        .iter()
        .any(|token| match_identifiers(token, pattern))
}
