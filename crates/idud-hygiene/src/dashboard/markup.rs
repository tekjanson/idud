use crate::manifest::Rule;

use super::rule_renderers::{
    render_call_graph, render_dependency, render_forbid_pattern, render_max_file_lines,
    render_max_nesting_depth, render_max_parameters, render_naming, render_require_pattern,
};

pub fn build_rule_details_markup(
    rule: &Rule,
    manifest_name: &str,
    manifest_description: &str,
) -> String {
    let documentation = rule.documentation().cloned().unwrap_or_default();
    let mut markup = match rule {
        Rule::ForbidPattern {
            pattern, include, ..
        } => render_forbid_pattern(&pattern.as_vec().join(", "), &include.as_vec().join(", ")),
        Rule::RequirePattern {
            pattern, include, ..
        } => render_require_pattern(&pattern.as_vec().join(", "), &include.as_vec().join(", ")),
        Rule::MaxFileLines {
            max_lines, include, ..
        } => render_max_file_lines(*max_lines, &include.as_vec().join(", ")),
        Rule::MaxParameters {
            max_parameters,
            include,
            ..
        } => render_max_parameters(*max_parameters, &include.as_vec().join(", ")),
        Rule::MaxNestingDepth {
            max_depth, include, ..
        } => render_max_nesting_depth(*max_depth, &include.as_vec().join(", ")),
        Rule::RequireCallGraph {
            include,
            nodes,
            edges,
            ..
        } => render_call_graph(&include.as_vec().join(", "), nodes, edges),
        Rule::RequireDependency {
            pattern, include, ..
        } => render_dependency(pattern, &include.as_vec().join(", "), true),
        Rule::ForbidDependency {
            pattern, include, ..
        } => render_dependency(pattern, &include.as_vec().join(", "), false),
        Rule::RequireNaming {
            target,
            pattern,
            include,
            ..
        } => render_naming(target, pattern, &include.as_vec().join(", ")),
    };

    if let Some(summary) = documentation.summary.as_deref() {
        if !summary.trim().is_empty() {
            markup.intro = summary.to_string();
        }
    }
    if let Some(explanation) = documentation.explanation.as_deref() {
        if !explanation.trim().is_empty() {
            markup.bullets.push(format!("<li>{}</li>", escape_html(explanation)));
        }
    }
    if let Some(visual) = documentation.visual.as_deref() {
        if !visual.trim().is_empty() {
            markup.visual = visual.to_string();
        }
    }

    let context_markup = format!(
        "<div class=\"detail-context\"><p class=\"detail-title\">Manifest context</p><p>{}</p></div>",
        escape_html(manifest_description)
    );
    format!(
        "<div class=\"detail-copy\">\n  <p class=\"detail-title\">{}</p>\n  <p>{}</p>\n  <ul class=\"detail-bullets\">{}</ul>\n  {context_markup}\n</div>\n<div class=\"detail-visual\">{}</div>",
        escape_html(manifest_name),
        markup.intro,
        markup.bullets.join(""),
        markup.visual
    )
}

pub fn escape_html(value: impl AsRef<str>) -> String {
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
