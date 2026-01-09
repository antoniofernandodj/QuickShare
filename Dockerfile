# =========================
# Stage 1: Build
# =========================
FROM rust:1.78-slim AS builder

# Adaptar Rust para nightly
RUN rustup toolchain install nightly && rustup default nightly

# Dependências de build
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar apenas o manifest (cache de deps)
COPY Cargo.toml ./

# Build dummy para cachear dependências
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copiar código real
COPY src ./src

# Build final
RUN cargo build --release


# =========================
# Stage 2: Runtime
# =========================
FROM debian:bookworm-slim

# Dependências mínimas de runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Usuário não-root
RUN useradd -m -u 1001 appuser
WORKDIR /app

# Copiar apenas o binário final
COPY --from=builder /app/target/release/quickshare /app/quickshare

# Permissões
RUN chown -R appuser:appuser /app
USER appuser

EXPOSE 8000
CMD ["/app/quickshare"]
