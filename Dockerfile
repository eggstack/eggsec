# Slapper - Security Testing Toolkit
# Multi-stage Docker build

# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpcap-dev \
    python3-dev \
    ruby-dev \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --features full && \
    rm -rf src

# Copy source code
COPY src ./src

# Build the application with all features
RUN cargo build --release --features full

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    python3 \
    ruby \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash slapper

# Copy binary from builder
COPY --from=builder /app/target/release/slapper /usr/local/bin/slapper

# Create directories for config and data
RUN mkdir -p /home/slapper/.config/slapper \
    /home/slapper/.local/share/slapper \
    /home/slapper/.cache/slapper \
    && chown -R slapper:slapper /home/slapper

# Copy example configurations
COPY examples/configs/*.toml /home/slapper/.config/slapper/

# Set environment
ENV SLAPPER_CONFIG_DIR=/home/slapper/.config/slapper
ENV SLAPPER_DATA_DIR=/home/slapper/.local/share/slapper

USER slapper

# Default command
ENTRYPOINT ["slapper"]
CMD ["--help"]

# Labels
LABEL org.opencontainers.image.title="Slapper"
LABEL org.opencontainers.image.description="Security Testing Toolkit"
LABEL org.opencontainers.image.source="https://github.com/slapper-tool/slapper"
LABEL org.opencontainers.image.authors="Slapper Team"
