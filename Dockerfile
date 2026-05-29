# ── Build Stage ──
FROM --platform=$BUILDPLATFORM rust:1.89 AS builder

ARG TARGETARCH

RUN if [ "$TARGETARCH" = "arm64" ]; then \
      apt-get update && \
      apt-get install -y gcc-aarch64-linux-gnu && \
      ln -s /usr/bin/aarch64-linux-gnu-gcc /usr/bin/aarch64-linux-musl-gcc && \
      rustup target add aarch64-unknown-linux-musl; \
    else \
      apt-get update && \
      apt-get install -y musl-tools && \
      rustup target add x86_64-unknown-linux-musl; \
    fi && \
    rm -rf /var/lib/apt/lists/*

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc
ENV CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN if [ "$TARGETARCH" = "arm64" ]; then \
      cargo build --release --target aarch64-unknown-linux-musl; \
    else \
      cargo build --release --target x86_64-unknown-linux-musl; \
    fi
RUN rm -rf src

COPY src/ ./src/
COPY migrations/ ./migrations/
RUN if [ "$TARGETARCH" = "arm64" ]; then \
      cargo build --release --target aarch64-unknown-linux-musl && \
      cp /app/target/aarch64-unknown-linux-musl/release/inkle /app/inkle_bin; \
    else \
      cargo build --release --target x86_64-unknown-linux-musl && \
      cp /app/target/x86_64-unknown-linux-musl/release/inkle /app/inkle_bin; \
    fi

# ── Runtime Stage ──
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/inkle_bin /app/inkle
COPY admin_templates/ ./admin_templates/
COPY static/ ./static/
COPY migrations/ ./migrations/

EXPOSE 3000

ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc

CMD ["./inkle"]