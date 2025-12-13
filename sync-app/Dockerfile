# Build stage
FROM rust:1.91 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin sync-app

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/sync-app /usr/local/bin/
CMD ["sync-app", "run"]
