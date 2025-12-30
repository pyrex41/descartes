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
COPY daemon ./daemon

# Build daemon binary in release mode
RUN cargo build --release -p descartes-daemon

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/descartes-daemon /app/

# Copy migrations
COPY daemon/migrations /app/migrations

# Set environment variables
ENV RUST_LOG=info
EXPOSE 8080 8081

# Run the daemon
CMD ["/app/descartes-daemon", "--host", "0.0.0.0"]
