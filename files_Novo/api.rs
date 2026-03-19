// src/api.rs — Handlers HTTP (axum 0.7)
//
// Rotas expostas:
//   GET  /health              → health check (usado pelo Nginx para failover)
//   GET  /clientes            → listar todos os clientes
//   POST /clientes            → criar cliente  (com replicação síncrona)
//   GET  /pedidos             → listar todos os pedidos
//   POST /pedidos             → criar pedido   (com replicação síncrona)
//   POST /interno/replicar    → endpoint interno (backup recebe dados do primário)
//   POST /admin/falha         → simula falha do nó primário (teste de failover)

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    models::*,
    storage::Storage,
    sync::replicar_para_backup,
    AppState,
};

// ─── Health Check ─────────────────────────────────────────────────────────────

/// Retorna 200 OK se o nó está vivo, 503 se foi simulada uma falha.
/// O Nginx usa este endpoint para decidir se deve redirecionar para o backup.
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let vivo = *state.alive.read().await;
    if vivo {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "no": state.role,
                "timestamp": Utc::now()
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "falha simulada",
                "no": state.role
            })),
        )
    }
}

// ─── Clientes ─────────────────────────────────────────────────────────────────

pub async fn listar_clientes(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let storage = state.storage.read().await;
    let clientes = storage.listar_clientes();
    Json(ApiResposta::ok(clientes, "OK", &state.role))
}

pub async fn criar_cliente(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<CriarClienteDto>,
) -> impl IntoResponse {
    // 1. Constrói o objeto completo
    let cliente = Cliente {
        id: Uuid::new_v4().to_string(),
        nome: dto.nome.clone(),
        email: dto.email.clone(),
        criado_em: Utc::now(),
    };

    // 2. Se for primário, replica de forma SÍNCRONA antes de confirmar
    if state.role == "primario" {
        if let Some(backup_url) = &state.backup_url {
            let payload = ReplicacaoPayload {
                operacao: "criar_cliente".to_string(),
                dados: serde_json::to_value(&cliente).unwrap(),
            };

            if let Err(e) = replicar_para_backup(backup_url, &payload).await {
                tracing::error!("[REPLICAÇÃO] Falhou: {}", e);
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::to_value(
                        ApiResposta::<()>::erro(
                            &format!("Replicação falhou — consistência não garantida: {e}"),
                            &state.role,
                        )
                    ).unwrap()),
                );
            }
        }
    }

    // 3. Persiste localmente (após ACK do backup)
    {
        let mut storage = state.storage.write().await;
        storage.inserir_cliente(cliente.clone());
    }

    tracing::info!("[CLIENTE] Criado: {} ({})", cliente.nome, cliente.id);
    (
        StatusCode::CREATED,
        Json(serde_json::to_value(ApiResposta::ok(cliente, "Cliente criado", &state.role)).unwrap()),
    )
}

// ─── Pedidos ──────────────────────────────────────────────────────────────────

pub async fn listar_pedidos(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let storage = state.storage.read().await;
    let pedidos = storage.listar_pedidos();
    Json(ApiResposta::ok(pedidos, "OK", &state.role))
}

pub async fn criar_pedido(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<CriarPedidoDto>,
) -> impl IntoResponse {
    // Valida se o cliente existe
    {
        let storage = state.storage.read().await;
        if storage.buscar_cliente(&dto.cliente_id).is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::to_value(
                    ApiResposta::<()>::erro("Cliente não encontrado", &state.role)
                ).unwrap()),
            );
        }
    }

    let pedido = Pedido {
        id: Uuid::new_v4().to_string(),
        cliente_id: dto.cliente_id.clone(),
        descricao: dto.descricao.clone(),
        valor: dto.valor,
        status: PedidoStatus::Pendente,
        criado_em: Utc::now(),
    };

    // Replicação síncrona (primário → backup)
    if state.role == "primario" {
        if let Some(backup_url) = &state.backup_url {
            let payload = ReplicacaoPayload {
                operacao: "criar_pedido".to_string(),
                dados: serde_json::to_value(&pedido).unwrap(),
            };

            if let Err(e) = replicar_para_backup(backup_url, &payload).await {
                tracing::error!("[REPLICAÇÃO] Falhou: {}", e);
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::to_value(
                        ApiResposta::<()>::erro(
                            &format!("Replicação falhou: {e}"),
                            &state.role,
                        )
                    ).unwrap()),
                );
            }
        }
    }

    {
        let mut storage = state.storage.write().await;
        storage.inserir_pedido(pedido.clone());
    }

    tracing::info!("[PEDIDO] Criado: {} — R${:.2}", pedido.id, pedido.valor);
    (
        StatusCode::CREATED,
        Json(serde_json::to_value(ApiResposta::ok(pedido, "Pedido criado", &state.role)).unwrap()),
    )
}

// ─── Replicação interna (backup recebe do primário) ───────────────────────────

/// Endpoint chamado apenas pelo primário.
/// O backup persiste localmente e retorna ACK.
pub async fn replicar(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReplicacaoPayload>,
) -> impl IntoResponse {
    tracing::info!("[BACKUP] Recebendo replicação: {}", payload.operacao);

    let resultado = match payload.operacao.as_str() {
        "criar_cliente" => {
            match serde_json::from_value::<Cliente>(payload.dados) {
                Ok(c) => {
                    state.storage.write().await.inserir_cliente(c);
                    Ok("Cliente replicado com sucesso")
                }
                Err(e) => Err(format!("Payload inválido: {e}")),
            }
        }
        "criar_pedido" => {
            match serde_json::from_value::<Pedido>(payload.dados) {
                Ok(p) => {
                    state.storage.write().await.inserir_pedido(p);
                    Ok("Pedido replicado com sucesso")
                }
                Err(e) => Err(format!("Payload inválido: {e}")),
            }
        }
        op => Err(format!("Operação desconhecida: {op}")),
    };

    match resultado {
        Ok(msg) => (
            StatusCode::OK,
            Json(AckResposta { aceito: true, mensagem: msg.to_string() }),
        ),
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(AckResposta { aceito: false, mensagem: msg }),
        ),
    }
}

// ─── Simulação de Falha ───────────────────────────────────────────────────────

/// Simula a falha do nó primário: passa a retornar 503 no /health.
/// O Nginx detecta via health check e redireciona tráfego para o backup.
pub async fn simular_falha(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut alive = state.alive.write().await;
    *alive = false;
    tracing::warn!("[ADMIN] Falha simulada! Nó {} ficará indisponível.", state.role);
    Json(serde_json::json!({
        "mensagem": "Falha simulada. O Nginx irá redirecionar para o backup.",
        "no": state.role
    }))
}
