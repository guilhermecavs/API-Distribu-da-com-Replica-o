# Dockerfile — Build multi-stage para imagem mínima de produção

# ── Estágio 1: compilação ──────────────────────────────────────────────────────
FROM rust:1.78-slim AS builder

WORKDIR /app

# Copia manifests primeiro (aproveita cache do Docker se o código não mudou)
COPY Cargo.toml Cargo.lock* ./

# Cria um main.rs falso para pré-compilar as dependências
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copia o código real e recompila apenas o binário da aplicação
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ── Estágio 2: imagem final mínima ────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copia apenas o binário compilado
COPY --from=builder /app/target/release/servidor .

# Variáveis padrão (sobrescritas no docker-compose.yml)
ENV NODE_ROLE=primario
ENV PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

CMD ["./servidor"]
