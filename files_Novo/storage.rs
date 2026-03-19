// src/storage.rs — Armazenamento em memória com HashMap

use std::collections::HashMap;
use crate::models::{Cliente, Pedido};

/// Armazena todos os dados em memória.
/// Em produção, substituir por banco de dados (PostgreSQL, etc.).
#[derive(Debug, Default)]
pub struct Storage {
    pub clientes: HashMap<String, Cliente>,
    pub pedidos:  HashMap<String, Pedido>,
}

impl Storage {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Clientes ──────────────────────────────────────────────────────────────

    pub fn inserir_cliente(&mut self, cliente: Cliente) {
        self.clientes.insert(cliente.id.clone(), cliente);
    }

    pub fn listar_clientes(&self) -> Vec<Cliente> {
        let mut lista: Vec<Cliente> = self.clientes.values().cloned().collect();
        lista.sort_by(|a, b| a.criado_em.cmp(&b.criado_em));
        lista
    }

    pub fn buscar_cliente(&self, id: &str) -> Option<&Cliente> {
        self.clientes.get(id)
    }

    // ── Pedidos ───────────────────────────────────────────────────────────────

    pub fn inserir_pedido(&mut self, pedido: Pedido) {
        self.pedidos.insert(pedido.id.clone(), pedido);
    }

    pub fn listar_pedidos(&self) -> Vec<Pedido> {
        let mut lista: Vec<Pedido> = self.pedidos.values().cloned().collect();
        lista.sort_by(|a, b| a.criado_em.cmp(&b.criado_em));
        lista
    }
}
