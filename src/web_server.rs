//! src/web_server.rs
//! Simple HTTP server serving the contract graph visualization

use actix_web::{web, App, HttpServer, HttpResponse, middleware};
use actix_files::Files;
use actix_multipart::Multipart;
use std::sync::Arc;
use crate::{ContractLedger, RepositoryTraverser, RepositoryIngestionConfig};
use serde_json::json;
use serde::{Deserialize, Serialize};
use pulldown_cmark::{Parser, Event};
use futures_util::TryStreamExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestRepoRequest {
    pub url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Serialize)]
pub struct IngestRepoResponse {
    pub success: bool,
    pub message: String,
    pub signatories_count: usize,
    pub contracts_count: usize,
}

pub struct WebServerConfig {
    pub port: u16,
    pub host: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub success: bool,
    pub message: String,
    pub signatories_added: usize,
    pub sections_parsed: usize,
}

#[derive(Debug, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub signatory_type: String,
    pub source_uri: String,
}

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub clause_type: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: (usize, usize),
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
        }
    }
}

pub async fn serve(ledger: Arc<ContractLedger>, config: WebServerConfig) -> std::io::Result<()> {
    let ledger = web::Data::new(ledger);
    let addr = format!("{}:{}", config.host, config.port);
    
    println!("🌐 Starting visualization server at http://{}", addr);
    
    HttpServer::new(move || {
        App::new()
            .app_data(ledger.clone())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api")
                    .route("/graph", web::get().to(get_graph))
                    .route("/signatories", web::get().to(get_signatories))
                    .route("/contracts", web::get().to(get_contracts))
                    .route("/chain/{id}", web::get().to(get_chain))
                    .route("/trace-chain/{id}", web::get().to(trace_chain))
                    .route("/export", web::get().to(export_graph))
                    .route("/search", web::get().to(search_nodes))
                    .route("/import-url", web::post().to(import_url))
                    .route("/import-file", web::post().to(import_file))
                    .route("/ingest-repo", web::post().to(ingest_repo))
                    .route("/training/discover", web::get().to(training_discover))
                    .route("/training/issue/{repo_owner}/{repo_name}/{issue_id}", 
                           web::get().to(training_fetch_issue))
                    .route("/training/predict", web::post().to(training_predict))
                    .route("/training/validate", web::post().to(training_validate))
                    .route("/training/metrics", web::get().to(training_metrics))
            )
            .service(Files::new("/", "./ui/dist").index_file("index.html"))
    })
    .bind(&addr)?
    .run()
    .await
}

async fn get_graph(ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    let signatories = ledger.get_all_signatories();
    let contracts = ledger.get_all_contracts();
    let stats = ledger.stats();

    let nodes: Vec<GraphNode> = signatories
        .iter()
        .map(|sig| GraphNode {
            id: sig.id.clone(),
            label: sig.label.clone(),
            signatory_type: format!("{:?}", sig.signatory_type),
            source_uri: sig.source_uri.clone(),
        })
        .collect();

    let edges: Vec<GraphEdge> = contracts
        .iter()
        .map(|contract| GraphEdge {
            source: contract.principal_id.clone(),
            target: contract.guarantor_id.clone(),
            clause_type: format!("{:?}", contract.clause_type),
            confidence: contract.confidence,
        })
        .collect();

    HttpResponse::Ok().json(GraphResponse { nodes, edges, stats })
}

async fn get_signatories(ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    let signatories = ledger.get_all_signatories();
    HttpResponse::Ok().json(json!({
        "signatories": signatories,
        "count": signatories.len()
    }))
}

async fn get_contracts(ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    let contracts = ledger.get_all_contracts();
    HttpResponse::Ok().json(json!({
        "contracts": contracts,
        "count": contracts.len()
    }))
}

async fn get_chain(
    id: web::Path<String>,
    ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    let node_id = id.into_inner();
    if let Some(chain) = ledger.trace_chain_of_obligation(&node_id, 5) {
        HttpResponse::Ok().json(chain)
    } else {
        HttpResponse::NotFound().json(json!({
            "error": format!("Signatory {} not found", node_id)
        }))
    }
}

