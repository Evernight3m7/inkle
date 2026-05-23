# ── Build Stage ──
FROM rust:1.89-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ ./migrations/
RUN cargo build --release

# ── Runtime Stage ──
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/inkle /app/inkle
COPY admin_templates/ ./admin_templates/
COPY static/ ./static/
COPY migrations/ ./migrations/

EXPOSE 3000

ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc

CMD ["./inkle"]
