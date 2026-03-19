// src/main.rs — Ponto de entrada do servidor distribuído
//
// Variáveis de ambiente:
//   NODE_ROLE   → "primario" (padrão) ou "backup"
//   PORT        → porta HTTP (padrão: 8080)
//   BACKUP_URL  → URL do backup, ex: http://backup:8081  (só necessário no primário)

use axum::{
    routing::{get, post},
    Router,
};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod models;
mod storage;
mod sync;

/// Estado global compartilhado entre todos os handlers via Arc<AppState>.
pub struct AppState {
    /// Armazenamento em memória (protegido por RwLock para acesso concorrente seguro)
    pub storage: RwLock<storage::Storage>,
    /// Papel deste nó: "primario" ou "backup"
    pub role: String,
    /// URL do nó backup (ex: "http://backup:8081") — None se este nó for o backup
    pub backup_url: Option<String>,
    /// Flag de disponibilidade — false simula uma falha para testes de failover
    pub alive: RwLock<bool>,
}

#[tokio::main]
async fn main() {
    // Inicializa o sistema de logs
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Lê configuração das variáveis de ambiente
    let role       = env::var("NODE_ROLE").unwrap_or_else(|_| "primario".to_string());
    let port: u16  = env::var("PORT").unwrap_or_else(|_| "8080".to_string())
                        .parse().expect("PORT deve ser um número válido");
    let backup_url = env::var("BACKUP_URL").ok();

    tracing::info!("╔══════════════════════════════════════════╗");
    tracing::info!("║  Sistema Distribuído — Rust + MPI        ║");
    tracing::info!("╠══════════════════════════════════════════╣");
    tracing::info!("║  Nó     : {:<31}║", role);
    tracing::info!("║  Porta  : {:<31}║", port);
    tracing::info!("║  Backup : {:<31}║", backup_url.as_deref().unwrap_or("N/A"));
    tracing::info!("╚══════════════════════════════════════════╝");

    let state = Arc::new(AppState {
        storage:    RwLock::new(storage::Storage::new()),
        role:       role.clone(),
        backup_url,
        alive:      RwLock::new(true),
    });

    // Define todas as rotas da aplicação
    let app = Router::new()
        .route("/health",           get(api::health))
        .route("/clientes",         get(api::listar_clientes).post(api::criar_cliente))
        .route("/pedidos",          get(api::listar_pedidos).post(api::criar_pedido))
        .route("/interno/replicar", post(api::replicar))
        .route("/admin/falha",      post(api::simular_falha))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Servidor {} escutando em http://0.0.0.0:{}", role, port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
