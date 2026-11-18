# Eggsec - Security Testing Toolkit
# Multi-stage Docker build

# Build stage
FROM rust:1.89-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpcap-dev \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the complete workspace. Cargo resolves every workspace member, so a
# hand-maintained subset of manifests is not sufficient for a reliable build.
COPY . .

# Build the CLI with the existing curated full feature aggregate
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
