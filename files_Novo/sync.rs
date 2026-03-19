// src/sync.rs — Consistência Forte: primário aguarda ACK do backup antes de confirmar.
//
// Implementação do modelo Primário-Backup com replicação síncrona.
// O primário NUNCA responde ao cliente antes de receber confirmação do backup,
// garantindo que ambos os nós sempre tenham os mesmos dados (consistência forte).

use crate::models::{AckResposta, ReplicacaoPayload};
use reqwest::Client;
use std::time::Duration;

/// Tempo máximo de espera pelo ACK do backup.
const TIMEOUT_REPLICACAO: Duration = Duration::from_secs(3);

/// Envia o payload para o backup e aguarda o ACK de forma síncrona.
///
/// # Retorno
/// - `Ok(())` → backup confirmou a replicação
/// - `Err(String)` → timeout, backup offline ou recusou a operação
pub async fn replicar_para_backup(
    backup_url: &str,
    payload: &ReplicacaoPayload,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(TIMEOUT_REPLICACAO)
        .build()
        .map_err(|e| format!("Erro ao criar cliente HTTP: {e}"))?;

    let url = format!("{}/interno/replicar", backup_url);

    let resposta = client
        .post(&url)
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Backup indisponível: {e}"))?;

    if !resposta.status().is_success() {
        return Err(format!(
            "Backup rejeitou a replicação (status {})",
            resposta.status()
        ));
    }

    let ack: AckResposta = resposta
        .json()
        .await
        .map_err(|e| format!("Resposta inválida do backup: {e}"))?;

    if ack.aceito {
        tracing::info!("[SYNC] ACK recebido do backup: {}", ack.mensagem);
        Ok(())
    } else {
        Err(format!("Backup recusou operação: {}", ack.mensagem))
    }
}
