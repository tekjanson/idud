// src/pipelines/broad_sweep.rs
//! PHASE 3.1: The Broad Sweep
//! Deterministic Signatory registration from repository files
//! Clones repo, traverses filesystem, chunks code into Signatories.

use crate::analysis::{AILinker, AILinkerConfig};
use crate::schemas::{ContractValidator, SignatoryFactory};
use crate::types::*;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct RepositoryIngestionConfig {
    pub repo_url: String,
    pub branch: String,
    pub work_dir: Option<PathBuf>,
    pub skip_clone: bool,
}

pub struct IngestionResult {
    pub repository: String,
    pub files_processed: usize,
    pub signatories_registered: Vec<Signatory>,
    pub contracts_discovered: Vec<Contract>,
    pub errors: Vec<String>,
}

pub struct RepositoryTraverser {
    config: RepositoryIngestionConfig,
    work_dir: PathBuf,
}

impl RepositoryTraverser {
    pub fn new(config: RepositoryIngestionConfig) -> Self {
        let work_dir = config
            .work_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/idud-repo-{}", std::process::id())));

        Self { config, work_dir }
    }

    /// Clone repository to working directory
    async fn clone(&self) -> Result<()> {
        std::fs::create_dir_all(&self.work_dir)?;
        let output = tokio::process::Command::new("git")
            .args(&[
                "clone",
                "--branch",
                &self.config.branch,
                "--depth",
                "1",
                &self.config.repo_url,
                self.work_dir.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to clone: {}", stderr));
        }

        Ok(())
    }

    /// Extract functions and classes from TypeScript/JavaScript
    fn extract_code_structure(&self, file_path: &Path) -> Result<Vec<CodeElement>> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut elements = Vec::new();

        // Function regex: function name() or const name = () =>
        let func_re = Regex::new(r"(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(")?;
        let const_func_re = Regex::new(r"const\s+(\w+)\s*=\s*(?:async\s+)?\(")?;
        let class_re = Regex::new(r"(?:export\s+)?class\s+(\w+)")?;

        for (line_num, line) in lines.iter().enumerate() {
            if let Some(caps) = func_re.captures(line) {
                elements.push(CodeElement {
                    element_type: "FUNCTION".to_string(),
                    name: caps[1].to_string(),
                    start: line_num + 1,
                    end: line_num + 10,
                    snippet: lines[line_num..std::cmp::min(line_num + 10, lines.len())].join("\n"),
                });
            }
            if let Some(caps) = const_func_re.captures(line) {
                elements.push(CodeElement {
                    element_type: "FUNCTION".to_string(),
                    name: caps[1].to_string(),
                    start: line_num + 1,
                    end: line_num + 10,
                    snippet: lines[line_num..std::cmp::min(line_num + 10, lines.len())].join("\n"),
                });
            }
            if let Some(caps) = class_re.captures(line) {
                elements.push(CodeElement {
                    element_type: "CLASS".to_string(),
                    name: caps[1].to_string(),
                    start: line_num + 1,
                    end: line_num + 20,
                    snippet: lines[line_num..std::cmp::min(line_num + 20, lines.len())].join("\n"),
                });
            }
        }

        Ok(elements)
    }

    /// Extract tests from test files
    fn extract_tests(&self, file_path: &Path) -> Result<Vec<TestElement>> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut tests = Vec::new();

        let test_re = Regex::new(r#"(?:test|it)\s*\(\s*["'`]([^"'`]+)["'`]"#)?;

        for (line_num, line) in lines.iter().enumerate() {
            if let Some(caps) = test_re.captures(line) {
                tests.push(TestElement {
                    name: caps[1].to_string(),
                    start: line_num + 1,
                    end: line_num + 10,
                    snippet: lines[line_num..std::cmp::min(line_num + 10, lines.len())].join("\n"),
                });
            }
        }

        Ok(tests)
    }

