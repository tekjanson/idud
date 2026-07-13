// src/analysis/extractors.rs
//! Regex-based extractors for different programming languages
//! Extracts imports, function calls, type references, and inheritance patterns

use once_cell::sync::Lazy;
use regex::Regex;

// ============================================================================
// RUST EXTRACTORS
// ============================================================================

pub struct RustExtractor;

impl RustExtractor {
    /// Extract `use` import statements
    pub fn extract_imports(content: &str) -> Vec<(String, f32)> {
        static RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"use\s+([a-zA-Z_][a-zA-Z0-9_:]*(?:::\{[^}]+\})?);").unwrap());

        RE.captures_iter(content)
            .map(|caps| {
                let import = caps[1].to_string();
                (import, 0.95) // explicit import = high confidence
            })
            .collect()
    }

    /// Extract function calls to external crates/modules
    pub fn extract_calls(content: &str) -> Vec<(String, f32)> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?:^|\s)([a-zA-Z_][a-zA-Z0-9_]*)::[a-zA-Z_][a-zA-Z0-9_]*\s*\(").unwrap()
        });

        RE.captures_iter(content)
            .map(|caps| {
                let module = caps[1].to_string();
                (module, 0.70) // inferred call = moderate confidence
            })
            .collect()
    }

    /// Extract trait implementations (type references)
    pub fn extract_trait_refs(content: &str) -> Vec<(String, f32)> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"impl\s+([a-zA-Z_][a-zA-Z0-9_]*(?::\s*[a-zA-Z_][a-zA-Z0-9_]*)?)\s+for")
                .unwrap()
        });

        RE.captures_iter(content)
            .map(|caps| {
                let trait_name = caps[1].to_string();
                (trait_name, 0.85) // explicit trait = high confidence
            })
            .collect()
    }

    /// Extract struct/enum inheritance patterns
    pub fn extract_inherit(content: &str) -> Vec<(String, f32)> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?:struct|enum)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:\([^)]*\))?(?:\{|;)")
                .unwrap()
        });

        RE.captures_iter(content)
            .map(|caps| {
                let name = caps[1].to_string();
                (name, 0.60) // structural reference = moderate confidence
            })
            .collect()
    }
}

// ============================================================================
// TYPESCRIPT/JAVASCRIPT EXTRACTORS
// ============================================================================

pub struct TypeScriptExtractor;

impl TypeScriptExtractor {
    /// Extract import/require statements
    pub fn extract_imports(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // ES6 import statements - flexible pattern to catch all import forms
        static ES6_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"import\s+[^;]*?from\s+["']([^"']+)["']"#).unwrap());

        for caps in ES6_RE.captures_iter(content) {
            let module = caps[1].to_string();
            deps.push((module, 0.95));
        }

        // CommonJS require statements
        static CJS_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"require\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap());

        for caps in CJS_RE.captures_iter(content) {
            let module = caps[1].to_string();
            deps.push((module, 0.95));
        }

        deps
    }

    /// Extract function/class calls and instantiations
    pub fn extract_calls(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // Constructor calls: new ClassName(...)
        static CONSTR_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"new\s+([a-zA-Z_$][a-zA-Z0-9_$]*(?:\.[a-zA-Z_$][a-zA-Z0-9_$]*)*)\s*\(")
                .unwrap()
        });

        for caps in CONSTR_RE.captures_iter(content) {
            let class = caps[1].to_string();
            deps.push((class, 0.75));
        }

        // Method calls: object.method(...)
        static METHOD_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\.([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(").unwrap()
        });

        for caps in METHOD_RE.captures_iter(content) {
            let method_path = format!("{}.{}", &caps[1], &caps[2]);
            deps.push((method_path, 0.65));
        }

        deps
    }

    /// Extract type references and annotations
    pub fn extract_type_refs(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // Type annotations: : TypeName
        static TYPE_ANNOT_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r":\s*([a-zA-Z_$][a-zA-Z0-9_$]*(?:<[^>]+>)?)\s*[,;=\)]").unwrap()
        });

        for caps in TYPE_ANNOT_RE.captures_iter(content) {
            let type_name = caps[1].to_string();
            // Exclude primitive types
            if !matches!(
                type_name.as_str(),
                "string" | "number" | "boolean" | "any" | "void" | "object" | "unknown"
            ) {
                deps.push((type_name, 0.70));
            }
        }

        // Generic type parameters: <TypeParam>
        static GENERIC_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"<\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\s*(?:[,>]|extends)").unwrap());

        for caps in GENERIC_RE.captures_iter(content) {
            let type_param = caps[1].to_string();
            if type_param.len() > 1 {
                deps.push((type_param, 0.55));
            }
        }

        deps
    }

    /// Extract class inheritance and interface implementation
    pub fn extract_inherit(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // Class extends Parent
        static EXTENDS_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"class\s+[a-zA-Z_$][a-zA-Z0-9_$]*\s+extends\s+([a-zA-Z_$][a-zA-Z0-9_$.]*)")
                .unwrap()
        });

        for caps in EXTENDS_RE.captures_iter(content) {
            let parent = caps[1].to_string();
            deps.push((parent, 0.90));
        }

        // Class implements Interface
        static IMPL_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"class\s+[a-zA-Z_$][a-zA-Z0-9_$]*(?:\s+extends\s+[a-zA-Z_$][a-zA-Z0-9_$.]*)?\s+implements\s+([a-zA-Z_$][a-zA-Z0-9_$,.\s]*)")
                .unwrap()
        });

        for caps in IMPL_RE.captures_iter(content) {
            let interfaces = &caps[1];
            for iface in interfaces.split(',') {
                let iface = iface.trim().to_string();
                deps.push((iface, 0.85));
            }
        }

        // Interface extends Interface
        static IFACE_EXTENDS_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"interface\s+[a-zA-Z_$][a-zA-Z0-9_$]*\s+extends\s+([a-zA-Z_$][a-zA-Z0-9_$,.\s]*)",
            )
            .unwrap()
        });

        for caps in IFACE_EXTENDS_RE.captures_iter(content) {
            let parents = &caps[1];
            for parent in parents.split(',') {
                let parent = parent.trim().to_string();
                deps.push((parent, 0.85));
            }
        }

        deps
    }
}

