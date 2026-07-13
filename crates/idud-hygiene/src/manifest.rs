use anyhow::{Context, Result};
use std::{fs, path::Path};

use crate::embedded_manifests::{embedded_manifest_content, embedded_manifest_paths};

pub use crate::manifest_schema::{
    CallGraphEdge, CallGraphNode, GoldenPattern, IncludeSpec, PatternSpec, Rule, RuleDocumentation,
};

pub fn load_golden_pattern(path: impl AsRef<Path>) -> Result<GoldenPattern> {
    let path = path.as_ref();
    let content = if path.exists() {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read golden pattern {}", path.display()))?
    } else if let Some(embedded) = embedded_manifest_content(path) {
        embedded.to_string()
    } else {
        return Err(anyhow::anyhow!(
            "failed to resolve golden pattern {}",
            path.display()
        ));
    };
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse golden pattern {}", path.display()))
}

pub fn embedded_manifest_paths_for_loading() -> Vec<std::path::PathBuf> {
    embedded_manifest_paths()
        .into_iter()
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()) != Some("pattern_registry.json")
        })
        .collect()
}
