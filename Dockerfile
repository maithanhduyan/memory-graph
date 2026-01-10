# ============================================================================
# Memory Graph MCP Server - Multi-stage Docker Build
# ============================================================================
# Usage:
#   docker build -t memory-graph .
#   docker run -v $(pwd)/data:/data -e MEMORY_FILE_PATH=/data/memory.jsonl memory-graph
# ============================================================================

# Stage 1: Build
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy all source files
COPY Cargo.toml Cargo.lock* memory.rs ./

# Build for release
RUN cargo build --release

# Stage 2: Runtime (minimal image)
FROM alpine:3.19 AS runtime

# Add labels for GitHub Container Registry
LABEL org.opencontainers.image.source="https://github.com/maithanhduyan/memory-graph"
LABEL org.opencontainers.image.description="A blazing-fast Knowledge Graph server for AI Agents"
LABEL org.opencontainers.image.licenses="MIT"

# Create non-root user for security
RUN addgroup -g 1000 memory && \
    adduser -u 1000 -G memory -s /bin/sh -D memory

# Create data directory
RUN mkdir -p /data && chown memory:memory /data

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/memory-server /app/memory-server

# Set ownership
RUN chown memory:memory /app/memory-server

# Switch to non-root user
USER memory

# Environment variables
ENV MEMORY_FILE_PATH=/data/memory.jsonl

# Health check (optional - for orchestration)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD pgrep memory-server || exit 1

# The server uses stdio, so we just run it
ENTRYPOINT ["/app/memory-server"]