// ============================================================================
// PYTHON EXTRACTORS
// ============================================================================

pub struct PythonExtractor;

impl PythonExtractor {
    /// Extract import statements (import x, from x import y)
    pub fn extract_imports(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // from x import y
        static FROM_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"from\s+([a-zA-Z_][a-zA-Z0-9_\.]*)\s+import").unwrap());

        for caps in FROM_RE.captures_iter(content) {
            let module = caps[1].to_string();
            deps.push((module, 0.95));
        }

        // import x
        static IMPORT_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"import\s+([a-zA-Z_][a-zA-Z0-9_\.]*(?:\s+as\s+[a-zA-Z_][a-zA-Z0-9_]*)?)")
                .unwrap()
        });

        for caps in IMPORT_RE.captures_iter(content) {
            let module = caps[1].to_string();
            // Remove 'as' clause
            if let Some(pos) = module.find(" as ") {
                let module = module[..pos].to_string();
                deps.push((module, 0.95));
            } else {
                deps.push((module, 0.95));
            }
        }

        deps
    }

    /// Extract function calls and class instantiations
    pub fn extract_calls(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // Function/method calls: obj.method(...)
        static CALL_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\(").unwrap()
        });

        for caps in CALL_RE.captures_iter(content) {
            let func = caps[1].to_string();
            // Skip common built-ins
            if !matches!(
                func.as_str(),
                "print"
                    | "len"
                    | "str"
                    | "int"
                    | "float"
                    | "list"
                    | "dict"
                    | "set"
                    | "tuple"
                    | "range"
                    | "enumerate"
                    | "zip"
                    | "map"
                    | "filter"
                    | "all"
                    | "any"
            ) {
                deps.push((func, 0.65));
            }
        }

        deps
    }

    /// Extract class definitions and inheritance
    pub fn extract_inherit(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // class Child(Parent)
        static CLASS_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)").unwrap());

        for caps in CLASS_RE.captures_iter(content) {
            let parents = &caps[2];
            for parent in parents.split(',') {
                let parent = parent.trim().to_string();
                if !parent.is_empty() && parent != "object" {
                    deps.push((parent, 0.85));
                }
            }
        }

        deps
    }

    /// Extract type hints (def func(arg: Type) -> ReturnType)
    pub fn extract_type_hints(content: &str) -> Vec<(String, f32)> {
        let mut deps = Vec::new();

        // Type annotation in function signature: -> Type or : Type
        static TYPE_HINT_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?::|->\s*)([a-zA-Z_][a-zA-Z0-9_]*(?:\[[^\]]+\])?)\s*[,\)]").unwrap()
        });

        for caps in TYPE_HINT_RE.captures_iter(content) {
            let type_name = caps[1].to_string();
            // Skip primitive/built-in types
            if !matches!(
                type_name.as_str(),
                "str"
                    | "int"
                    | "float"
                    | "bool"
                    | "list"
                    | "dict"
                    | "set"
                    | "tuple"
                    | "None"
                    | "Any"
                    | "Optional"
                    | "Union"
            ) {
                deps.push((type_name, 0.70));
            }
        }

        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_imports() {
        let code = r#"
        use std::collections::HashMap;
        use serde::{Deserialize, Serialize};
        "#;
        let imports = RustExtractor::extract_imports(code);
        assert!(imports.iter().any(|(i, _)| i.contains("std")));
        assert!(imports.iter().any(|(i, _)| i.contains("serde")));
    }

    #[test]
    fn test_typescript_imports() {
        let code = r#"
        import React, { useState } from 'react';
        const lodash = require('lodash');
        "#;
        let imports = TypeScriptExtractor::extract_imports(code);
        assert!(imports.iter().any(|(i, _)| i.contains("react")));
        assert!(imports.iter().any(|(i, _)| i.contains("lodash")));
    }

    #[test]
    fn test_typescript_inheritance() {
        let code = r#"
        class Child extends Parent implements Interface1, Interface2 {
        }
        "#;
        let inherits = TypeScriptExtractor::extract_inherit(code);
        assert!(inherits.iter().any(|(i, _)| i.contains("Parent")));
        assert!(inherits.iter().any(|(i, _)| i.contains("Interface1")));
    }

    #[test]
    fn test_python_imports() {
        let code = r#"
        import os
        from collections import defaultdict
        "#;
        let imports = PythonExtractor::extract_imports(code);
        assert!(imports.iter().any(|(i, _)| i == "os"));
        assert!(imports.iter().any(|(i, _)| i == "collections"));
    }

    #[test]
    fn test_python_inheritance() {
        let code = r#"
        class Child(Parent1, Parent2):
            pass
        "#;
        let inherits = PythonExtractor::extract_inherit(code);
        assert!(inherits.iter().any(|(i, _)| i.contains("Parent1")));
        assert!(inherits.iter().any(|(i, _)| i.contains("Parent2")));
    }
}
