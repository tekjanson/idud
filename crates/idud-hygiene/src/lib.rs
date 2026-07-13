use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
pub struct GoldenPattern {
    pub name: String,
    pub description: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IncludeSpec {
    Single(String),
    Multiple(Vec<String>),
}

impl IncludeSpec {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            IncludeSpec::Single(pattern) => vec![pattern.clone()],
            IncludeSpec::Multiple(patterns) => patterns.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PatternSpec {
    Single(String),
    Multiple(Vec<String>),
}

impl PatternSpec {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            PatternSpec::Single(pattern) => vec![pattern.clone()],
            PatternSpec::Multiple(patterns) => patterns.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CallGraphNode {
    pub id: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CallGraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum Rule {
    #[serde(rename = "forbid-pattern")]
    ForbidPattern {
        id: String,
        pattern: PatternSpec,
        include: IncludeSpec,
    },
    #[serde(rename = "require-pattern")]
    RequirePattern {
        id: String,
        pattern: PatternSpec,
        include: IncludeSpec,
    },
    #[serde(rename = "max-file-lines")]
    MaxFileLines {
        id: String,
        max_lines: usize,
        include: IncludeSpec,
    },
    #[serde(rename = "max-parameters")]
    MaxParameters {
        id: String,
        max_parameters: usize,
        include: IncludeSpec,
    },
    #[serde(rename = "max-nesting-depth")]
    MaxNestingDepth {
        id: String,
        max_depth: usize,
        include: IncludeSpec,
    },
    #[serde(rename = "require-call-graph")]
    RequireCallGraph {
        id: String,
        include: IncludeSpec,
        nodes: Vec<CallGraphNode>,
        edges: Vec<CallGraphEdge>,
    },
    #[serde(rename = "require-dependency")]
    RequireDependency {
        id: String,
        pattern: String,
        include: IncludeSpec,
    },
    #[serde(rename = "forbid-dependency")]
    ForbidDependency {
        id: String,
        pattern: String,
        include: IncludeSpec,
    },
    #[serde(rename = "require-naming")]
    RequireNaming {
        id: String,
        target: String,
        pattern: String,
        include: IncludeSpec,
    },
}

impl Rule {
    pub fn forbid_pattern(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        pattern: impl Into<String>,
    ) -> Self {
        Self::ForbidPattern {
            id: id.into(),
            pattern: PatternSpec::Single(pattern.into()),
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn require_pattern(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        pattern: impl Into<String>,
    ) -> Self {
        Self::RequirePattern {
            id: id.into(),
            pattern: PatternSpec::Single(pattern.into()),
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn max_file_lines(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        max_lines: usize,
    ) -> Self {
        Self::MaxFileLines {
            id: id.into(),
            max_lines,
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn max_parameters(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        max_parameters: usize,
    ) -> Self {
        Self::MaxParameters {
            id: id.into(),
            max_parameters,
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn max_nesting_depth(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        max_depth: usize,
    ) -> Self {
        Self::MaxNestingDepth {
            id: id.into(),
            max_depth,
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn require_call_graph(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        nodes: Vec<CallGraphNode>,
        edges: Vec<CallGraphEdge>,
    ) -> Self {
        Self::RequireCallGraph {
            id: id.into(),
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
            nodes,
            edges,
        }
    }

    pub fn require_dependency(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        pattern: impl Into<String>,
    ) -> Self {
        Self::RequireDependency {
            id: id.into(),
            pattern: pattern.into(),
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn forbid_dependency(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        pattern: impl Into<String>,
    ) -> Self {
        Self::ForbidDependency {
            id: id.into(),
            pattern: pattern.into(),
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn require_naming(
        id: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        target: impl Into<String>,
        pattern: impl Into<String>,
    ) -> Self {
        Self::RequireNaming {
            id: id.into(),
            target: target.into(),
            pattern: pattern.into(),
            include: IncludeSpec::Multiple(include.into_iter().map(Into::into).collect()),
        }
    }

    pub fn include(&self) -> Vec<String> {
        match self {
            Rule::ForbidPattern { include, .. }
            | Rule::RequirePattern { include, .. }
            | Rule::MaxFileLines { include, .. }
            | Rule::MaxParameters { include, .. }
            | Rule::MaxNestingDepth { include, .. }
            | Rule::RequireCallGraph { include, .. }
            | Rule::RequireDependency { include, .. }
            | Rule::ForbidDependency { include, .. }
            | Rule::RequireNaming { include, .. } => include.as_vec(),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Rule::ForbidPattern { id, .. }
            | Rule::RequirePattern { id, .. }
            | Rule::MaxFileLines { id, .. }
            | Rule::MaxParameters { id, .. }
            | Rule::MaxNestingDepth { id, .. }
            | Rule::RequireCallGraph { id, .. }
            | Rule::RequireDependency { id, .. }
            | Rule::ForbidDependency { id, .. }
            | Rule::RequireNaming { id, .. } => id,
        }
    }
}

pub fn load_golden_pattern(path: impl AsRef<Path>) -> Result<GoldenPattern> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read golden pattern {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse golden pattern {}", path.display()))
}

#[derive(Debug)]
pub struct RuleReport {
    pub id: String,
    pub passed: bool,
    pub violations: Vec<String>,
}

#[derive(Debug)]
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
        let files = discover_files(repo_root, &rule.include());
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
        let files = discover_files(repo_root, &rule.include());
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
            let files = discover_files(repo_root, &rule.include());
            violations.extend(collect_rule_violations(repo_root, rule, &files)?);
        }
    }

    Ok(violations)
}

struct ManifestView {
    name: String,
    description: String,
    path: String,
    passed: bool,
    passed_rules: usize,
    rule_reports: Vec<RuleReport>,
    rule_specs: Vec<Rule>,
}

fn build_rule_details_markup(
    rule: &Rule,
    manifest_name: &str,
    manifest_description: &str,
) -> String {
    let (intro, bullets, visual) = match rule {
        Rule::ForbidPattern {
            pattern, include, ..
        } => {
            let patterns = pattern.as_vec().join(", ");
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule forbids the pattern <code>{}</code> so it never slips into the targeted files.",
                    escape_html(&patterns)
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>Useful for banning unsafe APIs, secrets, or disallowed imports.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"visual-stack\"><div class=\"visual-block\">Target files<br/><code>{}</code></div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">Forbidden match<br/><code>{}</code></div></div>",
                    escape_html(&includes),
                    escape_html(&patterns)
                ),
            )
        }
        Rule::RequirePattern {
            pattern, include, ..
        } => {
            let patterns = pattern.as_vec().join(", ");
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule requires the pattern <code>{}</code> to appear in the targeted files.",
                    escape_html(&patterns)
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>Use it when a standard entry point, lifecycle hook, or API call must be present.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"visual-stack\"><div class=\"visual-block\">Target files<br/><code>{}</code></div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">Required match<br/><code>{}</code></div></div>",
                    escape_html(&includes),
                    escape_html(&patterns)
                ),
            )
        }
        Rule::MaxFileLines {
            max_lines, include, ..
        } => {
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule keeps files small by enforcing a hard ceiling of {} lines per file.",
                    max_lines
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>It helps keep modules readable and easier to review.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"metric-visual\"><div class=\"metric-track\"><div class=\"metric-fill\" style=\"width: 100%;\"></div></div><div class=\"metric-caption\">Budget: <strong>{}</strong> lines</div></div>",
                    max_lines
                ),
            )
        }
        Rule::MaxParameters {
            max_parameters,
            include,
            ..
        } => {
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule keeps functions focused by limiting them to {} parameters.",
                    max_parameters
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>It pushes complex behavior into smaller units instead of overloading one function.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"metric-visual\"><div class=\"metric-track\"><div class=\"metric-fill\" style=\"width: 100%;\"></div></div><div class=\"metric-caption\">Max parameters: <strong>{}</strong></div></div>",
                    max_parameters
                ),
            )
        }
        Rule::MaxNestingDepth {
            max_depth, include, ..
        } => {
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule flattens control flow by keeping nesting depth at or below {} levels.",
                    max_depth
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>It reduces cognitive load and makes the logic easier to follow.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"visual-stack\"><div class=\"visual-block\">Depth 1</div><div class=\"arrow\">⟶</div><div class=\"visual-block\">Depth 2</div><div class=\"arrow\">⟶</div><div class=\"visual-block\">Depth {}</div></div>",
                    max_depth
                ),
            )
        }
        Rule::RequireCallGraph {
            include,
            nodes,
            edges,
            ..
        } => {
            let includes = include.as_vec().join(", ");
            let mut nodes_markup = String::new();
            for (index, node) in nodes.iter().enumerate() {
                if index > 0 {
                    nodes_markup.push_str("<div class=\"arrow\">⟶</div>");
                }
                nodes_markup.push_str(&format!(
                    "<div class=\"visual-block\">{}</div>",
                    escape_html(&node.id)
                ));
            }
            if nodes_markup.is_empty() {
                nodes_markup = "<div class=\"visual-block\">No nodes declared</div>".to_string();
            }
            (
                "This rule requires the code to form a specific call-graph shape.".to_string(),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>Nodes: {}</li><li>Edges: {}</li>",
                    escape_html(&includes),
                    nodes
                        .iter()
                        .map(|node| node.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    edges
                        .iter()
                        .map(|edge| format!("{} → {}", edge.from, edge.to))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                format!("<div class=\"visual-stack\">{nodes_markup}</div>"),
            )
        }
        Rule::RequireDependency {
            pattern, include, ..
        } => {
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule requires a dependency on <code>{}</code> so the subsystem keeps its expected relationships.",
                    escape_html(pattern)
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>It makes architectural boundaries explicit and discoverable.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"visual-stack\"><div class=\"visual-block\">Current module</div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">{}</div></div>",
                    escape_html(pattern)
                ),
            )
        }
        Rule::ForbidDependency {
            pattern, include, ..
        } => {
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule forbids the dependency <code>{}</code> so the layer stays decoupled from the wrong neighbor.",
                    escape_html(pattern)
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>It prevents accidental coupling and keeps boundaries crisp.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"visual-stack\"><div class=\"visual-block\">Current module</div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">{}</div></div>",
                    escape_html(pattern)
                ),
            )
        }
        Rule::RequireNaming {
            target,
            pattern,
            include,
            ..
        } => {
            let includes = include.as_vec().join(", ");
            (
                format!(
                    "This rule expects every {} to match the naming pattern <code>{}</code>.",
                    target,
                    escape_html(pattern)
                ),
                format!(
                    "<li>Applies to: <code>{}</code></li><li>It stabilizes code reading and makes intent predictable across the repo.</li>",
                    escape_html(&includes)
                ),
                format!(
                    "<div class=\"visual-stack\"><div class=\"visual-block\">{} name</div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">{}</div></div>",
                    escape_html(target),
                    escape_html(pattern)
                ),
            )
        }
    };

