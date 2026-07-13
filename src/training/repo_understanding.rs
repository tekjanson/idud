//! Synthetic understanding generation for repository training, system mapping,
//! and product-journey discovery.
//!
//! This module turns deterministic repository signals (directory layout, file
//! categories, import relationships, and tests) into a compact, AI-readable
//! summary that can be used to define the system and map customer journeys.

use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
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
    pub journey_candidates: Vec<JourneyCandidate>,
    pub test_inventory: Vec<TestSummary>,
    pub graph_nodes: Vec<GraphNode>,
    pub graph_edges: Vec<GraphEdge>,
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

/// A customer-journey candidate inferred from tests and product-facing filenames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyCandidate {
    pub name: String,
    pub evidence: Vec<String>,
    pub related_files: Vec<String>,
}

/// A discovered test file with its inferred title and linked implementation files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub path: String,
    pub kind: String,
    pub title: String,
    pub linked_files: Vec<String>,
}

/// A node in the generated contract-centric graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub path: Option<String>,
    pub hash: Option<String>,
    pub group: String,
    pub description: String,
}

/// An edge in the generated contract-centric graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: String,
    pub confidence: Option<f32>,
}

/// Build a synthetic understanding artifact for a local repository.
pub fn build_synthetic_understanding(repo_path: impl AsRef<Path>) -> Result<SyntheticUnderstanding> {
    let repo_path = repo_path.as_ref();
    let repo_name = repo_path
        .file_name()
        .and_then(|name| {
            let name = name.to_string_lossy();
            if name == "." || name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .or_else(|| {
            repo_path
                .canonicalize()
                .ok()
                .and_then(|path| path.file_name().map(|name| name.to_string_lossy().to_string()))
        })
        .unwrap_or_else(|| "repository".to_string());

    let mut directory_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut directory_extensions: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut extension_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut notable_files: Vec<String> = Vec::new();
    let mut dependency_hints: Vec<DependencyHint> = Vec::new();
    let mut seen_dependencies: HashSet<(String, String)> = HashSet::new();
    let mut discovered_tests: Vec<(String, String)> = Vec::new();
    let mut discovered_code_files: Vec<String> = Vec::new();
    let mut graph_nodes: Vec<GraphNode> = Vec::new();
    let mut graph_edges: Vec<GraphEdge> = Vec::new();
    let mut node_ids: HashSet<String> = HashSet::new();
    let mut file_node_ids: HashMap<String, String> = HashMap::new();

    let import_regex = Regex::new(r#"(?:import|export|require)\s+(?:[\w*{}\s,]+from\s+)?['\"]([^'\"]+)['\"]"#)
        .context("failed to compile import regex")?;

    let (contract_nodes, contract_edges) = load_contract_graph(repo_path);
    for node in contract_nodes {
        let node_id = node.id.clone();
        if node_ids.insert(node_id.clone()) {
            graph_nodes.push(node);
        }
    }
    graph_edges.extend(contract_edges);

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

        let content = fs::read_to_string(path).unwrap_or_default();
        let is_test = is_test_file(&relative);
        let node_id = if is_test {
            format!("test:{}", relative)
        } else {
            format!("file:{}", relative)
        };
        let label = relative
            .split('/')
            .last()
            .unwrap_or(&relative)
            .to_string();
        let node = GraphNode {
            id: node_id.clone(),
            label: label.clone(),
            kind: if is_test { "test".to_string() } else { "file".to_string() },
            path: Some(relative.clone()),
            hash: None,
            group: group_for_path(&relative),
            description: if is_test {
                format!("Test artifact: {}", relative)
            } else {
                format!("Repository artifact: {}", relative)
            },
        };
        if node_ids.insert(node_id.clone()) {
            graph_nodes.push(node);
        }
        file_node_ids.insert(relative.clone(), node_id.clone());

        let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let hash_node_id = format!("hash:{}:{}", relative.replace('/', "_"), &hash[..12]);
        let hash_node = GraphNode {
            id: hash_node_id.clone(),
            label: format!("blake3 {}", &hash[..12]),
            kind: "hash".to_string(),
            path: Some(relative.clone()),
            hash: Some(hash.clone()),
            group: "hashes".to_string(),
            description: format!("Content hash for {}", relative),
        };
        if node_ids.insert(hash_node_id.clone()) {
            graph_nodes.push(hash_node);
        }
        graph_edges.push(GraphEdge {
            from: node_id.clone(),
            to: hash_node_id,
            kind: "hash".to_string(),
            label: "content hash".to_string(),
            confidence: None,
        });

        if is_test {
            discovered_tests.push((relative.clone(), content.clone()));
        } else if is_code_file(path) {
            discovered_code_files.push(relative.clone());
        }

        if is_code_file(path) {
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
                            to: module.clone(),
                            kind: "import".to_string(),
                        });
                    }

                    if let Some(target_path) = resolve_import_target(repo_path, &relative, module.as_str()) {
                        let target_node_id = file_node_ids
                            .entry(target_path.clone())
                            .or_insert_with(|| {
                                let target_node = GraphNode {
                                    id: format!("file:{}", target_path),
                                    label: target_path
                                        .split('/')
                                        .last()
                                        .unwrap_or(&target_path)
                                        .to_string(),
                                    kind: "file".to_string(),
                                    path: Some(target_path.clone()),
                                    hash: None,
                                    group: group_for_path(&target_path),
                                    description: format!("Resolved import target: {}", target_path),
                                };
                                if node_ids.insert(target_node.id.clone()) {
                                    graph_nodes.push(target_node);
                                }
                                format!("file:{}", target_path)
                            })
                            .clone();
                        graph_edges.push(GraphEdge {
                            from: node_id.clone(),
                            to: target_node_id,
                            kind: "import".to_string(),
                            label: "import".to_string(),
                            confidence: None,
                        });
                    }
                }
            }
        }
    }

    let mut journey_map: HashMap<String, JourneyCandidate> = HashMap::new();
    let mut test_inventory: Vec<TestSummary> = Vec::new();

    for (relative, content) in discovered_tests {
        let title = infer_test_title(&relative, &content);
        let kind = infer_test_kind(&relative);
        let linked_files = find_related_code_files(&relative, &discovered_code_files);
        let journey_tags = infer_journey_tags(&relative, &content);

        for journey in &journey_tags {
            let candidate = journey_map.entry(journey.clone()).or_insert_with(|| JourneyCandidate {
                name: journey.clone(),
                evidence: Vec::new(),
                related_files: Vec::new(),
            });
            if !candidate.evidence.contains(&relative) {
                candidate.evidence.push(relative.clone());
            }
            for linked_file in &linked_files {
                if !candidate.related_files.contains(linked_file) {
                    candidate.related_files.push(linked_file.clone());
                }
            }
        }

        test_inventory.push(TestSummary {
            path: relative.clone(),
            kind: kind.to_string(),
            title,
            linked_files: linked_files.clone(),
        });

        let test_node_id = file_node_ids.get(&relative).cloned().unwrap_or_else(|| format!("test:{}", relative));
        for linked_file in linked_files {
            let dependency = (relative.clone(), linked_file.clone());
            if seen_dependencies.insert(dependency) {
                dependency_hints.push(DependencyHint {
                    from: relative.clone(),
                    to: linked_file.clone(),
                    kind: "test-link".to_string(),
                });
            }

            if let Some(target_node_id) = file_node_ids.get(&linked_file) {
                graph_edges.push(GraphEdge {
                    from: test_node_id.clone(),
                    to: target_node_id.clone(),
                    kind: "test-link".to_string(),
                    label: "test linkage".to_string(),
                    confidence: None,
                });
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

    let mut journey_candidates = journey_map.into_values().collect::<Vec<_>>();
    journey_candidates.sort_by(|a, b| a.name.cmp(&b.name));
    test_inventory.sort_by(|a, b| a.path.cmp(&b.path));

    let inferred_domains = infer_domains(&top_level_directories, &extensions);
    let summary = summarize_repository(
        &repo_name,
        &top_level_directories,
        &inferred_domains,
        &journey_candidates,
        &test_inventory,
    );
    let synthetic_brief = render_brief(
        &repo_name,
        &top_level_directories,
        &inferred_domains,
        &dependency_hints,
        &journey_candidates,
        &test_inventory,
    );

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
        journey_candidates,
        test_inventory,
        graph_nodes,
        graph_edges,
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
    let html_path = output_path.with_extension("html");

    let json = serde_json::to_string_pretty(&understanding)?;
    fs::write(&json_path, json).context("failed to write synthetic understanding json")?;
    fs::write(&markdown_path, render_markdown(&understanding))
        .context("failed to write synthetic understanding markdown")?;
    fs::write(&html_path, render_html(&understanding))
        .context("failed to write synthetic understanding html")?;

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

    if lower_names.iter().any(|name| name.contains("mobile") || name.contains("android") || name.contains("ios")) {
        inferred.push("mobile application surfaces".to_string());
    }
    if lower_names.iter().any(|name| name.contains("worker") || name.contains("service") || name.contains("server")) {
        inferred.push("backend service workflows".to_string());
    }
    if lower_names.iter().any(|name| name.contains("docs") || name.contains("guide") || name.contains("agent")) {
        inferred.push("documentation and operating workflows".to_string());
    }
    if lower_names.iter().any(|name| name.contains("infra") || name.contains("terraform") || name.contains("docker") || name.contains("deploy")) {
        inferred.push("infrastructure and deployment".to_string());
    }
    if lower_names.iter().any(|name| name.contains("test") || name.contains("spec") || name.contains("e2e")) {
        inferred.push("quality assurance and regression coverage".to_string());
    }
    if extensions.iter().any(|ext| matches!(ext.extension.as_str(), "ts" | "tsx" | "js" | "jsx" | "py" | "rs")) {
        inferred.push("application logic and integrations".to_string());
    }
    inferred
}

fn summarize_repository(
    repo_name: &str,
    directories: &[DirectorySummary],
    domains: &[String],
    journey_candidates: &[JourneyCandidate],
    test_inventory: &[TestSummary],
) -> String {
    let mut summary = format!(
        "{} appears to be a product-oriented repository with {} top-level areas, {} inferred journey candidates, and {} tests.",
        repo_name,
        directories.len(),
        journey_candidates.len(),
        test_inventory.len()
    );
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
    journey_candidates: &[JourneyCandidate],
    test_inventory: &[TestSummary],
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
    if !journey_candidates.is_empty() {
        lines.push("- Customer journeys:".to_string());
        for journey in journey_candidates.iter().take(6) {
            lines.push(format!("  - {} (evidence: {})", journey.name, journey.evidence.len()));
        }
    }
    if !test_inventory.is_empty() {
        lines.push("- Test inventory:".to_string());
        for test in test_inventory.iter().take(6) {
            lines.push(format!("  - {} [{}] -> {}", test.title, test.kind, test.path));
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
        "## Customer journeys".to_string(),
    ];

    if understanding.journey_candidates.is_empty() {
        lines.push("- No journey candidates were inferred.".to_string());
    } else {
        for journey in &understanding.journey_candidates {
            lines.push(format!("- {}", journey.name));
            lines.push(format!("  - Evidence: {}", journey.evidence.join(", ")));
            if !journey.related_files.is_empty() {
                lines.push(format!("  - Related files: {}", journey.related_files.join(", ")));
            }
        }
    }

    lines.push(String::new());
    lines.push("## Test inventory".to_string());
    if understanding.test_inventory.is_empty() {
        lines.push("- No tests were discovered.".to_string());
    } else {
        for test in &understanding.test_inventory {
            lines.push(format!("- {} [{}]", test.title, test.kind));
            lines.push(format!("  - Path: {}", test.path));
            if !test.linked_files.is_empty() {
                lines.push(format!("  - Linked files: {}", test.linked_files.join(", ")));
            }
        }
    }

    lines.push(String::new());
    lines.push("## Top-level directories".to_string());
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

fn render_html(understanding: &SyntheticUnderstanding) -> String {
    let dirs_html = understanding
        .top_level_directories
        .iter()
        .map(|dir| {
            let ext_items = dir
                .extension_counts
                .iter()
                .map(|ext| format!("<li>{}: {}</li>", escape_html(&ext.extension), ext.count))
                .collect::<Vec<_>>()
                .join("");
            format!(
                "<section class='card'><h3>{}</h3><p>{} files</p><ul>{}</ul></section>",
                escape_html(&dir.name),
                dir.file_count,
                ext_items
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let domains_html = understanding
        .inferred_domains
        .iter()
        .map(|domain| format!("<li>{}</li>", escape_html(domain)))
        .collect::<Vec<_>>()
        .join("");

    let files_html = understanding
        .notable_files
        .iter()
        .map(|path| format!("<li>{}</li>", escape_html(path)))
        .collect::<Vec<_>>()
        .join("");

    let hints_html = understanding
        .dependency_hints
        .iter()
        .map(|hint| {
            let from = escape_html(&hint.from);
            let to = escape_html(&hint.to);
            format!("<li><span class='node'>{from}</span> → <span class='node'>{to}</span> <span class='pill'>{}</span></li>", escape_html(&hint.kind))
        })
        .collect::<Vec<_>>()
        .join("");

    let journeys_html = understanding
        .journey_candidates
        .iter()
        .map(|journey| {
            let evidence = escape_html(&journey.evidence.join(", "));
            let related_files = escape_html(&journey.related_files.join(", "));
            format!(
                "<li><strong>{}</strong><div class='muted'>Evidence: {}</div><div class='muted'>Related files: {}</div></li>",
                escape_html(&journey.name),
                evidence,
                related_files
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let tests_html = understanding
        .test_inventory
        .iter()
        .map(|test| {
            let linked = escape_html(&test.linked_files.join(", "));
            format!(
                "<li><strong>{}</strong> <span class='pill'>{}</span><div class='muted'>{}</div><div class='muted'>Linked files: {}</div></li>",
                escape_html(&test.title),
                escape_html(&test.kind),
                escape_html(&test.path),
                linked
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let mut ordered_nodes = understanding.graph_nodes.clone();
    ordered_nodes.sort_by(|a, b| {
        let rank = |kind: &str| match kind {
            "contract" => 0,
            "test" => 1,
            "file" => 2,
            "hash" => 3,
            _ => 4,
        };
        rank(&a.kind).cmp(&rank(&b.kind)).then_with(|| a.label.cmp(&b.label))
    });

    let columns = if ordered_nodes.len() > 16 { 4 } else if ordered_nodes.len() > 8 { 3 } else { 2 };
    let node_width = 170;
    let node_height = 92;
    let svg_width = 40 + columns * node_width + 20;
    let svg_height = 40 + ((ordered_nodes.len() as f32 / columns as f32).ceil() as usize) * node_height + 20;

    let mut node_positions: HashMap<String, (usize, usize)> = HashMap::new();
    let mut node_svg = String::new();
    let mut edge_svg = String::new();

    for (index, node) in ordered_nodes.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = 30 + column * node_width;
        let y = 30 + row * node_height;
        node_positions.insert(node.id.clone(), (x, y));

        let color = match node.kind.as_str() {
            "contract" => "#dbeafe",
            "test" => "#fef3c7",
            "hash" => "#dcfce7",
            "file" => "#f3e8ff",
            _ => "#e5e7eb",
        };
        let title = escape_html(&node.label);
        let subtitle = escape_html(node.path.as_deref().unwrap_or_default());
        node_svg.push_str(&format!(
            "<g><rect x='{x}' y='{y}' width='{node_width}' height='{node_height}' rx='12' fill='{color}' stroke='#6b7280' /><text x='{}' y='{}' font-size='13' font-weight='700' fill='#111827'>{}</text><text x='{}' y='{}' font-size='10' fill='#374151'>{}</text><text x='{}' y='{}' font-size='9' fill='#4b5563'>{}</text></g>",
            x + 10,
            y + 24,
            title,
            x + 10,
            y + 46,
            subtitle,
            x + 10,
            y + 66,
            escape_html(&node.kind)
        ));
    }

    for edge in &understanding.graph_edges {
        if let Some((x1, y1)) = node_positions.get(&edge.from) {
            if let Some((x2, y2)) = node_positions.get(&edge.to) {
                let x1 = x1 + node_width / 2;
                let y1 = y1 + node_height / 2;
                let x2 = x2 + node_width / 2;
                let y2 = y2 + node_height / 2;
                edge_svg.push_str(&format!(
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='#94a3b8' stroke-width='1.5' marker-end='url(#arrow)' />"
                ));
            }
        }
    }

    let contracts_html = understanding
        .graph_nodes
        .iter()
        .filter(|node| node.kind == "contract")
        .take(20)
        .map(|node| {
            let path = node.path.as_deref().unwrap_or_default();
            format!(
                "<li><strong>{}</strong><div class='muted'>{}</div></li>",
                escape_html(&node.label),
                escape_html(path)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let hashes_html = understanding
        .graph_nodes
        .iter()
        .filter(|node| node.kind == "hash")
        .take(10)
        .map(|node| {
            format!(
                "<li><strong>{}</strong><div class='muted'>{}</div></li>",
                escape_html(&node.label),
                escape_html(node.path.as_deref().unwrap_or_default())
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Synthetic Understanding for {}</title>
  <style>
    body {{ font-family: Arial, sans-serif; margin: 2rem; line-height: 1.5; color: #111827; }}
    .panel {{ border: 1px solid #d1d5db; border-radius: 10px; padding: 1rem 1.25rem; margin-bottom: 1rem; background: #fff; }}
    .graph {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1rem; }}
    .card {{ border: 1px solid #e5e7eb; border-radius: 8px; padding: 0.75rem; background: #f9fafb; }}
    .node {{ background: #eff6ff; padding: 0.2rem 0.45rem; border-radius: 999px; font-size: 0.9rem; display: inline-block; margin: 0.15rem 0; word-break: break-all; }}
    .pill {{ background: #f3f4f6; padding: 0.2rem 0.45rem; border-radius: 999px; font-size: 0.8rem; color: #4b5563; }}
    .muted {{ color: #6b7280; font-size: 0.95rem; margin-top: 0.2rem; }}
    h1, h2, h3 {{ color: #111827; }}
    ul {{ padding-left: 1.2rem; }}
    li {{ margin-bottom: 0.35rem; }}
    .summary {{ display: flex; flex-wrap: wrap; gap: 0.75rem; }}
    .chip {{ border: 1px solid #d1d5db; border-radius: 999px; padding: 0.25rem 0.7rem; background: #f9fafb; }}
  </style>
</head>
<body>
  <h1>Synthetic Understanding for {}</h1>
  <p><strong>Generated:</strong> {}</p>
  <p>{}</p>

  <div class="panel">
    <h2>Contract-first system map</h2>
    <div class="summary">
      <span class="chip">Contracts: {}</span>
      <span class="chip">Tests: {}</span>
      <span class="chip">Files: {}</span>
      <span class="chip">Hashes: {}</span>
    </div>
    <svg viewBox="0 0 {svg_width} {svg_height}" width="100%" height="auto" xmlns="http://www.w3.org/2000/svg">
      <defs><marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto" markerUnits="strokeWidth"><path d="M0,0 L0,6 L6,3 z" fill="#94a3b8"/></marker></defs>
      {edge_svg}
      {node_svg}
    </svg>
  </div>

  <div class="panel">
    <h2>Contract nodes</h2>
    <ul>{contracts_html}</ul>
  </div>

  <div class="panel">
    <h2>Hash anchors</h2>
    <ul>{hashes_html}</ul>
  </div>

  <div class="panel">
    <h2>Customer journeys</h2>
    <ul>{journeys_html}</ul>
  </div>

  <div class="panel">
    <h2>Test inventory</h2>
    <ul>{tests_html}</ul>
  </div>

  <div class="panel">
    <h2>File-to-file links</h2>
    <ul>{hints_html}</ul>
  </div>

  <div class="panel">
    <h2>Directory map</h2>
    <div class="graph">{dirs_html}</div>
  </div>

  <div class="panel">
    <h2>Inferred domains</h2>
    <ul>{domains_html}</ul>
  </div>

  <div class="panel">
    <h2>Notable files</h2>
    <ul>{files_html}</ul>
  </div>
</body>
</html>"##,
        escape_html(&understanding.repository),
        escape_html(&understanding.repository),
        escape_html(&understanding.generated_at),
        escape_html(&understanding.summary),
        understanding.graph_nodes.iter().filter(|node| node.kind == "contract").count(),
        understanding.test_inventory.len(),
        understanding.graph_nodes.iter().filter(|node| node.kind == "file").count(),
        understanding.graph_nodes.iter().filter(|node| node.kind == "hash").count()
    )
}

fn load_contract_graph(repo_path: &Path) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_ids = HashSet::new();

    let data_dir = repo_path.join("data");
    let Ok(entries) = fs::read_dir(&data_dir) else {
        return (nodes, edges);
    };

    let mut contract_files = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && (path.file_name().and_then(|name| name.to_str()).unwrap_or_default().contains("contracts")
                    || path.file_name().and_then(|name| name.to_str()).unwrap_or_default().ends_with("-contracts.json"))
            {
                contract_files.push(path);
            }
        }
    }
    contract_files.sort();

    for path in contract_files {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };

        if let Some(signatories) = value.get("signatories").and_then(|v| v.as_array()) {
            for signatory in signatories {
                let Some(signatory_obj) = signatory.as_object() else {
                    continue;
                };
                let id = signatory_obj
                    .get("id")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                let label = signatory_obj
                    .get("label")
                    .and_then(|value| value.as_str())
                    .unwrap_or("contract")
                    .to_string();
                let path_value = signatory_obj
                    .get("metadata")
                    .and_then(|value| value.as_object())
                    .and_then(|metadata| metadata.get("filePath"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        signatory_obj
                            .get("source_uri")
                            .and_then(|value| value.as_str())
                            .map(|uri| uri.split('#').next().unwrap_or(uri).trim_start_matches("./blob/main/").to_string())
                    });
                ensure_graph_node(
                    &mut nodes,
                    &mut node_ids,
                    format!("contract:{}", id),
                    label,
                    "contract".to_string(),
                    path_value,
                    "contracts".to_string(),
                    "Contract signatory extracted from ledger export".to_string(),
                );
            }
        }

        if let Some(contracts) = value.get("contracts").and_then(|v| v.as_array()) {
            for contract in contracts {
                let Some(contract_obj) = contract.as_object() else {
                    continue;
                };
                let principal_id = contract_obj
                    .get("principal_id")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let guarantor_id = contract_obj
                    .get("guarantor_id")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if principal_id.is_empty() || guarantor_id.is_empty() {
                    continue;
                }
                let clause_type = contract_obj
                    .get("clause_type")
                    .and_then(|value| value.as_str())
                    .unwrap_or("contract")
                    .to_string();
                let confidence = contract_obj
                    .get("confidence")
                    .and_then(|value| value.as_f64())
                    .map(|value| value as f32);
                ensure_graph_node(
                    &mut nodes,
                    &mut node_ids,
                    format!("contract:{}", principal_id),
                    format!("{}", principal_id),
                    "contract".to_string(),
                    None,
                    "contracts".to_string(),
                    "Contract principal".to_string(),
                );
                ensure_graph_node(
                    &mut nodes,
                    &mut node_ids,
                    format!("contract:{}", guarantor_id),
                    format!("{}", guarantor_id),
                    "contract".to_string(),
                    None,
                    "contracts".to_string(),
                    "Contract guarantor".to_string(),
                );
                edges.push(GraphEdge {
                    from: format!("contract:{}", principal_id),
                    to: format!("contract:{}", guarantor_id),
                    kind: "contract".to_string(),
                    label: clause_type,
                    confidence,
                });
            }
        }
    }

    (nodes, edges)
}

fn ensure_graph_node(
    nodes: &mut Vec<GraphNode>,
    node_ids: &mut HashSet<String>,
    id: String,
    label: String,
    kind: String,
    path: Option<String>,
    group: String,
    description: String,
) {
    if node_ids.insert(id.clone()) {
        nodes.push(GraphNode {
            id,
            label,
            kind,
            path,
            hash: None,
            group,
            description,
        });
    }
}

fn group_for_path(relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    let top_level = normalized.split('/').next().unwrap_or("root");
    match top_level {
        "src" => "source".to_string(),
        "tests" => "tests".to_string(),
        "data" => "data".to_string(),
        "docs" | "documentation" => "docs".to_string(),
        _ => "root".to_string(),
    }
}

fn resolve_import_target(repo_path: &Path, current_relative: &str, module: &str) -> Option<String> {
    let trimmed = module.trim();
    if trimmed.is_empty() || trimmed.starts_with("external") || trimmed.starts_with("http") {
        return None;
    }

    let current_dir = Path::new(current_relative)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let candidates = [
        repo_path.join(trimmed),
        repo_path.join(current_dir).join(trimmed),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return candidate.strip_prefix(repo_path).ok().map(|path| path.to_string_lossy().to_string());
        }
        let mut with_extension = candidate.clone();
        for ext in [".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".md"] {
            with_extension.set_extension(ext.trim_start_matches('.'));
            if with_extension.is_file() {
                return with_extension.strip_prefix(repo_path).ok().map(|path| path.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn is_test_file(relative_path: &str) -> bool {
    let lower = relative_path.to_lowercase();
    let file_name = lower.split('/').last().unwrap_or(&lower);
    lower.contains("/__tests__/")
        || lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.contains("/spec/")
        || lower.contains("/specs/")
        || file_name.contains(".test.")
        || file_name.contains(".spec.")
        || file_name.contains("_test.")
        || file_name.contains("_spec.")
        || file_name.contains("-test.")
        || file_name.contains("-spec.")
        || file_name.starts_with("test")
        || file_name.starts_with("spec")
}

fn infer_test_title(relative_path: &str, content: &str) -> String {
    let title_regex = Regex::new(r#"(?i)(?:describe|it|test)\s*\(\s*['\"]([^'\"]+)['\"]"#).unwrap();
    if let Some(capture) = title_regex.captures(content) {
        if let Some(value) = capture.get(1) {
            return value.as_str().trim().to_string();
        }
    }

    let stem = relative_path
        .split('/')
        .last()
        .unwrap_or(relative_path)
        .to_lowercase();
    let normalized = stem
        .split(|c: char| c == '.' || c == '_' || c == '-' || c == '/')
        .filter(|token| !token.is_empty() && !matches!(*token, "test" | "tests" | "spec" | "specs" | "e2e" | "unit" | "integration" | "acceptance"))
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        "repository workflow".to_string()
    } else {
        normalized
    }
}

fn infer_test_kind(relative_path: &str) -> &'static str {
    let lower = relative_path.to_lowercase();
    if lower.contains("e2e") || lower.contains("end-to-end") {
        "e2e"
    } else if lower.contains("integration") || lower.contains("contract") {
        "integration"
    } else if lower.contains("api") {
        "api"
    } else {
        "unit"
    }
}

fn infer_journey_tags(relative_path: &str, content: &str) -> Vec<String> {
    let text = format!("{} {}", relative_path.to_lowercase(), content.to_lowercase());
    let mut tags = Vec::new();
    let keyword_map = [
        ("auth", "authentication"),
        ("login", "authentication"),
        ("signin", "authentication"),
        ("signup", "authentication"),
        ("register", "authentication"),
        ("checkout", "checkout"),
        ("payment", "checkout"),
        ("billing", "checkout"),
        ("cart", "checkout"),
        ("search", "discovery"),
        ("browse", "discovery"),
        ("find", "discovery"),
        ("onboard", "onboarding"),
        ("welcome", "onboarding"),
        ("invite", "onboarding"),
        ("profile", "account management"),
        ("account", "account management"),
        ("settings", "account management"),
        ("notify", "notifications"),
        ("message", "communications"),
        ("share", "collaboration"),
        ("workspace", "collaboration"),
        ("import", "data operations"),
        ("export", "data operations"),
        ("sync", "data operations"),
        ("backup", "data operations"),
    ];

    for (keyword, tag) in keyword_map {
        if text.contains(keyword) && !tags.contains(&tag.to_string()) {
            tags.push(tag.to_string());
        }
    }

    if tags.is_empty() {
        tags.push("core workflow".to_string());
    }

    tags
}

fn find_related_code_files(relative_path: &str, discovered_code_files: &[String]) -> Vec<String> {
    let test_tokens = tokenize_path(relative_path);
    let mut related = Vec::new();

    for code_file in discovered_code_files {
        let code_tokens = tokenize_path(code_file);
        let overlap = test_tokens.iter().filter(|token| code_tokens.contains(*token)).count();
        if overlap > 0 {
            related.push(code_file.clone());
        }
    }

    related.sort();
    related.dedup();
    related.truncate(4);
    related
}

fn tokenize_path(path: &str) -> Vec<String> {
    let mut tokens = path
        .to_lowercase()
        .replace('\\', "/")
        .split(|c: char| c == '/' || c == '.' || c == '_' || c == '-' || c == ' ')
        .filter(|token| !token.is_empty())
        .filter(|token| !matches!(*token, "src" | "lib" | "app" | "test" | "tests" | "spec" | "specs" | "e2e" | "integration" | "unit" | "index" | "component" | "module"))
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    tokens.sort();
    tokens.dedup();
    tokens
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
