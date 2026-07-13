use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::{
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

pub fn enforce_golden_pattern(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<String>> {
    let repo_root = repo_root.as_ref();
    let pattern = load_golden_pattern(manifest_path)?;
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
    let pattern = load_golden_pattern(manifest_path)?;
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
                if !node_ids.contains(edge.from.as_str())
                    || !node_ids.contains(edge.to.as_str())
                {
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
        r"(?m)^\s*(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)\s*(<[^>]+>)?\s*\([^;]*\)\s*(->\s*[^ {]+)?\s*\{",
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
