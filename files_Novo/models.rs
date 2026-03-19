// src/models.rs — Modelos de dados (clientes, pedidos, replicação)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Cliente ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cliente {
    pub id: String,
    pub nome: String,
    pub email: String,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriarClienteDto {
    pub nome: String,
    pub email: String,
}

// ─── Pedido ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pedido {
    pub id: String,
    pub cliente_id: String,
    pub descricao: String,
    pub valor: f64,
    pub status: PedidoStatus,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PedidoStatus {
    Pendente,
    Confirmado,
    Cancelado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriarPedidoDto {
    pub cliente_id: String,
    pub descricao: String,
    pub valor: f64,
}

// ─── Replicação ───────────────────────────────────────────────────────────────

/// Payload enviado do primário ao backup para replicação síncrona.
/// O campo `operacao` define o tipo de escrita; `dados` carrega o objeto completo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicacaoPayload {
    pub operacao: String,          // "criar_cliente" | "criar_pedido"
    pub dados: serde_json::Value,  // objeto já persistido no primário
}

/// ACK retornado pelo backup após persistência local.
#[derive(Debug, Serialize, Deserialize)]
pub struct AckResposta {
    pub aceito: bool,
    pub mensagem: String,
}

// ─── Resposta genérica da API ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResposta<T: Serialize> {
    pub sucesso: bool,
    pub dados: Option<T>,
    pub mensagem: String,
    pub no: String,   // identifica qual nó respondeu (primário | backup)
}

impl<T: Serialize> ApiResposta<T> {
    pub fn ok(dados: T, mensagem: &str, no: &str) -> Self {
        Self {
            sucesso: true,
            dados: Some(dados),
            mensagem: mensagem.to_string(),
            no: no.to_string(),
        }
    }

    pub fn erro(mensagem: &str, no: &str) -> ApiResposta<()> {
        ApiResposta {
            sucesso: false,
            dados: None,
            mensagem: mensagem.to_string(),
            no: no.to_string(),
        }
    }
}
