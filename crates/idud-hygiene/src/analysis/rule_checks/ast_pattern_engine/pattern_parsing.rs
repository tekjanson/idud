pub(crate) fn normalize_pattern(pattern_text: &str) -> String {
    pattern_text.replace('\\', "")
}

pub(crate) fn split_pattern_alternatives(pattern: &str) -> Vec<&str> {
    if pattern.contains('|') {
        pattern
            .split('|')
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty())
            .collect()
    } else {
        vec![pattern]
    }
}

pub(crate) fn parse_method_pattern(pattern: &str) -> Option<(&str, Vec<&str>)> {
    let pattern = pattern.trim();
    if pattern.contains("::") || !pattern.contains('.') {
        return None;
    }

    let (receiver, rest) = pattern.split_once('.')?;
    let methods = if rest.starts_with('(') && rest.ends_with(')') {
        rest[1..rest.len() - 1]
            .split('|')
            .map(str::trim)
            .filter(|method| !method.is_empty())
            .collect::<Vec<_>>()
    } else {
        vec![rest.trim()]
    };
    Some((receiver.trim(), methods))
}

pub(crate) fn parse_call_pattern(pattern: &str) -> Option<(&str, Vec<&str>)> {
    let pattern = pattern.trim();
    let (path, args) = pattern.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let path = path.trim();
    if !path.contains("::") && args.is_empty() {
        return None;
    }
    let args = if args.is_empty() {
        Vec::new()
    } else {
        args.split(',')
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .collect::<Vec<_>>()
    };
    Some((path, args))
}

pub(crate) fn strip_generics(pattern: &str) -> &str {
    pattern.split('<').next().unwrap_or(pattern).trim()
}
