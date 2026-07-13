// tests/integration_ast_analysis.rs
//! Integration tests for AST-based dependency analysis

use idud::ASTAnalyzer;
use std::path::Path;

#[test]
fn test_rust_analysis_integration() {
    let rust_code = r#"
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub fn process_data(input: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    Ok(HashMap::new())
}
"#;

    let deps = ASTAnalyzer::analyze_file(Path::new("main.rs"), rust_code).unwrap();

    assert!(!deps.is_empty());
    assert!(deps
        .iter()
        .any(|d| d.dep_type == "import" && d.to_uri.contains("std")));
    assert!(deps
        .iter()
        .any(|d| d.dep_type == "import" && d.to_uri.contains("serde")));

    // Verify confidence scores
    for dep in &deps {
        match dep.dep_type.as_str() {
            "import" => assert!(dep.confidence >= 0.90),
            "call" => assert!(dep.confidence >= 0.60 && dep.confidence <= 0.80),
            _ => {}
        }
    }
}

#[test]
fn test_typescript_analysis_integration() {
    let ts_code = r#"
import React, { useState } from 'react';
import { useRouter } from 'next/router';
import axios from 'axios';

export const MyComponent: React.FC = () => {
    const [count, setCount] = useState(0);
    const router = useRouter();
    
    const fetchData = async () => {
        const response = await axios.get('/api/data');
        setCount(response.data.length);
    };
    
    return <div onClick={fetchData}>{count}</div>;
};
"#;

    let deps = ASTAnalyzer::analyze_file(Path::new("component.tsx"), ts_code).unwrap();

    assert!(!deps.is_empty());

    let imports: Vec<_> = deps.iter().filter(|d| d.dep_type == "import").collect();
    assert!(!imports.is_empty());
    assert!(imports.iter().any(|d| d.to_uri.contains("react")));
    assert!(imports.iter().any(|d| d.to_uri.contains("next")));
    assert!(imports.iter().any(|d| d.to_uri.contains("axios")));
}

#[test]
fn test_python_analysis_integration() {
    let py_code = r#"
import os
from typing import List, Optional
from pathlib import Path
import requests
from .models import User

class DataProcessor(object):
    def __init__(self):
        self.users: List[User] = []
    
    def load_data(self, path: str) -> Optional[List[User]]:
        file_path = Path(path)
        response = requests.get('http://api.example.com/users')
        return response.json()
"#;

    let deps = ASTAnalyzer::analyze_file(Path::new("processor.py"), py_code).unwrap();

    assert!(!deps.is_empty());

    let imports: Vec<_> = deps.iter().filter(|d| d.dep_type == "import").collect();
    assert!(!imports.is_empty());
    assert!(imports.iter().any(|d| d.to_uri == "os"));
    assert!(imports.iter().any(|d| d.to_uri == "typing"));
    assert!(imports.iter().any(|d| d.to_uri == "requests"));
}

#[test]
fn test_confidence_scores_are_calibrated() {
    let rust_code = "use std::io;";
    let ts_code = "import { x } from 'module';";
    let py_code = "import os";

    let rust_deps = ASTAnalyzer::analyze_file(Path::new("test.rs"), rust_code).unwrap();
    let ts_deps = ASTAnalyzer::analyze_file(Path::new("test.ts"), ts_code).unwrap();
    let py_deps = ASTAnalyzer::analyze_file(Path::new("test.py"), py_code).unwrap();

    // All imports should have high confidence (0.95)
    for dep in rust_deps.iter().chain(ts_deps.iter()).chain(py_deps.iter()) {
        if dep.dep_type == "import" {
            assert_eq!(dep.confidence, 0.95, "Import confidence should be 0.95");
        }
    }
}
