mod analysis;
mod dashboard;
mod manifest;

pub use analysis::{
    enforce_golden_manifests, enforce_golden_pattern, report_golden_manifests,
    report_golden_pattern, ManifestReport, RuleReport,
};
pub use dashboard::render_hygiene_dashboard;
pub use manifest::{
    load_golden_pattern, CallGraphEdge, CallGraphNode, GoldenPattern, IncludeSpec, PatternSpec,
    Rule, RuleDocumentation,
};

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
