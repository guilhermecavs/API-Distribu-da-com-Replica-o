# projeto-sd — API Distribuída com Replicação (Rust + Nginx)

> Trabalho Final — Sistemas Distribuídos  
> Universidade Federal de Uberlândia (UFU)

## Arquitetura

```
┌─────────┐        ┌──────────────────────────────────────────┐
│ Cliente │  :80   │              Nginx (Gateway)              │
│  HTTP   │───────►│  Roteia para primário; failover p/ backup │
└─────────┘        └───────────────┬──────────────────────────┘
                                   │ proxy_pass
                    ┌──────────────▼──────────────┐
                    │       Nó Primário :8080      │
                    │  - Recebe e valida requisição│
                    │  - Replica SINCRONAMENTE     │──── ACK ────┐
                    │  - Aguarda ACK do backup     │             │
                    │  - Persiste e responde       │   ┌─────────▼────────┐
                    └──────────────────────────────┘   │  Nó Backup :8081 │
                                                       │  - Recebe payload│
                                                       │  - Persiste local │
                                                       │  - Envia ACK      │
                                                       └──────────────────┘
```

**Consistência Forte**: o primário só confirma ao cliente após o backup persistir os dados (`src/sync.rs`).

## Requisitos

- [Docker](https://docs.docker.com/get-docker/) e Docker Compose
- **OU** Rust 1.78+ e `cargo` (para rodar sem Docker)

## Compilação e Execução

### Com Docker (recomendado)

```bash
# Clonar o repositório
git clone https://github.com/SEU_USUARIO/projeto-sd
cd projeto-sd

# Subir todos os serviços (Nginx + primário + backup)
docker compose up --build

# Em outro terminal, testar:
curl http://localhost/health
```

### Sem Docker (manual)

```bash
# Terminal 1 — Backup (inicie primeiro)
NODE_ROLE=backup PORT=8081 cargo run --release

# Terminal 2 — Primário
NODE_ROLE=primario PORT=8080 BACKUP_URL=http://localhost:8081 cargo run --release

# Terminal 3 — Nginx (precisa do nginx instalado)
nginx -c $(pwd)/nginx/nginx.conf
```

## Endpoints da API

| Método | Rota                  | Descrição                          |
|--------|-----------------------|------------------------------------|
| GET    | `/health`             | Status do nó                       |
| GET    | `/clientes`           | Lista todos os clientes            |
| POST   | `/clientes`           | Cria cliente (com replicação)      |
| GET    | `/pedidos`            | Lista todos os pedidos             |
| POST   | `/pedidos`            | Cria pedido (com replicação)       |
| POST   | `/admin/falha`        | **Simula falha** do nó primário    |
| POST   | `/interno/replicar`   | Uso interno (primário → backup)    |

## Exemplos de Uso

```bash
# Criar cliente
curl -X POST http://localhost/clientes \
  -H "Content-Type: application/json" \
  -d '{"nome": "João Silva", "email": "joao@email.com"}'

# Listar clientes
curl http://localhost/clientes

# Criar pedido (use o id retornado acima)
curl -X POST http://localhost/pedidos \
  -H "Content-Type: application/json" \
  -d '{"cliente_id": "<ID>", "descricao": "Notebook", "valor": 3500.00}'

# Simular falha do primário (Nginx redireciona para backup automaticamente)
curl -X POST http://localhost/admin/falha

# Verificar que o backup está respondendo
curl http://localhost/clientes
# Observe o campo "no": "backup" na resposta JSON
```

## Teste de Failover

```bash
# 1. Criar dados no primário
curl -X POST http://localhost/clientes \
  -d '{"nome":"Teste","email":"t@t.com"}' -H "Content-Type: application/json"

# 2. Simular falha
curl -X POST http://localhost/admin/falha

# 3. O Nginx detecta 503 no /health e redireciona para o backup
curl http://localhost/clientes
# → campo "no" mostra "backup", e os dados estão presentes (replicação funcionou)

# 4. Restaurar (reiniciar o contêiner do primário)
docker compose restart primario
```

## Estrutura do Projeto

```
projeto-sd/
├── src/
│   ├── main.rs          # Inicialização, roteamento e estado global
│   ├── api.rs           # Handlers HTTP (rotas)
│   ├── models.rs        # Structs: Cliente, Pedido, Replicação
│   ├── storage.rs       # Armazenamento em memória (HashMap)
│   └── sync.rs          # Consistência forte — espera ACK do backup
├── nginx/
│   └── nginx.conf       # Gateway com failover passivo
├── Dockerfile           # Build multi-stage (imagem mínima)
├── docker-compose.yml   # Orquestração dos 3 contêineres
└── README.md
```

## Conceitos Implementados

| Conceito | Onde |
|---|---|
| Consistência Forte | `src/sync.rs` — primário aguarda ACK antes de responder |
| Replicação Primário-Backup | `src/api.rs` + `src/sync.rs` |
| Tolerância a Falhas | `nginx.conf` — `proxy_next_upstream` + `backup` flag |
| Detecção de Falha | `/health` retorna 503 → Nginx redireciona automaticamente |
| Simulação de Falha | `POST /admin/falha` muda flag `alive` para false |
