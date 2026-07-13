use anyhow::Result;
use std::{fmt::Write, path::Path};

use crate::analysis::{
    paths::{relative_path, resolve_manifest_files},
    RuleReport,
};
use crate::manifest::Rule;

use super::markup::{build_rule_details_markup, escape_html};

struct ManifestView {
    name: String,
    description: String,
    path: String,
    passed: bool,
    passed_rules: usize,
    rule_reports: Vec<RuleReport>,
    rule_specs: Vec<Rule>,
}

pub fn render_hygiene_dashboard(
    repo_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<String> {
    let repo_root = repo_root.as_ref();
    let manifest_files = resolve_manifest_files(repo_root, manifest_path.as_ref())?;
    let mut manifests = Vec::new();
    for manifest_file in manifest_files {
        let pattern = crate::manifest::load_golden_pattern(&manifest_file)?;
        let rule_reports = crate::analysis::report_golden_pattern(repo_root, &manifest_file)?;
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
    let cards = manifests
        .iter()
        .map(build_manifest_card)
        .collect::<Vec<_>>()
        .join("");
    let summary_label = if failed_rules == 0 {
        format!("All hygiene contracts are satisfied • {} of {} manifests passing • {} of {} rules passing", passed_manifests, manifests.len(), passed_rules, total_rules)
    } else {
        format!("Some hygiene contracts still need attention • {} of {} manifests passing • {} of {} rules passing", passed_manifests, manifests.len(), passed_rules, total_rules)
    };
    Ok(include_str!("../dashboard_template.html")
        .replace("__SUMMARY_LABEL__", &summary_label)
        .replace("__MANIFESTS_COUNT__", &manifests.len().to_string())
        .replace("__RULES_COUNT__", &total_rules.to_string())
        .replace("__VIOLATIONS_COUNT__", &failed_rules.to_string())
        .replace(
            "__STATE_CLASS__",
            if failed_rules == 0 { "pass" } else { "fail" },
        )
        .replace(
            "__STATE_LABEL__",
            if failed_rules == 0 {
                "All green"
            } else {
                "Needs attention"
            },
        )
        .replace("__CARDS__", &cards))
}

fn build_manifest_card(manifest: &ManifestView) -> String {
    let mut rules_markup = String::new();
    for (index, rule) in manifest.rule_specs.iter().enumerate() {
        let report = &manifest.rule_reports[index];
        let status_class = if report.passed { "pass" } else { "fail" };
        let status_label = if report.passed {
            "Passing"
        } else {
            "Needs attention"
        };
        let detail_markup = build_rule_details_markup(rule, &manifest.name, &manifest.description);
        let violations_markup = if report.violations.is_empty() {
            "<p class=\"no-violations\">No violations detected.</p>".to_string()
        } else {
            let mut items = String::new();
            for violation in &report.violations {
                writeln!(items, "<li>{}</li>", escape_html(violation)).unwrap();
            }
            format!("<ul class=\"violations\">{items}</ul>")
        };
        writeln!(rules_markup, "<article class=\"rule-card {status_class}\">\n  <div class=\"rule-head\">\n    <h4>{}</h4>\n    <span class=\"pill {status_class}\">{}</span>\n  </div>\n  <div class=\"rule-body\">\n    <details class=\"rule-details\">\n      <summary>What this rule means</summary>\n      <div class=\"rule-detail-body\">{}</div>\n    </details>\n    <div class=\"rule-evidence\">{}</div>\n  </div>\n</article>", escape_html(&report.id), escape_html(status_label), detail_markup, violations_markup).unwrap();
    }
    let manifest_status = if manifest.passed { "PASS" } else { "FAIL" };
    let manifest_status_class = if manifest.passed { "pass" } else { "fail" };
    format!("<section class=\"manifest-card\">\n  <div class=\"manifest-head\">\n    <div>\n      <p class=\"eyebrow\">Manifest</p>\n      <h3>{}</h3>\n      <p class=\"description\">{}</p>\n    </div>\n    <div class=\"manifest-status\">\n      <span class=\"pill {manifest_status_class}\">{}</span>\n      <p>{}/{} rules passing</p>\n    </div>\n  </div>\n  <p class=\"path\">{}</p>\n  <div class=\"rules-grid\">{}</div>\n</section>", escape_html(&manifest.name), escape_html(&manifest.description), escape_html(manifest_status), manifest.passed_rules, manifest.rule_reports.len(), escape_html(&manifest.path), rules_markup)
}
