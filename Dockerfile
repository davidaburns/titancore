FROM rust:1.90-slim-bookworm AS builder
ARG SERVER_NAME=server

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

RUN cargo build --release --bin ${SERVER_NAME}

FROM debian:bookworm-slim
ARG SERVER_NAME=server
ARG PORT=8080

RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 appuser
WORKDIR /app

COPY --from=builder /workspace/target/release/${SERVER_NAME} /app/server

RUN chown -R appuser:appuser /app
USER appuser

EXPOSE ${PORT}
CMD ["/app/server"]
