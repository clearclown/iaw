# Aether (ajj) - Multi-stage build using Alpine for static musl build
FROM docker.io/library/rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy project files (without Cargo.lock for compatibility)
COPY Cargo.toml ./
COPY src ./src
COPY tests ./tests

# Build the binary (statically linked with musl)
RUN cargo build --release

# Runtime stage - minimal Alpine
FROM docker.io/library/alpine:latest

RUN apk add --no-cache \
    ca-certificates \
    curl

# Install jujutsu (jj)
RUN curl -sSL https://github.com/martinvonz/jj/releases/download/v0.23.0/jj-v0.23.0-x86_64-unknown-linux-musl.tar.gz | \
    tar -xzf - -C /usr/local/bin

COPY --from=builder /app/target/release/ajj /usr/local/bin/ajj

# Create non-root user
RUN adduser -D -s /bin/sh aether
USER aether
WORKDIR /home/aether

ENTRYPOINT ["ajj"]
CMD ["--help"]