async fn trace_chain(
    id: web::Path<String>,
    ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    let node_id = id.into_inner();
    if let Some(chain) = ledger.trace_chain_of_obligation(&node_id, 5) {
        HttpResponse::Ok().json(chain)
    } else {
        HttpResponse::NotFound().json(json!({
            "error": format!("Signatory {} not found", node_id)
        }))
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

async fn search_nodes(
    query: web::Query<SearchQuery>,
    ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    let search_term = query.q.to_lowercase();
    let signatories = ledger.get_all_signatories();
    
    let results: Vec<&crate::types::Signatory> = signatories
        .iter()
        .filter(|sig| {
            sig.label.to_lowercase().contains(&search_term)
                || sig.id.to_lowercase().contains(&search_term)
                || sig.source_uri.to_lowercase().contains(&search_term)
        })
        .collect();

    HttpResponse::Ok().json(json!({
        "results": results,
        "count": results.len()
    }))
}

async fn export_graph(ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    let signatories = ledger.get_all_signatories();
    let contracts = ledger.get_all_contracts();
    let stats = ledger.stats();

    let export_data = json!({
        "version": "1.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "stats": {
            "signatories": stats.0,
            "contracts": stats.1
        },
        "signatories": signatories,
        "contracts": contracts
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header(("Content-Disposition", "attachment; filename=\"idud-export.json\""))
        .json(export_data)
}

async fn import_url(
    req: web::Json<ImportUrlRequest>,
    ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    match fetch_and_parse_markdown(&req.url).await {
        Ok((content, sections)) => {
            let signatories_added = register_markdown_sections(ledger.get_ref(), &content, &sections);
            HttpResponse::Ok().json(ImportResponse {
                success: true,
                message: format!("Successfully imported {} sections from URL", sections.len()),
                signatories_added,
                sections_parsed: sections.len(),
            })
        }
        Err(e) => {
            HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": format!("Failed to import from URL: {}", e),
            }))
        }
    }
}

async fn import_file(
    mut payload: Multipart,
    ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    let mut file_content = String::new();
    
    while let Ok(Some(mut field)) = payload.try_next().await {
        if field.name() == "file" {
            let mut data = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                data.extend_from_slice(&chunk);
            }
            file_content = String::from_utf8_lossy(&data).to_string();
        }
    }
    
    if file_content.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "No file content provided",
        }));
    }
    
    match parse_markdown_content(&file_content) {
        Ok(sections) => {
            let signatories_added = register_markdown_sections(ledger.get_ref(), &file_content, &sections);
            HttpResponse::Ok().json(ImportResponse {
                success: true,
                message: format!("Successfully imported {} sections from file", sections.len()),
                signatories_added,
                sections_parsed: sections.len(),
            })
        }
        Err(e) => {
            HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": format!("Failed to parse markdown: {}", e),
            }))
        }
    }
}

async fn fetch_and_parse_markdown(url: &str) -> Result<(String, Vec<MarkdownSection>), String> {
    let content = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    let sections = parse_markdown_content(&content)?;
    Ok((content, sections))
}

#[derive(Debug, Clone)]
struct MarkdownSection {
    title: String,
    level: usize,
    content: String,
    links: Vec<String>,
}

