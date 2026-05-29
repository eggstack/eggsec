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
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/slapper/Cargo.toml crates/slapper/
COPY crates/slapper-nse/Cargo.toml crates/slapper-nse/

# Create dummy sources to cache dependencies
RUN mkdir -p crates/slapper/src crates/slapper-nse/src && \
    echo "fn main() {}" > crates/slapper/src/main.rs && \
    echo "" > crates/slapper/src/lib.rs && \
    echo "" > crates/slapper-nse/src/lib.rs && \
    cargo build -p slapper --release --features full && \
    rm -rf crates/*/src

# Copy source code
COPY crates/slapper/src crates/slapper/src
COPY crates/slapper-nse/src crates/slapper-nse/src
COPY crates/slapper/build.rs crates/slapper/

# Build the application with all features
RUN cargo build -p slapper --release --features full

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
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
LABEL org.opencontainers.image.source="https://github.com/dbowm91/slapper"
LABEL org.opencontainers.image.authors="Slapper Team"