    let context_markup = format!(
        "<div class=\"detail-context\"><p class=\"detail-title\">Manifest context</p><p>{}</p></div>",
        escape_html(manifest_description)
    );

    format!(
        "<div class=\"detail-copy\">\n  <p class=\"detail-title\">{}</p>\n  <p>{}</p>\n  <ul class=\"detail-bullets\">{bullets}</ul>\n  {context_markup}\n</div>\n<div class=\"detail-visual\">{visual}</div>",
        escape_html(manifest_name),
        intro
    )
}

pub fn render_hygiene_dashboard(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<String> {
    let repo_root = repo_root.as_ref();
    let manifest_files = resolve_manifest_files(repo_root, manifest_path.as_ref())?;
    let mut manifests = Vec::new();

    for manifest_file in manifest_files {
        let pattern = load_golden_pattern(&manifest_file)?;
        let rule_reports = report_golden_pattern(repo_root, &manifest_file)?;
        let passed_rules = rule_reports.iter().filter(|report| report.passed).count();
        let failed_rules = rule_reports.len().saturating_sub(passed_rules);
        manifests.push(ManifestView {
            name: pattern.name,
            description: pattern.description,
            path: relative_path(repo_root, &manifest_file),
            passed: failed_rules == 0,
            passed_rules,
            rule_reports,
            rule_specs: pattern.rules,
        });
    }

    let total_rules = manifests
        .iter()
        .map(|manifest| manifest.rule_reports.len())
        .sum::<usize>();
    let passed_rules = manifests
        .iter()
        .map(|manifest| manifest.passed_rules)
        .sum::<usize>();
    let failed_rules = total_rules.saturating_sub(passed_rules);
    let passed_manifests = manifests.iter().filter(|manifest| manifest.passed).count();

    let mut cards = String::new();
    for manifest in &manifests {
        let mut rules_markup = String::new();
        for (index, rule) in manifest.rule_specs.iter().enumerate() {
            let report = &manifest.rule_reports[index];
            let status_class = if report.passed { "pass" } else { "fail" };
            let status_label = if report.passed {
                "Passing"
            } else {
                "Needs attention"
            };
            let detail_markup =
                build_rule_details_markup(rule, &manifest.name, &manifest.description);
            let violations_markup = if report.violations.is_empty() {
                "<p class=\"no-violations\">No violations detected.</p>".to_string()
            } else {
                let mut items = String::new();
                for violation in &report.violations {
                    writeln!(items, "<li>{}</li>", escape_html(violation)).unwrap();
                }
                format!("<ul class=\"violations\">{items}</ul>")
            };
            writeln!(
                rules_markup,
                "<article class=\"rule-card {status_class}\">\n  <div class=\"rule-head\">\n    <h4>{}</h4>\n    <span class=\"pill {status_class}\">{}</span>\n  </div>\n  <div class=\"rule-body\">\n    <details class=\"rule-details\">\n      <summary>What this rule means</summary>\n      <div class=\"rule-detail-body\">{}</div>\n    </details>\n    <div class=\"rule-evidence\">{}</div>\n  </div>\n</article>",
                escape_html(&report.id),
                escape_html(status_label),
                detail_markup,
                violations_markup
            )
            .unwrap();
        }
        let manifest_status = if manifest.passed { "PASS" } else { "FAIL" };
        let manifest_status_class = if manifest.passed { "pass" } else { "fail" };
        writeln!(
            cards,
            "<section class=\"manifest-card\">\n  <div class=\"manifest-head\">\n    <div>\n      <p class=\"eyebrow\">Manifest</p>\n      <h3>{}</h3>\n      <p class=\"description\">{}</p>\n    </div>\n    <div class=\"manifest-status\">\n      <span class=\"pill {manifest_status_class}\">{}</span>\n      <p>{}/{} rules passing</p>\n    </div>\n  </div>\n  <p class=\"path\">{}</p>\n  <div class=\"rules-grid\">{}</div>\n</section>",
            escape_html(&manifest.name),
            escape_html(&manifest.description),
            escape_html(manifest_status),
            manifest.passed_rules,
            manifest.rule_reports.len(),
            escape_html(&manifest.path),
            rules_markup
        )
        .unwrap();
    }

    let summary_label = if failed_rules == 0 {
        format!(
            "All hygiene contracts are satisfied • {} of {} manifests passing • {} of {} rules passing",
            passed_manifests,
            manifests.len(),
            passed_rules,
            total_rules
        )
    } else {
        format!(
            "Some hygiene contracts still need attention • {} of {} manifests passing • {} of {} rules passing",
            passed_manifests,
            manifests.len(),
            passed_rules,
            total_rules
        )
    };

    Ok(format!(
        r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\">
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <title>idud hygiene dashboard</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #050816;
      --panel: rgba(16, 24, 48, 0.95);
      --panel-2: rgba(255, 255, 255, 0.04);
      --text: #f7fbff;
      --muted: #95a5c6;
      --accent: #63d2ff;
      --accent-2: #7f6dff;
      --pass: #1fbf7d;
      --fail: #ff6b71;
      --border: rgba(255, 255, 255, 0.13);
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif;
      background: radial-gradient(circle at top left, rgba(99, 210, 255, 0.2), transparent 30%), var(--bg);
      color: var(--text);
      min-height: 100vh;
    }}
    .shell {{ max-width: 1400px; margin: 0 auto; padding: 32px 24px 56px; }}
    .hero {{
      display: grid;
      grid-template-columns: 1.6fr 1fr;
      gap: 24px;
      align-items: stretch;
      margin-bottom: 24px;
    }}
    .hero-card, .summary-card, .manifest-card {{
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 24px;
      box-shadow: 0 24px 60px rgba(0, 0, 0, 0.26);
      backdrop-filter: blur(18px);
    }}
    .hero-card {{ padding: 28px; }}
    .eyebrow {{ text-transform: uppercase; letter-spacing: 0.24em; color: var(--accent); font-size: 0.75rem; font-weight: 700; margin: 0 0 8px; }}
    h1 {{ font-size: clamp(1.8rem, 3.2vw, 2.6rem); margin: 0 0 10px; }}
    .lead {{ color: var(--muted); max-width: 720px; line-height: 1.6; margin: 0; }}
    .summary-card {{ padding: 24px; display: grid; gap: 14px; align-content: start; }}
    .stats {{ display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }}
    .metric {{ background: var(--panel-2); border-radius: 16px; padding: 14px; }}
    .metric strong {{ display: block; font-size: 1.4rem; margin-bottom: 6px; }}
    .metric span {{ color: var(--muted); font-size: 0.85rem; }}
    .manifest-card {{ padding: 24px; margin-bottom: 20px; }}
    .manifest-head {{ display: flex; justify-content: space-between; align-items: start; gap: 16px; margin-bottom: 10px; }}
    .manifest-head h3 {{ margin: 0 0 6px; font-size: 1.2rem; }}
    .description {{ color: var(--muted); margin: 0; line-height: 1.6; }}
    .path {{ color: var(--muted); font-size: 0.9rem; margin: 0 0 16px; word-break: break-all; }}
    .pill {{ display: inline-flex; align-items: center; gap: 6px; padding: 7px 12px; border-radius: 999px; font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; }}
    .pill.pass {{ background: rgba(31, 191, 125, 0.18); color: var(--pass); }}
    .pill.fail {{ background: rgba(255, 107, 113, 0.18); color: var(--fail); }}
    .rules-grid {{ display: grid; gap: 14px; }}
    .rule-card {{ background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 16px; padding: 16px; }}
    .rule-head {{ display: flex; justify-content: space-between; gap: 12px; align-items: center; margin-bottom: 8px; }}
    .rule-head h4 {{ margin: 0; font-size: 0.98rem; }}
    .rule-body {{ display: grid; gap: 12px; }}
    .rule-details {{ border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 14px; padding: 12px 14px; background: rgba(255, 255, 255, 0.03); }}
    .rule-details summary {{ cursor: pointer; font-weight: 600; color: var(--text); list-style: none; display: flex; justify-content: space-between; align-items: center; }}
    .rule-details summary::-webkit-details-marker {{ display: none; }}
    .rule-details[open] summary {{ margin-bottom: 10px; }}
    .rule-detail-body {{ display: grid; gap: 12px; grid-template-columns: minmax(0, 1.2fr) minmax(220px, 0.8fr); }}
    .detail-copy {{ display: grid; gap: 8px; color: var(--muted); line-height: 1.6; }}
    .detail-title {{ color: var(--text); font-weight: 700; margin: 0; }}
    .detail-copy p {{ margin: 0; }}
    .detail-bullets {{ margin: 0; padding-left: 18px; display: grid; gap: 6px; }}
    .detail-context {{ padding: 10px 12px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); }}
    .detail-visual {{ background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 14px; padding: 14px; min-height: 100px; display: flex; align-items: center; justify-content: center; }}
    .visual-stack {{ display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }}
    .visual-block {{ padding: 10px 12px; border-radius: 12px; background: rgba(99, 210, 255, 0.14); border: 1px solid rgba(99, 210, 255, 0.24); color: var(--text); font-weight: 600; text-align: center; min-width: 120px; }}
    .visual-block.secondary {{ background: rgba(127, 109, 255, 0.16); border-color: rgba(127, 109, 255, 0.28); }}
    .arrow {{ color: var(--accent); font-size: 1.1rem; font-weight: 700; }}
    .metric-visual {{ display: grid; gap: 8px; width: 100%; }}
    .metric-track {{ height: 10px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }}
    .metric-fill {{ height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--accent), var(--accent-2)); }}
    .metric-caption {{ color: var(--muted); font-size: 0.95rem; }}
    code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace; font-size: 0.86rem; color: var(--accent); }}
    .no-violations {{ color: var(--pass); margin: 0; font-weight: 600; }}
    .violations {{ margin: 0; padding-left: 18px; color: var(--muted); display: grid; gap: 6px; }}
    .violations li {{ line-height: 1.5; }}
    @media (max-width: 860px) {{
      .hero {{ grid-template-columns: 1fr; }}
      .manifest-head {{ flex-direction: column; }}
      .stats {{ grid-template-columns: 1fr; }}
      .rule-detail-body {{ grid-template-columns: 1fr; }}
    }}
  </style>
