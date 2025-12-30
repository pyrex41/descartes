# Build stage
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files needed for build
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY worker ./worker

# Build worker binary in release mode
RUN cargo build --release -p descartes-worker

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/descartes-worker /app/

# Set environment variables
ENV RUST_LOG=info

# The worker expects --task argument and environment variables:
# CALLBACK_URL, PROJECT_ID, ANTHROPIC_API_KEY (optional)
ENTRYPOINT ["/app/descartes-worker"]
