//! Repository hygiene tests.
//!
//! These tests enforce that the repository stays lean and free of stray artifacts
//! in the root directory so the codebase remains easy to navigate. They also enforce
//! a configurable golden-pattern contract for the architecture-critical modules.

use idud_hygiene::enforce_golden_pattern;
use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn repo_root_stays_clean() {
    let allowed_entries: BTreeSet<&str> = [
        ".env.example",
        ".git",
        ".github",
        ".githooks",
        ".gitignore",
        "Cargo.lock",
        "Cargo.toml",
        "LICENSE",
        "Makefile",
        "README.md",
        "CONTRIBUTING.md",
        "SETUP.md",
        "crates",
        "data",
        "docs",
        "package.json",
        "scripts",
        "src",
        "target",
        "tests",
        "training",
        "ui",
    ]
    .into_iter()
    .collect();

    let root = Path::new(".");
    let mut unexpected = Vec::new();

    for entry in fs::read_dir(root).expect("root directory should be readable") {
        let entry = entry.expect("entry should be readable");
        let name = entry.file_name().to_string_lossy().into_owned();
        if !allowed_entries.contains(name.as_str()) {
            unexpected.push(name);
        }
    }

    assert!(
        unexpected.is_empty(),
        "Repo root contains unexpected entries: {:?}",
        unexpected
    );

    assert!(
        !Path::new("idud.db").exists(),
        "Remove local database artifacts from the repository root"
    );
    assert!(
        Path::new("docs/reference").exists(),
        "Reference documentation should live under docs/reference"
    );
}

#[test]
fn golden_architecture_pattern_is_enforced() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifests = [
        manifest_dir.join("crates/idud-hygiene/golden_patterns/architecture_hygiene.json"),
        manifest_dir.join("crates/idud-hygiene/golden_patterns/layered_architecture_hygiene.json"),
        manifest_dir.join("crates/idud-hygiene/golden_patterns/pattern_catalog_hygiene.json"),
    ];

    let mut all_violations = Vec::new();
    for pattern_path in manifests {
        let violations =
            enforce_golden_pattern(manifest_dir, &pattern_path).unwrap_or_else(|err| {
                panic!(
                    "failed to enforce hygiene manifest {:?}: {err}",
                    pattern_path
                )
            });
        if !violations.is_empty() {
            all_violations.extend(violations);
        }
    }

    assert!(
        all_violations.is_empty(),
        "Golden architecture pattern violations:\n{}",
        all_violations.join("\n")
    );
}
