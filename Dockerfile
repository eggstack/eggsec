# Eggsec - Security Testing Toolkit
# Multi-stage Docker build

# Build stage
FROM rust:1.80-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpcap-dev \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/eggsec-core/Cargo.toml crates/eggsec-core/
COPY crates/eggsec/Cargo.toml crates/eggsec/
COPY crates/eggsec-nse/Cargo.toml crates/eggsec-nse/
COPY crates/eggsec-tui/Cargo.toml crates/eggsec-tui/
COPY crates/eggsec-cli/Cargo.toml crates/eggsec-cli/
COPY crates/eggsec-output/Cargo.toml crates/eggsec-output/
COPY crates/eggsec-tool-core/Cargo.toml crates/eggsec-tool-core/
COPY crates/eggsec-agent/Cargo.toml crates/eggsec-agent/

# Create dummy sources to cache dependencies
RUN mkdir -p crates/eggsec-core/src crates/eggsec/src crates/eggsec-nse/src \
    crates/eggsec-tui/src crates/eggsec-cli/src crates/eggsec-output/src \
    crates/eggsec-tool-core/src crates/eggsec-agent/src && \
    touch crates/eggsec-core/src/lib.rs crates/eggsec/src/lib.rs \
    crates/eggsec-nse/src/lib.rs crates/eggsec-tui/src/lib.rs \
    crates/eggsec-cli/src/main.rs crates/eggsec-output/src/lib.rs \
    crates/eggsec-tool-core/src/lib.rs crates/eggsec-agent/src/lib.rs && \
    cargo build -p eggsec-cli --release --features full && \
    rm -rf crates/*/src

# Copy source code
COPY crates/ crates/

# Build the application with all features
RUN cargo build -p eggsec-cli --release --features full

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash eggsec

# Copy binary from builder
COPY --from=builder /app/target/release/eggsec /usr/local/bin/eggsec

# Create directories for config and data
RUN mkdir -p /home/eggsec/.config/eggsec \
    /home/eggsec/.local/share/eggsec \
    /home/eggsec/.cache/eggsec \
    && chown -R eggsec:eggsec /home/eggsec

# Copy example configurations
COPY examples/configs/*.toml /home/eggsec/.config/eggsec/

# Set environment
ENV EGGSEC_CONFIG_DIR=/home/eggsec/.config/eggsec
ENV EGGSEC_DATA_DIR=/home/eggsec/.local/share/eggsec

USER eggsec

# Default command
ENTRYPOINT ["eggsec"]
CMD ["--help"]

# Labels
LABEL org.opencontainers.image.title="Eggsec"
LABEL org.opencontainers.image.description="Security Testing Toolkit"
LABEL org.opencontainers.image.source="https://github.com/eggstack/eggsec"
LABEL org.opencontainers.image.authors="Eggsec Team"
