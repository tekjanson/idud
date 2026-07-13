use super::{collector::SemanticToken, pattern_parsing::strip_generics};

pub(crate) fn matches_call_pattern(token: &SemanticToken, path: &str, args: &[&str]) -> bool {
    match token {
        SemanticToken::Call {
            path: token_path,
            args: token_args,
        } => {
            let expected = path
                .split("::")
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>();
            if token_path.len() != expected.len() {
                return false;
            }
            if !expected
                .iter()
                .enumerate()
                .all(|(idx, expected_segment)| token_path[idx] == *expected_segment)
            {
                return false;
            }
            if args.is_empty() {
                return true;
            }
            args.iter()
                .all(|arg| token_args.iter().any(|token_arg| token_arg == *arg))
        }
        _ => false,
    }
}

pub(crate) fn matches_return_type_pattern(token: &SemanticToken, pattern: &str) -> bool {
    let expected = strip_generics(pattern.trim_start_matches("->").trim());
    if expected.is_empty() {
        return false;
    }
    match token {
        SemanticToken::Path(segments) => segments.iter().any(|segment| segment == expected),
        _ => false,
    }
}

pub(crate) fn matches_path_pattern(token: &SemanticToken, pattern: &str) -> bool {
    let pattern = strip_generics(pattern);
    let expected = pattern
        .split("::")
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if expected.is_empty() {
        return false;
    }
    match token {
        SemanticToken::Path(segments) => {
            if segments.len() < expected.len() {
                return false;
            }
            let start = segments.len() - expected.len();
            expected
                .iter()
                .enumerate()
                .all(|(idx, expected_segment)| segments[start + idx] == *expected_segment)
        }
        _ => false,
    }
}

pub(crate) fn matches_attribute_pattern(token: &SemanticToken, pattern: &str) -> bool {
    let inner = pattern
        .strip_prefix("#![")
        .or_else(|| pattern.strip_prefix("#["))
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(pattern);
    let (path, args) = inner.split_once('(').unwrap_or((inner, ""));
    let args = args.strip_suffix(')').unwrap_or(args);

    match token {
        SemanticToken::Attribute {
            path: token_path,
            args: token_args,
        } => {
            let path_matches = token_path.iter().any(|segment| segment == path);
            let arg_matches = if args.is_empty() {
                true
            } else {
                token_args.iter().any(|token_arg| token_arg == args)
            };
            path_matches && arg_matches
        }
        _ => false,
    }
}

pub(crate) fn match_identifiers(token: &SemanticToken, pattern: &str) -> bool {
    let trimmed = pattern.trim();
    let candidates = if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed[1..trimmed.len() - 1]
            .split('|')
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty())
            .collect::<Vec<_>>()
    } else {
        vec![trimmed]
    };

    match token {
        SemanticToken::Macro(name) => candidates.iter().any(|candidate| {
            let candidate = candidate
                .trim_end_matches('!')
                .trim_end_matches('(')
                .trim_end_matches(')');
            let name = name.trim_end_matches('!');
            name == candidate
        }),
        SemanticToken::FunctionCall(name) => candidates.iter().any(|candidate| {
            let candidate = candidate
                .trim_end_matches('!')
                .trim_end_matches('(')
                .trim_end_matches(')');
            let name = name.trim_end_matches('!');
            name == candidate
        }),
        SemanticToken::MethodCall { method, .. } => candidates.iter().any(|candidate| {
            let candidate = candidate
                .trim_end_matches('!')
                .trim_end_matches('(')
                .trim_end_matches(')');
            let method = method.trim_end_matches('!');
            method == candidate
        }),
        SemanticToken::Keyword(name) => candidates.iter().any(|candidate| {
            let candidate = candidate
                .trim_end_matches('!')
                .trim_end_matches('(')
                .trim_end_matches(')');
            let name = name.trim_end_matches('!');
            name == candidate
        }),
        SemanticToken::Identifier(name) => candidates.iter().any(|candidate| {
            let candidate = candidate
                .trim_end_matches('!')
                .trim_end_matches('(')
                .trim_end_matches(')');
            name == candidate
        }),
        SemanticToken::Module(name) => candidates.iter().any(|candidate| {
            let candidate = candidate.trim_start_matches("mod ").trim();
            name == candidate
        }),
        SemanticToken::Path(segments) => {
            let last = segments.last().map(String::as_str);
            candidates.iter().any(|candidate| last == Some(*candidate))
        }
        SemanticToken::Attribute { .. } | SemanticToken::Call { .. } => false,
    }
}
