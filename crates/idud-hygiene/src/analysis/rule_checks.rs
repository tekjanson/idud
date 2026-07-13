mod call_graph;
mod dependencies;
mod patterns;
mod sizes;

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::manifest::Rule;

pub fn collect_rule_violations(
    repo_root: &Path,
    rule: &Rule,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    match rule {
        Rule::ForbidPattern { id, pattern, .. } => violations.extend(
            patterns::check_forbid_pattern(repo_root, id, pattern, files)?,
        ),
        Rule::RequirePattern { id, pattern, .. } => violations.extend(
            patterns::check_require_pattern(repo_root, id, pattern, files)?,
        ),
        Rule::MaxFileLines { id, max_lines, .. } => violations.extend(sizes::check_max_file_lines(
            repo_root, id, max_lines, files,
        )?),
        Rule::MaxParameters {
            id, max_parameters, ..
        } => violations.extend(sizes::check_max_parameters(
            repo_root,
            id,
            max_parameters,
            files,
        )?),
        Rule::MaxNestingDepth { id, max_depth, .. } => violations.extend(
            sizes::check_max_nesting_depth(repo_root, id, max_depth, files)?,
        ),
        Rule::RequireDependency { id, pattern, .. } => violations.extend(
            dependencies::check_require_dependency(repo_root, id, pattern, files)?,
        ),
        Rule::ForbidDependency { id, pattern, .. } => violations.extend(
            dependencies::check_forbid_dependency(repo_root, id, pattern, files)?,
        ),
        Rule::RequireNaming {
            id,
            target,
            pattern,
            ..
        } => violations.extend(dependencies::check_require_naming(
            repo_root, id, target, pattern, files,
        )?),
        Rule::RequireCallGraph {
            id, nodes, edges, ..
        } => violations.extend(call_graph::check_require_call_graph(
            repo_root, id, nodes, edges, files,
        )?),
    }
    Ok(violations)
}
