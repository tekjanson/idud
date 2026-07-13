use super::markup::escape_html;
use crate::manifest::{CallGraphEdge, CallGraphNode};

pub struct RuleMarkup {
    pub intro: String,
    pub bullets: Vec<String>,
    pub visual: String,
}

pub fn render_forbid_pattern(patterns: &str, includes: &str) -> RuleMarkup {
    RuleMarkup {
        intro: format!(
            "This rule forbids the pattern <code>{}</code> so it never slips into the targeted files.",
            escape_html(patterns)
        ),
        bullets: vec![format!("<li>Applies to: <code>{}</code></li><li>Useful for banning unsafe APIs, secrets, or disallowed imports.</li>", escape_html(includes))],
        visual: format!("<div class=\"visual-stack\"><div class=\"visual-block\">Target files<br/><code>{}</code></div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">Forbidden match<br/><code>{}</code></div></div>", escape_html(includes), escape_html(patterns)),
    }
}

pub fn render_require_pattern(patterns: &str, includes: &str) -> RuleMarkup {
    RuleMarkup {
        intro: format!(
            "This rule requires the pattern <code>{}</code> to appear in the targeted files.",
            escape_html(patterns)
        ),
        bullets: vec![format!("<li>Applies to: <code>{}</code></li><li>Use it when a standard entry point, lifecycle hook, or API call must be present.</li>", escape_html(includes))],
        visual: format!("<div class=\"visual-stack\"><div class=\"visual-block\">Target files<br/><code>{}</code></div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">Required match<br/><code>{}</code></div></div>", escape_html(includes), escape_html(patterns)),
    }
}

pub fn render_max_file_lines(max_lines: usize, includes: &str) -> RuleMarkup {
    RuleMarkup {
        intro: format!("This rule keeps files small by enforcing a hard ceiling of {} lines per file.", max_lines),
        bullets: vec![format!("<li>Applies to: <code>{}</code></li><li>It helps keep modules readable and easier to review.</li>", escape_html(includes))],
        visual: format!("<div class=\"metric-visual\"><div class=\"metric-track\"><div class=\"metric-fill\" style=\"width: 100%;\"></div></div><div class=\"metric-caption\">Budget: <strong>{}</strong> lines</div></div>", max_lines),
    }
}

pub fn render_max_parameters(max_parameters: usize, includes: &str) -> RuleMarkup {
    RuleMarkup {
        intro: format!("This rule keeps functions focused by limiting them to {} parameters.", max_parameters),
        bullets: vec![format!("<li>Applies to: <code>{}</code></li><li>It pushes complex behavior into smaller units instead of overloading one function.</li>", escape_html(includes))],
        visual: format!("<div class=\"metric-visual\"><div class=\"metric-track\"><div class=\"metric-fill\" style=\"width: 100%;\"></div></div><div class=\"metric-caption\">Max parameters: <strong>{}</strong></div></div>", max_parameters),
    }
}

pub fn render_max_nesting_depth(max_depth: usize, includes: &str) -> RuleMarkup {
    RuleMarkup {
        intro: format!("This rule flattens control flow by keeping nesting depth at or below {} levels.", max_depth),
        bullets: vec![format!("<li>Applies to: <code>{}</code></li><li>It reduces cognitive load and makes the logic easier to follow.</li>", escape_html(includes))],
        visual: format!("<div class=\"visual-stack\"><div class=\"visual-block\">Depth 1</div><div class=\"arrow\">⟶</div><div class=\"visual-block\">Depth 2</div><div class=\"arrow\">⟶</div><div class=\"visual-block\">Depth {}</div></div>", max_depth),
    }
}

pub fn render_call_graph(
    includes: &str,
    nodes: &[CallGraphNode],
    edges: &[CallGraphEdge],
) -> RuleMarkup {
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
    RuleMarkup {
        intro: "This rule requires the code to form a specific call-graph shape.".to_string(),
        bullets: vec![format!(
            "<li>Applies to: <code>{}</code></li><li>Nodes: {}</li><li>Edges: {}</li>",
            escape_html(includes),
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
        )],
        visual: format!("<div class=\"visual-stack\">{nodes_markup}</div>"),
    }
}

pub fn render_dependency(pattern: &str, includes: &str, required: bool) -> RuleMarkup {
    let intro = if required {
        format!("This rule requires a dependency on <code>{}</code> so the subsystem keeps its expected relationships.", escape_html(pattern))
    } else {
        format!("This rule forbids the dependency <code>{}</code> so the layer stays decoupled from the wrong neighbor.", escape_html(pattern))
    };
    let bullets = format!(
        "<li>Applies to: <code>{}</code></li><li>{}</li>",
        escape_html(includes),
        if required {
            "It makes architectural boundaries explicit and discoverable."
        } else {
            "It prevents accidental coupling and keeps boundaries crisp."
        }
    );
    let visual = format!("<div class=\"visual-stack\"><div class=\"visual-block\">Current module</div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">{}</div></div>", escape_html(pattern));
    RuleMarkup {
        intro,
        bullets: vec![bullets],
        visual,
    }
}

pub fn render_naming(target: &str, pattern: &str, includes: &str) -> RuleMarkup {
    RuleMarkup {
        intro: format!("This rule expects every {} to match the naming pattern <code>{}</code>.", target, escape_html(pattern)),
        bullets: vec![format!("<li>Applies to: <code>{}</code></li><li>It stabilizes code reading and makes intent predictable across the repo.</li>", escape_html(includes))],
        visual: format!("<div class=\"visual-stack\"><div class=\"visual-block\">{} name</div><div class=\"arrow\">⟶</div><div class=\"visual-block secondary\">{}</div></div>", escape_html(target), escape_html(pattern)),
    }
}
