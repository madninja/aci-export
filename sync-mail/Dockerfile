# Build stage
FROM rust:1.91-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release --bin sync-mail

# Runtime stage
FROM gcr.io/distroless/static-debian12
COPY --from=builder /app/target/release/sync-mail /usr/local/bin/
CMD ["sync-mail", "run"]
