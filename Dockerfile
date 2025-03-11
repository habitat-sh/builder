# WARNING: This image is not meant for production use.
# It is for testing purposes only. Do not use in a production environment.

# Stage 1: Build the Rust binary
FROM rust:latest AS builder

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
RUN cd components/builder-api && cargo build --target-dir /src/target --verbose

# Run the Habitat install script
RUN curl https://raw.githubusercontent.com/habitat-sh/habitat/main/components/hab/install.sh | bash

# Verify Habitat installation (optional)
RUN hab --version

ENV HAB_LICENSE=accept-no-persist
RUN hab user key generate bldr

# Stage 2: Create a minimal image with the Rust binary
FROM rust:latest as final

RUN apt-get update && apt-get install -y \
    libssl-dev \
    libpq5 \
    libc6 \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create the /app directory to store the binary and config
RUN mkdir -p /app

COPY --from=builder /src/target/debug/bldr-api /app/bldr-api

# Ensure the config file is included, adjust path if needed
COPY --from=builder /src/config/config.toml /app/config/config.toml

# Copy the bldr user keys
COPY --from=builder /hab/cache/keys /app/keys

# Run the compiled static binary
ENTRYPOINT ["/app/bldr-api", "start", "-c", "/app/config/config.toml"]
