use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use super::super::helpers::collect_functions;
use super::super::paths::relative_path;
use crate::manifest::{CallGraphEdge, CallGraphNode};

pub fn check_require_call_graph(
    repo_root: &Path,
    id: &str,
    nodes: &[CallGraphNode],
    edges: &[CallGraphEdge],
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    let node_ids: HashSet<&str> = nodes.iter().map(|node| node.id.as_str()).collect();
    for edge in edges {
        if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
            return Err(anyhow!(
                "call-graph rule {id} references unknown nodes {} -> {}",
                edge.from,
                edge.to
            ));
        }
    }
    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let functions = collect_functions(&content);
        let mut matched_nodes: HashMap<String, Vec<super::super::helpers::FunctionDefinition>> =
            HashMap::new();
        for node in nodes {
            let regex = Regex::new(&node.pattern)
                .with_context(|| format!("invalid regex for call-graph node {}", node.id))?;
            let matches: Vec<_> = functions
                .iter()
                .filter(|function| regex.is_match(&function.name))
                .cloned()
                .collect();
            if matches.is_empty() {
                violations.push(format!(
                    "[{id}] node `{}` had no matching functions in {}",
                    node.id,
                    relative_path(repo_root, file)
                ));
                continue;
            }
            matched_nodes.insert(node.id.clone(), matches);
        }
        for edge in edges {
            let Some(source_functions) = matched_nodes.get(&edge.from) else {
                continue;
            };
            let Some(target_functions) = matched_nodes.get(&edge.to) else {
                continue;
            };
            let mut edge_found = false;
            for source in source_functions {
                for target in target_functions {
                    if source.body_contains_call(&target.name) {
                        edge_found = true;
                        break;
                    }
                }
                if edge_found {
                    break;
                }
            }
            if !edge_found {
                violations.push(format!(
                    "[{id}] edge {} -> {} was not found in {}",
                    edge.from,
                    edge.to,
                    relative_path(repo_root, file)
                ));
            }
        }
    }
    Ok(violations)
}
