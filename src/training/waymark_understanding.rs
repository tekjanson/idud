//! Synthetic understanding generation for repository training runs.
//!
//! This module turns deterministic repository signals (directory layout, file
//! categories, and import relationships) into a compact, AI-readable summary
//! that can be used as a training artifact for future indexing work.

use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A compact synthetic understanding artifact for a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticUnderstanding {
    pub repository: String,
    pub generated_at: String,
    pub summary: String,
    pub top_level_directories: Vec<DirectorySummary>,
    pub extensions: Vec<ExtensionSummary>,
    pub inferred_domains: Vec<String>,
    pub dependency_hints: Vec<DependencyHint>,
    pub notable_files: Vec<String>,
    pub synthetic_brief: String,
}

/// A summary of files found under a top-level directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorySummary {
    pub name: String,
    pub file_count: usize,
    pub extension_counts: Vec<ExtensionSummary>,
}

/// A count for a file extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSummary {
    pub extension: String,
    pub count: usize,
}

/// A lightweight import-edge hint between source files and modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHint {
    pub from: String,
    pub to: String,
    pub kind: String,
}

/// Build a synthetic understanding artifact for a local repository.
pub fn build_synthetic_understanding(repo_path: impl AsRef<Path>) -> Result<SyntheticUnderstanding> {
    let repo_path = repo_path.as_ref();
    let repo_name = repo_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut directory_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut directory_extensions: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut extension_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut notable_files: Vec<String> = Vec::new();
    let mut dependency_hints: Vec<DependencyHint> = Vec::new();
    let mut seen_dependencies: HashSet<(String, String)> = HashSet::new();

    let import_regex = Regex::new(r#"(?:import|export|require)\s+(?:[\w*{}\s,]+from\s+)?['\"]([^'\"]+)['\"]"#)
        .context("failed to compile import regex")?;

    for entry in WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let relative = path.strip_prefix(repo_path).unwrap_or(path).to_string_lossy().to_string();

        if should_skip(&relative) {
            continue;
        }

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("noext").to_lowercase();
        *extension_counts.entry(extension.clone()).or_insert(0) += 1;

        let top_level = relative
            .split('/')
            .next()
            .unwrap_or("<root>")
            .to_string();
        *directory_counts.entry(top_level.clone()).or_insert(0) += 1;
        *directory_extensions
            .entry(top_level.clone())
            .or_default()
            .entry(extension.clone())
            .or_insert(0) += 1;

        if is_notable_file(&relative) {
            notable_files.push(relative.clone());
        }

        if is_code_file(path) {
            let content = fs::read_to_string(path).unwrap_or_default();
            for capture in import_regex.captures_iter(&content) {
                if let Some(import) = capture.get(1) {
                    let module = normalize_import(import.as_str());
                    if module.is_empty() {
                        continue;
                    }

                    let dependency = (relative.clone(), module.clone());
                    if seen_dependencies.insert(dependency) {
                        dependency_hints.push(DependencyHint {
                            from: relative.clone(),
                            to: module,
                            kind: "import".to_string(),
                        });
                    }
                }
            }
        }
    }

    let mut top_level_directories = directory_counts
        .into_iter()
        .map(|(name, count)| DirectorySummary {
            name: name.clone(),
            file_count: count,
            extension_counts: extension_counts_for_directory(name.as_str(), &directory_extensions),
        })
        .collect::<Vec<_>>();

    top_level_directories.sort_by(|a, b| b.file_count.cmp(&a.file_count));

    let mut extensions = extension_counts
        .into_iter()
        .map(|(extension, count)| ExtensionSummary { extension, count })
        .collect::<Vec<_>>();
    extensions.sort_by(|a, b| b.count.cmp(&a.count));

    let inferred_domains = infer_domains(&top_level_directories, &extensions);
    let summary = summarize_repository(&repo_name, &top_level_directories, &inferred_domains);
    let synthetic_brief = render_brief(&repo_name, &top_level_directories, &inferred_domains, &dependency_hints);

    Ok(SyntheticUnderstanding {
        repository: repo_name,
        generated_at: Utc::now().to_rfc3339(),
        summary,
        top_level_directories,
        extensions,
        inferred_domains,
        dependency_hints,
        notable_files,
        synthetic_brief,
    })
}

/// Write a synthetic understanding artifact to disk as JSON and Markdown.
pub fn write_synthetic_understanding(
    repo_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<PathBuf> {
    let repo_path = repo_path.as_ref();
    let output_path = output_path.as_ref();
    let output_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir).context("failed to create output directory")?;

    let understanding = build_synthetic_understanding(repo_path)?;
    let json_path = output_path.to_path_buf();
    let markdown_path = output_path.with_extension("md");

    let json = serde_json::to_string_pretty(&understanding)?;
    fs::write(&json_path, json).context("failed to write synthetic understanding json")?;
    fs::write(&markdown_path, render_markdown(&understanding))
        .context("failed to write synthetic understanding markdown")?;

    Ok(json_path)
}

fn should_skip(relative_path: &str) -> bool {
    let normalized = relative_path.trim().replace('\\', "/");
    let first_segment = normalized.split('/').next().unwrap_or("");

    normalized.is_empty()
        || normalized == ".git"
        || normalized.starts_with(".git/")
        || normalized.contains("/.git/")
        || first_segment == "node_modules"
        || first_segment == "target"
        || first_segment == "dist"
        || first_segment == "build"
        || first_segment == ".venv"
        || normalized.contains("/node_modules/")
        || normalized.contains("/target/")
        || normalized.contains("/dist/")
        || normalized.contains("/build/")
        || normalized.contains("/.venv/")
}

