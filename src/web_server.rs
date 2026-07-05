//! src/web_server.rs
//! Simple HTTP server serving the contract graph visualization

use actix_web::{web, App, HttpServer, HttpResponse, middleware};
use actix_files::Files;
use std::sync::Arc;
use crate::ContractLedger;
use serde_json::json;

pub struct WebServerConfig {
    pub port: u16,
    pub host: String,
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
            )
            .service(Files::new("/", "./ui/dist").index_file("index.html"))
    })
    .bind(&addr)?
    .run()
    .await
}

async fn get_graph(_ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "nodes": [],
        "edges": []
    }))
}

async fn get_signatories(_ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "signatories": []
    }))
}

async fn get_contracts(_ledger: web::Data<Arc<ContractLedger>>) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "contracts": []
    }))
}

async fn get_chain(
    id: web::Path<String>,
    _ledger: web::Data<Arc<ContractLedger>>,
) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "chain": [],
        "root": id.into_inner()
    }))
}