</head>
<body>
  <div class=\"shell\">
    <section class=\"hero\">
      <div class=\"hero-card\">
        <p class=\"eyebrow\">Local-first architecture hygiene</p>
        <h1>idud hygiene dashboard</h1>
        <p class=\"lead\">{}</p>
      </div>
      <aside class=\"summary-card\">
        <div class=\"stats\">
          <div class=\"metric\"><strong>{}</strong><span>manifests</span></div>
          <div class=\"metric\"><strong>{}</strong><span>rules</span></div>
          <div class=\"metric\"><strong>{}</strong><span>violations</span></div>
        </div>
        <span class=\"pill {}\">{}</span>
      </aside>
    </section>
    <main>
      {}
    </main>
  </div>
</body>
</html>
"#,
        escape_html(&summary_label),
        manifests.len(),
        total_rules,
        failed_rules,
        if failed_rules == 0 { "pass" } else { "fail" },
        if failed_rules == 0 {
            "All green"
        } else {
            "Needs attention"
        },
        cards
    ))
}

fn resolve_manifest_files(repo_root: &Path, manifest_path: &Path) -> Result<Vec<PathBuf>> {
    let resolved = resolve_path(repo_root, manifest_path)?;
    if resolved.is_dir() {
        let mut files = fs::read_dir(&resolved)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
            })
            .filter(|path| {
                path.file_name().and_then(|name| name.to_str()) != Some("pattern_registry.json")
            })
            .collect::<Vec<_>>();
        files.sort();
        if files.is_empty() {
            return Err(anyhow::anyhow!(
                "no manifest JSON files found in {}",
                resolved.display()
            ));
        }
        Ok(files)
    } else if resolved.is_file() {
        Ok(vec![resolved])
    } else {
        Err(anyhow::anyhow!(
            "manifest path {} does not exist",
            resolved.display()
        ))
    }
}

