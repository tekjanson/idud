// src/analysis/ast_analyzer.rs
//! AST-based dependency analyzer
//! Extracts contracts from source code using language-specific regex extractors

use super::extractors::{PythonExtractor, RustExtractor, TypeScriptExtractor};
use anyhow::Result;
use std::path::Path;

/// Represents a dependency extracted from source code
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Source file or function URI
    pub from_uri: String,
    /// Target file, module, or function URI
    pub to_uri: String,
    /// Type of dependency: "import", "call", "type_ref", "inherit"
    pub dep_type: String,
    /// Confidence score: 0.0-1.0
    /// Explicit imports: 0.95, inferred calls: 0.60
    pub confidence: f32,
}

impl Dependency {
    pub fn new(from_uri: String, to_uri: String, dep_type: String, confidence: f32) -> Self {
        Self {
            from_uri,
            to_uri,
            dep_type,
            confidence: confidence.max(0.0).min(1.0),
        }
    }
}

/// Main AST analyzer struct
pub struct ASTAnalyzer;

impl ASTAnalyzer {
    /// Analyze a Rust file and extract dependencies
    pub fn analyze_rust_file(file_uri: &str, content: &str) -> Vec<Dependency> {
        let mut deps = Vec::new();

        // Extract imports
        for (import, confidence) in RustExtractor::extract_imports(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                import,
                "import".to_string(),
                confidence,
            ));
        }

        // Extract calls
        for (call, confidence) in RustExtractor::extract_calls(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                call,
                "call".to_string(),
                confidence,
            ));
        }

        // Extract trait references
        for (trait_ref, confidence) in RustExtractor::extract_trait_refs(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                trait_ref,
                "type_ref".to_string(),
                confidence,
            ));
        }

        // Extract inheritance patterns
        for (inherit, confidence) in RustExtractor::extract_inherit(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                inherit,
                "inherit".to_string(),
                confidence,
            ));
        }

        deps
    }

    /// Analyze a TypeScript/JavaScript file and extract dependencies
    pub fn analyze_typescript_file(file_uri: &str, content: &str) -> Vec<Dependency> {
        let mut deps = Vec::new();

        // Extract imports
        for (import, confidence) in TypeScriptExtractor::extract_imports(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                import,
                "import".to_string(),
                confidence,
            ));
        }

        // Extract calls
        for (call, confidence) in TypeScriptExtractor::extract_calls(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                call,
                "call".to_string(),
                confidence,
            ));
        }

        // Extract type references
        for (type_ref, confidence) in TypeScriptExtractor::extract_type_refs(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                type_ref,
                "type_ref".to_string(),
                confidence,
            ));
        }

        // Extract inheritance
        for (inherit, confidence) in TypeScriptExtractor::extract_inherit(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                inherit,
                "inherit".to_string(),
                confidence,
            ));
        }

        deps
    }

    /// Analyze a Python file and extract dependencies
    pub fn analyze_python_file(file_uri: &str, content: &str) -> Vec<Dependency> {
        let mut deps = Vec::new();

        // Extract imports
        for (import, confidence) in PythonExtractor::extract_imports(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                import,
                "import".to_string(),
                confidence,
            ));
        }

        // Extract calls
        for (call, confidence) in PythonExtractor::extract_calls(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                call,
                "call".to_string(),
                confidence,
            ));
        }

        // Extract inheritance
        for (inherit, confidence) in PythonExtractor::extract_inherit(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                inherit,
                "inherit".to_string(),
                confidence,
            ));
        }

        // Extract type hints
        for (type_hint, confidence) in PythonExtractor::extract_type_hints(content) {
            deps.push(Dependency::new(
                file_uri.to_string(),
                type_hint,
                "type_ref".to_string(),
                confidence,
            ));
        }

        deps
    }

    /// Analyze a file based on extension and return dependencies
    pub fn analyze_file(file_path: &Path, content: &str) -> Result<Vec<Dependency>> {
        let file_uri = file_path.to_string_lossy().to_string();
        let file_str = file_uri.to_lowercase();

        if file_str.ends_with(".rs") {
            Ok(Self::analyze_rust_file(&file_uri, content))
        } else if file_str.ends_with(".ts")
            || file_str.ends_with(".tsx")
            || file_str.ends_with(".js")
            || file_str.ends_with(".jsx")
        {
            Ok(Self::analyze_typescript_file(&file_uri, content))
        } else if file_str.ends_with(".py") {
            Ok(Self::analyze_python_file(&file_uri, content))
        } else {
            Ok(Vec::new())
        }
    }

    /// Analyze all files in a directory recursively
    pub fn analyze_all_files(repo_path: &Path) -> Result<Vec<Dependency>> {
        use walkdir::WalkDir;

        let mut all_deps = Vec::new();

        for entry in WalkDir::new(repo_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| !e.path().to_string_lossy().contains("/.git/"))
            .filter(|e| !e.path().to_string_lossy().contains("/node_modules/"))
            .filter(|e| !e.path().to_string_lossy().contains("/target/"))
        {
            let path = entry.path();
            if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(deps) = Self::analyze_file(path, &content) {
                        all_deps.extend(deps);
                    }
                }
            }
        }

        Ok(all_deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_rust_file() {
        let code = r#"
        use std::collections::HashMap;
        use serde::{Deserialize, Serialize};

        impl MyTrait for MyStruct {
            fn method(&self) {}
        }
        "#;

        let deps = ASTAnalyzer::analyze_rust_file("test.rs", code);
        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.dep_type == "import"));
        assert!(deps.iter().any(|d| d.dep_type == "type_ref"));
    }

    #[test]
    fn test_analyze_typescript_file() {
        let code = r#"
        import React, { useState } from 'react';
        import * as utils from './utils';

        class MyClass extends BaseClass implements Interface1 {
            constructor() {
                super();
            }

            method() {
                const obj = new Helper();
            }
        }
        "#;

        let deps = ASTAnalyzer::analyze_typescript_file("test.ts", code);
        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.dep_type == "import" && d.to_uri.contains("react")));
        assert!(deps.iter().any(|d| d.dep_type == "inherit"));
    }

    #[test]
    fn test_analyze_python_file() {
        let code = r#"
        import os
        from collections import defaultdict
        from typing import Optional, List

        class Child(Parent):
            def __init__(self):
                super().__init__()

            def process(self, data: List[str]) -> Optional[str]:
                return data[0] if data else None
        "#;

        let deps = ASTAnalyzer::analyze_python_file("test.py", code);
        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.dep_type == "import" && d.to_uri == "os"));
        assert!(deps.iter().any(|d| d.dep_type == "inherit"));
    }

    #[test]
    fn test_confidence_scores() {
        let code = "use std::collections::HashMap;";
        let deps = ASTAnalyzer::analyze_rust_file("test.rs", code);

        // Explicit imports should have high confidence
        for dep in deps.iter().filter(|d| d.dep_type == "import") {
            assert!(dep.confidence >= 0.90);
        }
    }

    #[test]
    fn test_analyze_file_by_extension() {
        let rust_code = "use std::io;";
        let ts_code = "import React from 'react';";
        let py_code = "import os";

        let rust_deps = ASTAnalyzer::analyze_file(Path::new("test.rs"), rust_code).unwrap();
        let ts_deps = ASTAnalyzer::analyze_file(Path::new("test.ts"), ts_code).unwrap();
        let py_deps = ASTAnalyzer::analyze_file(Path::new("test.py"), py_code).unwrap();

        assert!(!rust_deps.is_empty());
        assert!(!ts_deps.is_empty());
        assert!(!py_deps.is_empty());
    }
}
