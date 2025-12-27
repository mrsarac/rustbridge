# =============================================================================
# RustBridge Production Dockerfile
# Multi-stage build for minimal image size (~25MB)
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build
# -----------------------------------------------------------------------------
FROM rust:1.92-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies (including libudev for serial/RTU support)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source for dependency compilation
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release && \
    rm -rf src target/release/deps/rustbridge*

# Copy actual source code
COPY src ./src
COPY tests ./tests

# Build the application with optimizations
RUN cargo build --release --locked

# Strip the binary for smaller size (~70% reduction)
RUN strip target/release/rustbridge

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libudev1 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -r -s /bin/false -u 1000 rustbridge

# Copy binary from builder
COPY --from=builder /app/target/release/rustbridge /app/rustbridge

# Copy default config
COPY config.yaml /app/config.yaml

# Set ownership
RUN chown -R rustbridge:rustbridge /app

# Switch to non-root user
USER rustbridge

# Expose ports
# 3000 - HTTP API + WebSocket
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:3000/health || exit 1

# Environment variables
ENV RUST_LOG=info
ENV RUSTBRIDGE_CONFIG=/app/config.yaml

# Labels
LABEL org.opencontainers.image.title="RustBridge"
LABEL org.opencontainers.image.description="Industrial Protocol Bridge - Modbus TCP/RTU to JSON/MQTT Gateway"
LABEL org.opencontainers.image.source="https://github.com/mrsarac/rustbridge"
LABEL org.opencontainers.image.licenses="MIT"

# Run the application
CMD ["/app/rustbridge"]
