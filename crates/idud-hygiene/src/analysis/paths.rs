use anyhow::{Context, Result};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn resolve_manifest_files(repo_root: &Path, manifest_path: &Path) -> Result<Vec<PathBuf>> {
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

pub fn resolve_path(repo_root: &Path, input: &Path) -> Result<PathBuf> {
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

pub fn discover_files(repo_root: &Path, include_patterns: &[String]) -> Vec<PathBuf> {
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

pub fn relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn should_skip(path: &Path) -> bool {
    let display = path.to_string_lossy();
    display.contains("/target/")
        || display.contains("\\target\\")
        || display.contains("/.git/")
        || display.contains("\\.git\\")
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
