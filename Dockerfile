# Build stage
FROM rust:1.87-slim AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && \
    echo "fn main() {println!(\"Dummy build\");}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/padenc_api* target/release/padenc_api*

COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl-dev && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/padenc_api /app/
RUN mkdir -p /data

EXPOSE 8080

CMD ["/app/padenc_api"]