fn resolve_path(repo_root: &Path, input: &Path) -> Result<PathBuf> {
    if input.is_absolute() {
        return Ok(input.to_path_buf());
    }

    let repo_candidate = repo_root.join(input);
    if repo_candidate.exists() {
        return Ok(repo_candidate);
    }

    let cwd = std::env::current_dir().context("failed to resolve current working directory")?;
    let cwd_candidate = cwd.join(input);
    if cwd_candidate.exists() {
        return Ok(cwd_candidate);
    }

    Ok(repo_candidate)
}

fn escape_html(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn collect_rule_violations(
    repo_root: &Path,
    rule: &Rule,
    files: &[PathBuf],
) -> Result<Vec<String>> {
    let mut violations = Vec::new();

    match rule {
        Rule::ForbidPattern { id, pattern, .. } => {
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
        }
        Rule::RequirePattern { id, pattern, .. } => {
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
        }
        Rule::MaxFileLines { id, max_lines, .. } => {
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
        }
        Rule::MaxParameters {
            id, max_parameters, ..
        } => {
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
        }
        Rule::MaxNestingDepth { id, max_depth, .. } => {
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
        }
        Rule::RequireDependency { id, pattern, .. } => {
            for file in files {
                let content = fs::read_to_string(file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let sanitized = strip_test_modules(&content);
                let regex = Regex::new(&regex::escape(pattern)).with_context(|| {
                    format!("invalid dependency pattern for rule {id}: {pattern}")
                })?;
                if !regex.is_match(&sanitized) {
                    violations.push(format!(
                        "[{id}] missing dependency `{pattern}` in {}",
                        relative_path(repo_root, file)
                    ));
                }
            }
        }
        Rule::ForbidDependency { id, pattern, .. } => {
            for file in files {
                let content = fs::read_to_string(file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let sanitized = strip_test_modules(&content);
                let regex = Regex::new(&regex::escape(pattern)).with_context(|| {
                    format!("invalid dependency pattern for rule {id}: {pattern}")
                })?;
                if regex.is_match(&sanitized) {
                    violations.push(format!(
                        "[{id}] forbidden dependency `{pattern}` matched {}",
                        relative_path(repo_root, file)
                    ));
                }
            }
        }
        Rule::RequireNaming {
            id,
            target,
            pattern,
            ..
        } => {
            let regex = Regex::new(pattern)
                .with_context(|| format!("invalid naming pattern for rule {id}: {pattern}"))?;
            for file in files {
                let content = fs::read_to_string(file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let sanitized = strip_test_modules(&content);
                let names = collect_named_entities(&sanitized, target.as_str());
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
        }
        Rule::RequireCallGraph {
            id, nodes, edges, ..
        } => {
            let node_ids: std::collections::HashSet<&str> =
                nodes.iter().map(|node| node.id.as_str()).collect();
            for edge in edges {
                if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
                    return Err(anyhow::anyhow!(
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
                let mut matched_nodes = std::collections::HashMap::new();
                for node in nodes {
                    let regex = Regex::new(&node.pattern).with_context(|| {
                        format!("invalid regex for call-graph node {}", node.id)
                    })?;
                    let matches: Vec<&FunctionDefinition> = functions
                        .iter()
                        .filter(|function| regex.is_match(&function.name))
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
        }
    }

    Ok(violations)
}

#[derive(Debug)]
struct FunctionDefinition {
    name: String,
    body: String,
}

impl FunctionDefinition {
    fn body_contains_call(&self, target_name: &str) -> bool {
        self.body.contains(&format!("{target_name}("))
    }
}

fn collect_functions(content: &str) -> Vec<FunctionDefinition> {
    let fn_re = Regex::new(
        r"(?m)^\s*(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)\s*(<[^>]+>)?\s*\([^;]*\)\s*(->\s*[^\{]+)?\s*\{",
    )
    .unwrap_or_else(|_| Regex::new(r"^$" ).unwrap());
    let mut functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if let Some(captures) = fn_re.captures(line) {
            let name = captures
                .get(3)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let mut body_lines = vec![line.to_string()];
            let mut brace_depth = count_braces(line);
            let mut next_index = index + 1;
            while next_index < lines.len() && brace_depth > 0 {
                body_lines.push(lines[next_index].to_string());
                brace_depth += count_braces(lines[next_index]);
                next_index += 1;
            }

            functions.push(FunctionDefinition {
                name: name.clone(),
                body: body_lines.join("\n"),
            });
            index = next_index;
            continue;
        }

        index += 1;
    }

    functions
}

fn collect_named_entities(content: &str, target: &str) -> Vec<String> {
    let regex = match target {
        "function" => Regex::new(r"(?m)^\s*(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)\b")
            .unwrap_or_else(|_| Regex::new(r"^$").unwrap()),
        "type" => Regex::new(r"(?m)^\s*(pub\s+)?(struct|enum|trait)\s+([A-Za-z0-9_]+)\b")
            .unwrap_or_else(|_| Regex::new(r"^$").unwrap()),
        "module" => Regex::new(r"(?m)^\s*(pub\s+)?mod\s+([A-Za-z0-9_]+)\b")
            .unwrap_or_else(|_| Regex::new(r"^$").unwrap()),
        _ => Regex::new(r"^$").unwrap(),
    };

    let mut names = Vec::new();
    for captures in regex.captures_iter(content) {
        let name = match target {
            "function" => captures.get(3),
            "type" => captures.get(3),
            "module" => captures.get(2),
            _ => None,
        };

        if let Some(name) = name {
            names.push(name.as_str().to_string());
        }
    }

    names
}

fn count_braces(text: &str) -> i32 {
    text.chars().fold(0, |acc, ch| match ch {
        '{' => acc + 1,
        '}' => acc - 1,
        _ => acc,
    })
}

fn discover_files(repo_root: &Path, include_patterns: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in WalkDir::new(repo_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if should_skip(path) {
            continue;
        }

        let relative = relative_path(repo_root, path);
        if include_patterns
            .iter()
            .any(|pattern| matches_glob(&relative, pattern))
        {
            files.push(path.to_path_buf());
        }
    }

    files.sort();
    files
}

fn should_skip(path: &Path) -> bool {
    let display = path.to_string_lossy();
    display.contains("/target/")
        || display.contains("\\target\\")
        || display.contains("/.git/")
        || display.contains("\\.git\\")
}

fn relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn matches_glob(path: &str, pattern: &str) -> bool {
    glob_to_regex(pattern)
        .map(|regex| regex.is_match(path))
        .unwrap_or(false)
}

fn glob_to_regex(pattern: &str) -> Result<Regex> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if chars.peek() == Some(&'/') {
                        chars.next();
                        regex.push_str("(?:[^/]+/)*");
                    } else {
                        regex.push_str(".*");
                    }
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }

    regex.push('$');
    Regex::new(&regex).context("failed to compile glob pattern")
}

fn strip_test_modules(content: &str) -> String {
    let mut output = String::new();
    let mut skip_depth = 0;
    let mut pending_cfg = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if skip_depth > 0 {
            let opens = line.chars().filter(|ch| *ch == '{').count();
            let closes = line.chars().filter(|ch| *ch == '}').count();
            skip_depth += opens as i32 - closes as i32;
            continue;
        }

        if pending_cfg {
            pending_cfg = false;
            if trimmed.starts_with("mod tests") {
                let opens = trimmed.chars().filter(|ch| *ch == '{').count();
                let closes = trimmed.chars().filter(|ch| *ch == '}').count();
                skip_depth = opens as i32 - closes as i32;
                if skip_depth <= 0 {
                    output.push_str(line);
                    output.push('\n');
                }
                continue;
            }
        }

        if trimmed.starts_with("#[cfg(test)]")
            || trimmed.starts_with("#[cfg(any")
            || trimmed.starts_with("#[cfg(all")
        {
            pending_cfg = true;
            continue;
        }

        if trimmed.starts_with("mod tests") {
            let opens = trimmed.chars().filter(|ch| *ch == '{').count();
            let closes = trimmed.chars().filter(|ch| *ch == '}').count();
            skip_depth = opens as i32 - closes as i32;
            if skip_depth <= 0 {
                output.push_str(line);
                output.push('\n');
            }
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}

fn count_parameter_violations(content: &str, max_parameters: usize) -> Vec<String> {
    let mut violations = Vec::new();
    let fn_re =
        Regex::new(r"(?m)^\s*(pub\s+)?(async\s+)?fn\s+[A-Za-z0-9_]+\s*(<[^>]+>)?\s*\(").unwrap();
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if !fn_re.is_match(line) {
            continue;
        }

        let mut signature = line.to_string();
        let mut current = idx;
        while !signature.contains(')') && current + 1 < lines.len() {
            current += 1;
            signature.push(' ');
            signature.push_str(lines[current]);
        }

        let parameter_count = count_parameters(&signature);
        if parameter_count > max_parameters {
            violations.push(format!(
                "function has {parameter_count} parameters (max {max_parameters})"
            ));
        }
    }

    violations
}

fn count_nesting_depth(content: &str) -> usize {
    let mut depth = 0i32;
    let mut max_depth = 0usize;
    for line in content.lines() {
        let opens = line.chars().filter(|ch| *ch == '{').count();
        let closes = line.chars().filter(|ch| *ch == '}').count();
        depth += opens as i32;
        max_depth = std::cmp::max(max_depth, depth as usize);
        depth -= closes as i32;
    }
    max_depth
}

fn count_parameters(signature: &str) -> usize {
    let Some(start) = signature.find('(') else {
        return 0;
    };
    let Some(end) = signature.find(')') else {
        return 0;
    };
    let params = &signature[start + 1..end];

    params
        .split(',')
        .filter(|param| {
            let trimmed = param.trim();
            !trimmed.is_empty() && !matches!(trimmed, "self" | "&self" | "&mut self" | "mut self")
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn enforces_controller_service_call_graph() {
        let temp_dir = std::env::temp_dir().join(format!(
            "idud-hygiene-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(temp_dir.join("src")).unwrap();
        fs::write(
            temp_dir.join("src/app.rs"),
            r#"
            pub fn handle_create() {
                service::create_record();
            }

            pub mod service {
                pub fn create_record() {}
            }
            "#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("pattern.json"),
            r#"{
                "name": "sample",
                "description": "sample",
                "rules": [
                    {
                        "id": "controller-calls-service",
                        "kind": "require-call-graph",
                        "include": ["src/**/*.rs"],
                        "nodes": [
                            { "id": "controller", "pattern": "handle_create" },
                            { "id": "service", "pattern": "create_record" }
                        ],
                        "edges": [{ "from": "controller", "to": "service" }]
                    }
                ]
            }"#,
        )
        .unwrap();

        let violations = enforce_golden_pattern(&temp_dir, temp_dir.join("pattern.json")).unwrap();

        assert!(violations.is_empty(), "violations: {:?}", violations);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn enforces_dependency_and_naming_rules() {
        let temp_dir = std::env::temp_dir().join(format!(
            "idud-hygiene-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(temp_dir.join("src")).unwrap();
        fs::write(
            temp_dir.join("src/app.rs"),
            r#"
            use crate::types::Thing;
            use std::process::Command;

            pub fn good_name() {}
            pub fn badName() {}
            "#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("pattern.json"),
            r#"{
                "name": "sample",
                "description": "sample",
                "rules": [
                    {
                        "id": "require-types-import",
                        "kind": "require-dependency",
                        "pattern": "crate::types",
                        "include": ["src/**/*.rs"]
                    },
                    {
                        "id": "forbid-std-process",
                        "kind": "forbid-dependency",
                        "pattern": "std::process::Command",
                        "include": ["src/**/*.rs"]
                    },
                    {
                        "id": "functions-use-snake-case",
                        "kind": "require-naming",
                        "target": "function",
                        "pattern": "^[a-z][a-z0-9_]*$",
                        "include": ["src/**/*.rs"]
                    }
                ]
            }"#,
        )
        .unwrap();

        let violations = enforce_golden_pattern(&temp_dir, temp_dir.join("pattern.json")).unwrap();

        assert!(
            violations
                .iter()
                .all(|violation| !violation.contains("missing dependency")),
            "unexpected missing dependency violation, got: {:?}",
            violations
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("forbidden dependency")),
            "expected forbidden dependency violation, got: {:?}",
            violations
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("badName")),
            "expected naming violation, got: {:?}",
            violations
        );

        let _ = fs::remove_dir_all(temp_dir);
    }
}
