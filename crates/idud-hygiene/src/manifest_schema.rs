use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GoldenPattern {
    pub name: String,
    pub description: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallGraphNode {
    pub id: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallGraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuleDocumentation {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub visual: Option<String>,
    #[serde(default)]
    pub example: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Rule {
    #[serde(rename = "forbid-pattern")]
    ForbidPattern {
        id: String,
        pattern: PatternSpec,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "require-pattern")]
    RequirePattern {
        id: String,
        pattern: PatternSpec,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "max-file-lines")]
    MaxFileLines {
        id: String,
        max_lines: usize,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "max-parameters")]
    MaxParameters {
        id: String,
        max_parameters: usize,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "max-nesting-depth")]
    MaxNestingDepth {
        id: String,
        max_depth: usize,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "require-call-graph")]
    RequireCallGraph {
        id: String,
        include: IncludeSpec,
        nodes: Vec<CallGraphNode>,
        edges: Vec<CallGraphEdge>,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "require-dependency")]
    RequireDependency {
        id: String,
        pattern: String,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "forbid-dependency")]
    ForbidDependency {
        id: String,
        pattern: String,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
    #[serde(rename = "require-naming")]
    RequireNaming {
        id: String,
        target: String,
        pattern: String,
        include: IncludeSpec,
        #[serde(default)]
        documentation: Option<RuleDocumentation>,
    },
}

impl Rule {
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

    pub fn documentation(&self) -> Option<&RuleDocumentation> {
        match self {
            Rule::ForbidPattern { documentation, .. }
            | Rule::RequirePattern { documentation, .. }
            | Rule::MaxFileLines { documentation, .. }
            | Rule::MaxParameters { documentation, .. }
            | Rule::MaxNestingDepth { documentation, .. }
            | Rule::RequireCallGraph { documentation, .. }
            | Rule::RequireDependency { documentation, .. }
            | Rule::ForbidDependency { documentation, .. }
            | Rule::RequireNaming { documentation, .. } => documentation.as_ref(),
        }
    }
}
