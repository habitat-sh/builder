# WARNING: This image is not meant for production use.
# It is for testing purposes only. Do not use in a production environment.

# Stage 1: Build the Rust binary
FROM rust:1.79.0 AS builder

WORKDIR /src

RUN apt update && apt install -y --no-install-recommends
RUN update-ca-certificates
# RUN rustup update && rustup target add aarch64-unknown-linux-musl

RUN apt-get update && apt-get install -y \
    libarchive-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy the source code
COPY . .

# Build the project
RUN cd components/builder-api && cargo build --target-dir /src/target --verbose --release

RUN strip /src/target/release/bldr-api

# Stage 2: Create a minimal image with the Rust binary
FROM debian:bookworm-slim AS final

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libc6-dev \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create the /app directory to store the binary and config
RUN mkdir -p /app

COPY --from=builder /src/target/release/bldr-api /app/bldr-api

# Ensure the config file is included, adjust path if needed
COPY --from=builder /src/config/config.toml /app/config/config.toml

# Run the compiled static binary
ENTRYPOINT ["/app/bldr-api", "start", "-c", "/app/config/config.toml"]