fn is_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "py" | "rs"))
        .unwrap_or(false)
}

fn is_notable_file(relative_path: &str) -> bool {
    relative_path.contains("README")
        || relative_path.contains("package.json")
        || relative_path.contains("docker")
        || relative_path.contains("terraform")
        || relative_path.contains("agent")
        || relative_path.contains("workflow")
        || relative_path.contains("docs")
}

fn normalize_import(module: &str) -> String {
    let trimmed = module.trim();
    if trimmed.starts_with('.') || trimmed.starts_with('/') || trimmed.starts_with('@') {
        trimmed.to_string()
    } else {
        "external".to_string()
    }
}

fn extension_counts_for_directory(
    directory: &str,
    directory_extensions: &BTreeMap<String, BTreeMap<String, usize>>,
) -> Vec<ExtensionSummary> {
    directory_extensions
        .get(directory)
        .map(|counts| {
            counts
                .iter()
                .map(|(extension, count)| ExtensionSummary {
                    extension: extension.clone(),
                    count: *count,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn infer_domains(top_level_directories: &[DirectorySummary], extensions: &[ExtensionSummary]) -> Vec<String> {
    let mut inferred = Vec::new();
    let lower_names = top_level_directories.iter().map(|d| d.name.to_lowercase()).collect::<Vec<_>>();

    if lower_names.iter().any(|name| name.contains("android") || name.contains("mobile")) {
        inferred.push("mobile application".to_string());
    }
    if lower_names.iter().any(|name| name.contains("worker") || name.contains("dev")) {
        inferred.push("backend worker services".to_string());
    }
    if lower_names.iter().any(|name| name.contains("docs") || name.contains("agent")) {
        inferred.push("documentation and agent workflows".to_string());
    }
    if lower_names.iter().any(|name| name.contains("terraform") || name.contains("docker") || name.contains("mosquitto")) {
        inferred.push("infrastructure and deployment".to_string());
    }
    if extensions.iter().any(|ext| matches!(ext.extension.as_str(), "ts" | "tsx" | "js" | "jsx")) {
        inferred.push("JavaScript/TypeScript application code".to_string());
    }
    inferred
}

fn summarize_repository(
    repo_name: &str,
    directories: &[DirectorySummary],
    domains: &[String],
) -> String {
    let mut summary = format!("{} appears to be a multi-service repository with {} top-level areas.", repo_name, directories.len());
    if !domains.is_empty() {
        summary.push_str(" Its structure suggests domains such as ");
        summary.push_str(&domains.join(", "));
        summary.push('.');
    }
    summary
}

fn render_brief(
    repo_name: &str,
    directories: &[DirectorySummary],
    domains: &[String],
    dependency_hints: &[DependencyHint],
) -> String {
    let mut lines = vec![format!("Synthetic understanding for {repo_name}:")];
    lines.push("- Primary areas:".to_string());
    for directory in directories.iter().take(6) {
        lines.push(format!("  - {}: {} files", directory.name, directory.file_count));
    }
    if !domains.is_empty() {
        lines.push("- Inferred domains:".to_string());
        for domain in domains {
            lines.push(format!("  - {domain}"));
        }
    }
    if !dependency_hints.is_empty() {
        lines.push("- Dependency hints:".to_string());
        for hint in dependency_hints.iter().take(8) {
            lines.push(format!("  - {} -> {} ({})", hint.from, hint.to, hint.kind));
        }
    }
    lines.join("\n")
}

fn render_markdown(understanding: &SyntheticUnderstanding) -> String {
    let mut lines = vec![
        format!("# Synthetic Understanding for {}", understanding.repository),
        String::new(),
        format!("Generated at: {}", understanding.generated_at),
        String::new(),
        understanding.summary.clone(),
        String::new(),
        "## Top-level directories".to_string(),
    ];

    for directory in &understanding.top_level_directories {
        lines.push(format!("- {}: {} files", directory.name, directory.file_count));
    }

    lines.push(String::new());
    lines.push("## Inferred domains".to_string());
    for domain in &understanding.inferred_domains {
        lines.push(format!("- {domain}"));
    }

    lines.push(String::new());
    lines.push("## Notable files".to_string());
    for path in &understanding.notable_files {
        lines.push(format!("- {path}"));
    }

    lines.push(String::new());
    lines.push("## Synthetic brief".to_string());
    lines.push(understanding.synthetic_brief.clone());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builds_understanding_from_a_local_repo() {
        let temp_dir = tempdir().unwrap();
        let repo = temp_dir.path();
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::create_dir_all(repo.join("docs")).unwrap();
        fs::write(repo.join("src/main.ts"), "import './lib';\n").unwrap();
        fs::write(repo.join("src/lib.ts"), "export const x = 1;\n").unwrap();
        fs::write(repo.join("docs/readme.md"), "# docs\n").unwrap();

        let understanding = build_synthetic_understanding(repo).unwrap();
        assert_eq!(understanding.repository, repo.file_name().unwrap().to_string_lossy());
        assert!(!understanding.summary.is_empty());
        assert!(understanding.top_level_directories.iter().any(|dir| dir.name == "src"));
    }
}