    /// Extract markdown sections
    fn extract_markdown_sections(&self, file_path: &Path) -> Result<Vec<MarkdownSection>> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut sections = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            if line.starts_with('#') {
                let heading = line.trim_start_matches('#').trim().to_string();
                let end = std::cmp::min(line_num + 10, lines.len());
                sections.push(MarkdownSection {
                    heading,
                    start: line_num + 1,
                    end,
                    snippet: lines[line_num..end].join("\n"),
                });
            }
        }

        Ok(sections)
    }

    /// Main ingestion: traverse repo and register all signatories and discover contracts
    pub async fn ingest(&self) -> Result<IngestionResult> {
        eprintln!("[INGEST] Starting ingestion of: {:?}", self.work_dir);
        let mut signatories = Vec::new();
        let mut errors = Vec::new();

        // Clone repository (skip if local)
        if !self.config.skip_clone {
            eprintln!("[INGEST] Cloning repository...");
            if let Err(e) = self.clone().await {
                errors.push(format!("Clone failed: {}", e));
                return Ok(IngestionResult {
                    repository: self.config.repo_url.clone(),
                    files_processed: 0,
                    signatories_registered: signatories,
                    contracts_discovered: Vec::new(),
                    errors,
                });
            }
        } else {
            eprintln!("[INGEST] Skipping clone, using local directory");
        }

        let mut files_processed = 0;
        eprintln!("[INGEST] Starting walk of directory...");

        // Walk repository with early directory filtering
        let walker = WalkDir::new(&self.work_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                let path_str = path.to_string_lossy();
                // Skip build/cache directories early
                !path_str.contains("/.git")
                    && !path_str.contains("/node_modules")
                    && !path_str.contains("/target")
                    && !path_str.contains("/dist")
                    && !path_str.contains("/build")
            });

        for entry in walker {
            let path = entry.path();

            if path.is_file() {
                // Skip binary and non-code files early
                let path_str = path.to_string_lossy().to_lowercase();
                if path_str.ends_with(".png")
                    || path_str.ends_with(".jpg")
                    || path_str.ends_with(".jpeg")
                    || path_str.ends_with(".gif")
                    || path_str.ends_with(".zip")
                    || path_str.ends_with(".tar")
                    || path_str.ends_with(".gz")
                    || path_str.ends_with(".bin")
                    || path_str.ends_with(".so")
                    || path_str.ends_with(".dylib")
                    || path_str.ends_with(".dll")
                    || path_str.ends_with(".pdf")
                {
                    continue;
                }

                files_processed += 1;

                if let Ok(relative) = path.strip_prefix(&self.work_dir) {
                    let rel_str = relative.to_string_lossy().to_string();

                    // Register FILE signatory
                    let file_signatory = SignatoryFactory::register_file(
                        &self.config.repo_url,
                        &rel_str,
                        &self.config.branch,
                    );
                    if ContractValidator::audit_signatory(&file_signatory).is_ok() {
                        signatories.push(file_signatory.clone());
                    }

                    // Extract code elements for TS/JS
                    if rel_str.ends_with(".ts")
                        || rel_str.ends_with(".tsx")
                        || rel_str.ends_with(".js")
                    {
                        match self.extract_code_structure(path) {
                            Ok(elements) => {
                                for elem in elements {
                                    if elem.element_type == "FUNCTION" {
                                        let signatory = SignatoryFactory::register_function(
                                            &self.config.repo_url,
                                            &rel_str,
                                            &elem.name,
                                            elem.snippet,
                                            elem.start,
                                            elem.end,
                                            &self.config.branch,
                                        );
                                        if ContractValidator::audit_signatory(&signatory).is_ok() {
                                            signatories.push(signatory);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(format!("Code extraction error in {}: {}", rel_str, e));
                            }
                        }

                        // Extract tests
                        if rel_str.contains(".test.") || rel_str.contains(".spec.") {
                            match self.extract_tests(path) {
                                Ok(tests) => {
                                    for test in tests {
                                        let signatory = SignatoryFactory::register_test(
                                            &self.config.repo_url,
                                            &rel_str,
                                            &test.name,
                                            test.snippet,
                                            test.start,
                                            test.end,
                                            &self.config.branch,
                                        );
                                        if ContractValidator::audit_signatory(&signatory).is_ok() {
                                            signatories.push(signatory);
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!(
                                        "Test extraction error in {}: {}",
                                        rel_str, e
                                    ));
                                }
                            }
                        }
                    }

                    // Extract markdown sections
                    if rel_str.ends_with(".md") {
                        match self.extract_markdown_sections(path) {
                            Ok(sections) => {
                                for section in sections {
                                    let doc_uri = format!(
                                        "{}/blob/{}/{}",
                                        &self.config.repo_url, &self.config.branch, &rel_str
                                    );
                                    let signatory = SignatoryFactory::register_documentation(
                                        &doc_uri,
                                        &section.heading.to_lowercase().replace(" ", "-"),
                                        &section.heading,
                                        section.snippet,
                                    );
                                    if ContractValidator::audit_signatory(&signatory).is_ok() {
                                        signatories.push(signatory);
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(format!(
                                    "Markdown extraction error in {}: {}",
                                    rel_str, e
                                ));
                            }
                        }
                    }
                }
            }
        }

        eprintln!("[INGEST] Walk complete, files processed: {}, signatories: {}", files_processed, signatories.len());

        // Analyze dependencies using AI linking (optional post-processing step)
        let mut contracts_discovered = Vec::new();
        
        // Check if AI linking is enabled
        let enable_ai_linking = std::env::var("ENABLE_AI_LINKING")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true); // Default to true

        if enable_ai_linking && !signatories.is_empty() {
            eprintln!("[INGEST] Starting AI linking pass...");
            tracing::info!("Starting AI linking pass on {} signatories", signatories.len());
            let mut linker = AILinker::new(AILinkerConfig::default());
            match linker.link_files(&signatories, &contracts_discovered) {
                Ok(ai_contracts) => {
                    eprintln!("[INGEST] AI linking found {} contracts", ai_contracts.len());
                    tracing::info!("AI linking inferred {} semantic dependencies", ai_contracts.len());
                    contracts_discovered.extend(ai_contracts);
                }
                Err(e) => {
                    eprintln!("[INGEST] AI linking error: {}", e);
                    tracing::warn!("AI linking failed (continuing without AI contracts): {}", e);
                }
            }
        }

        // Only clean up if we cloned (don't delete local repos!)
        if !self.config.skip_clone {
            eprintln!("[INGEST] Cleaning up cloned repository");
            let _ = std::fs::remove_dir_all(&self.work_dir);
        }

        eprintln!("[INGEST] Returning results");
        Ok(IngestionResult {
            repository: self.config.repo_url.clone(),
            files_processed,
            signatories_registered: signatories,
            contracts_discovered,
            errors,
        })
    }
}

struct CodeElement {
    element_type: String,
    name: String,
    start: usize,
    end: usize,
    snippet: String,
}

struct TestElement {
    name: String,
    start: usize,
    end: usize,
    snippet: String,
}

struct MarkdownSection {
    heading: String,
    start: usize,
    end: usize,
    snippet: String,
}
