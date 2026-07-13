use regex::Regex;

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub body: String,
}

impl FunctionDefinition {
    pub fn body_contains_call(&self, target_name: &str) -> bool {
        self.body.contains(&format!("{target_name}("))
    }
}

pub fn strip_test_modules(content: &str) -> String {
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

pub fn count_parameter_violations(content: &str, max_parameters: usize) -> Vec<String> {
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

pub fn count_nesting_depth(content: &str) -> usize {
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

pub fn collect_functions(content: &str) -> Vec<FunctionDefinition> {
    let fn_re = Regex::new(
        r"(?m)^\s*(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)\s*(<[^>]+>)?\s*\([^;]*\)\s*(->\s*[^\{]+)?\s*\{",
    )
    .unwrap_or_else(|_| Regex::new(r"^$").unwrap());
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

pub fn collect_named_entities(content: &str, target: &str) -> Vec<String> {
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
            "function" | "type" => captures.get(3),
            "module" => captures.get(2),
            _ => None,
        };
        if let Some(name) = name {
            names.push(name.as_str().to_string());
        }
    }
    names
}

pub fn count_braces(text: &str) -> i32 {
    text.chars().fold(0, |acc, ch| match ch {
        '{' => acc + 1,
        '}' => acc - 1,
        _ => acc,
    })
}

pub fn count_parameters(signature: &str) -> usize {
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
