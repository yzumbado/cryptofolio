# Multi-stage build for smaller final image
FROM rust:1.75-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY tests ./tests

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 cryptofolio

# Copy binary from builder
COPY --from=builder /app/target/release/cryptofolio /usr/local/bin/cryptofolio

# Set ownership
RUN chown cryptofolio:cryptofolio /usr/local/bin/cryptofolio

# Switch to non-root user
USER cryptofolio

# Create data directory
RUN mkdir -p /home/cryptofolio/.cryptofolio

# Set working directory
WORKDIR /home/cryptofolio

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/cryptofolio"]

# Default command (show help)
CMD ["--help"]