fn parse_markdown_content(content: &str) -> Result<Vec<MarkdownSection>, String> {
    let parser = Parser::new(content);
    let mut sections = Vec::new();
    let mut current_section: Option<MarkdownSection> = None;
    let mut current_content = String::new();
    
    for event in parser {
        match event {
            Event::Start(tag) => {
                match tag {
                    pulldown_cmark::Tag::Heading(level, _, _) => {
                        if let Some(section) = current_section.take() {
                            sections.push(MarkdownSection {
                                title: section.title,
                                level: section.level,
                                content: current_content.trim().to_string(),
                                links: section.links,
                            });
                            current_content.clear();
                        }
                        current_section = Some(MarkdownSection {
                            title: String::new(),
                            level: match level {
                                pulldown_cmark::HeadingLevel::H1 => 1,
                                pulldown_cmark::HeadingLevel::H2 => 2,
                                pulldown_cmark::HeadingLevel::H3 => 3,
                                pulldown_cmark::HeadingLevel::H4 => 4,
                                pulldown_cmark::HeadingLevel::H5 => 5,
                                pulldown_cmark::HeadingLevel::H6 => 6,
                            },
                            content: String::new(),
                            links: Vec::new(),
                        });
                    }
                    pulldown_cmark::Tag::Link(_, url, _) => {
                        if let Some(section) = &mut current_section {
                            section.links.push(url.to_string());
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                if let Some(section) = &mut current_section {
                    if section.title.is_empty() {
                        section.title = text.trim().to_string();
                    }
                }
                current_content.push_str(&text);
                current_content.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => {
                current_content.push('\n');
            }
            _ => {}
        }
    }
    
    if let Some(section) = current_section.take() {
        sections.push(MarkdownSection {
            title: section.title,
            level: section.level,
            content: current_content.trim().to_string(),
            links: section.links,
        });
    }
    
    Ok(sections)
}

fn register_markdown_sections(
    ledger: &Arc<ContractLedger>,
    _content: &str,
    sections: &[MarkdownSection],
) -> usize {
    use crate::types::{Signatory, SignatoryType, Contract, ClauseType, ContractSource};
    
    let mut registered = 0;
    let mut section_ids = Vec::new();
    
    for section in sections {
        let signatory = Signatory::new(
            SignatoryType::MarkdownSection,
            format!("doc://markdown#{}", registered),
            section.title.clone(),
            section.content.clone(),
        )
        .with_metadata("level".to_string(), json!(section.level))
        .with_metadata("links".to_string(), json!(section.links));
        
        if let Ok(sig_id) = ledger.register_signatory(signatory) {
            section_ids.push(sig_id);
            registered += 1;
        }
    }
    
    for (i, section_id) in section_ids.iter().enumerate() {
        let level = sections.get(i).map(|s| s.level).unwrap_or(1);
        
        if let Some(other_id) = section_ids.iter().skip(i + 1).next() {
            let other_level = sections
                .get(i + 1)
                .map(|s| s.level)
                .unwrap_or(level);
            
            if other_level > level {
                let contract = Contract::new(
                    other_id.clone(),
                    section_id.clone(),
                    ClauseType::Documents,
                    0.95,
                    ContractSource::Deterministic,
                )
                .with_reasoning(format!("Section at level {} documents level {}", level, other_level));
                
                let _ = ledger.draft_contract(contract);
            }
        }
    }
    
    registered
}

async fn ingest_repo(
    _ledger: web::Data<Arc<ContractLedger>>,
    req: web::Json<IngestRepoRequest>,
) -> HttpResponse {
    let url = req.url.clone();
    let branch = req.branch.clone();
    
    println!("📦 Ingest request: {} (branch: {})", url, branch);
    
    let config = RepositoryIngestionConfig {
        repo_url: url.clone(),
        branch: branch.clone(),
        work_dir: None,
    };
    
    match RepositoryTraverser::new(config).ingest().await {
        Ok(result) => {
            let sig_count = result.signatories_registered.len();
            println!("✅ Ingestion completed: {} signatories", sig_count);
            HttpResponse::Ok().json(IngestRepoResponse {
                success: true,
                message: format!("Successfully ingested {} ({})", url, branch),
                signatories_count: sig_count,
                contracts_count: 0,
            })
        }
        Err(e) => {
            eprintln!("❌ Ingestion failed: {}", e);
            HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": format!("Failed to ingest repository: {}", e),
                "signatories_count": 0,
                "contracts_count": 0,
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DiscoverQuery {
    #[serde(default = "default_discover_limit")]
    pub limit: usize,
}

fn default_discover_limit() -> usize {
    100
}

async fn training_discover(query: web::Query<DiscoverQuery>) -> HttpResponse {
    let limit = query.limit.min(1000).max(1);
    
    match crate::discover_training_repos(limit).await {
        Ok(candidates) => {
            HttpResponse::Ok().json(json!({
                "success": true,
                "candidates": candidates,
                "count": candidates.len(),
            }))
        }
        Err(e) => {
            eprintln!("❌ Repository discovery failed: {}", e);
            let status_code = match e {
                crate::training::discovery::DiscoveryError::RateLimited => {
                    actix_web::http::StatusCode::TOO_MANY_REQUESTS
                }
                _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            };
            HttpResponse::build(status_code).json(json!({
                "success": false,
                "error": e.to_string(),
            }))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingPredictRequest {
    pub issue_text: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TrainingPredictResponse {
    pub predicted_files: Vec<String>,
    pub model_used: String,
    pub tokens_used: crate::training::TokenUsage,
    pub reasoning: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssueParams {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_id: u32,
}

async fn training_fetch_issue(params: web::Path<IssueParams>) -> HttpResponse {
    let repo_owner = &params.repo_owner;
    let repo_name = &params.repo_name;
    let issue_id = params.issue_id;
    
    match crate::fetch_issue_and_linked_pr(repo_owner, repo_name, issue_id).await {
        Ok(issue_data) => {
            HttpResponse::Ok().json(json!({
                "success": true,
                "data": issue_data,
            }))
        }
        Err(e) => {
            eprintln!("❌ Issue fetch failed: {}", e);
            let status_code = match e {
                crate::training::discovery::DiscoveryError::RateLimited => {
                    actix_web::http::StatusCode::TOO_MANY_REQUESTS
                }
                crate::training::discovery::DiscoveryError::RepoNotFound(_) => {
                    actix_web::http::StatusCode::NOT_FOUND
                }
                _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            };
            HttpResponse::build(status_code).json(json!({
                "success": false,
                "error": e.to_string(),
            }))
        }
    }
}

async fn training_predict(
    req: web::Json<TrainingPredictRequest>,
    ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    let api_key = match &req.api_key {
        Some(key) => key.clone(),
        None => match std::env::var("ANTHROPIC_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                return HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "error": "Anthropic API key not provided and ANTHROPIC_API_KEY environment variable not set",
                }));
            }
        },
    };

    let signatories = ledger.get_all_signatories();
    let contracts = ledger.get_all_contracts();

    let prediction_request = crate::PredictionRequest {
        issue_text: req.issue_text.clone(),
        dependency_graph: contracts,
        signatories,
    };

    match crate::predict_files_from_issue(prediction_request, &api_key).await {
        Ok(response) => {
            HttpResponse::Ok().json(json!({
                "success": true,
                "predicted_files": response.predicted_files,
                "model_used": response.model_used,
                "tokens_used": response.tokens_used,
                "reasoning": response.reasoning,
            }))
        }
        Err(e) => {
            eprintln!("❌ Prediction failed: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Prediction failed: {}", e),
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TrainingValidateRequest {
    pub repo_url: String,
    pub issue_id: String,
    pub issue_text: String,
    pub predicted_files: Vec<String>,
    pub actual_files: Vec<String>,
    #[serde(default)]
    pub batch_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TrainingValidateResponse {
    pub success: bool,
    pub run_id: String,
    pub metrics: crate::ValidationMetrics,
}

#[derive(Debug, Serialize)]
pub struct TrainingMetricsResponse {
    pub success: bool,
    pub aggregated_metrics: crate::AggregatedMetrics,
    #[serde(default)]
    pub language_metrics: Option<std::collections::HashMap<String, crate::LanguageMetrics>>,
}

async fn training_validate(
    req: web::Json<TrainingValidateRequest>,
) -> HttpResponse {
    let datalake_path = "./data/training_datalake";
    
    match crate::TrainingDataLake::new(datalake_path) {
        Ok(datalake) => {
            // Validate the prediction
            let metrics = crate::validate_prediction(
                req.predicted_files.clone(),
                req.actual_files.clone(),
            );
            
            // Write to datalake
            match crate::write_training_result(
                &datalake,
                req.repo_url.clone(),
                req.issue_id.clone(),
                req.issue_text.clone(),
                req.predicted_files.clone(),
                req.actual_files.clone(),
            ) {
                Ok(run_id) => {
                    HttpResponse::Ok().json(TrainingValidateResponse {
                        success: true,
                        run_id: run_id.to_string(),
                        metrics,
                    })
                }
                Err(e) => {
                    eprintln!("❌ Failed to write training result: {}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "success": false,
                        "error": format!("Failed to write training result: {}", e),
                    }))
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize training datalake: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to initialize training datalake: {}", e),
            }))
        }
    }
}

async fn training_metrics() -> HttpResponse {
    let datalake_path = "./data/training_datalake";
    
    match crate::TrainingDataLake::new(datalake_path) {
        Ok(datalake) => {
            // Calculate aggregated metrics
            match crate::calculate_aggregate_metrics(&datalake) {
                Ok(aggregated) => {
                    // Calculate language metrics
                    let lang_metrics = crate::calculate_metrics_by_language(&datalake)
                        .ok()
                        .and_then(|m| if m.is_empty() { None } else { Some(m) });
                    
                    HttpResponse::Ok().json(TrainingMetricsResponse {
                        success: true,
                        aggregated_metrics: aggregated,
                        language_metrics: lang_metrics,
                    })
                }
                Err(e) => {
                    eprintln!("❌ Failed to calculate aggregated metrics: {}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "success": false,
                        "error": format!("Failed to calculate aggregated metrics: {}", e),
                    }))
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize training datalake: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to initialize training datalake: {}", e),
            }))
        }
    }
}
