FROM rust:1.79 AS builder

RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    musl-dev \
    libssl-dev \
    libarchive-dev \
    libpq-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY components /workspace/components
COPY Cargo.toml Cargo.lock /workspace/

WORKDIR /workspace/components/builder-api/

RUN cargo fetch --manifest-path=/workspace/Cargo.toml
RUN cargo build --release --bin bldr-api --manifest-path=/workspace/Cargo.toml

# RUN ls -lah /workspace/target/release/

RUN strip /workspace/target/release/bldr-api

FROM alpine:latest

RUN apk add --no-cache libpq

COPY --from=builder /workspace/target/release/bldr-api /usr/local/bin/builder-api

RUN addgroup -S builder && adduser -S builder -G builder
USER builder

WORKDIR /home/builder

EXPOSE 9636

ENTRYPOINT ["/usr/local/bin/builder-api", "start", "--config", "/etc/builder-api.toml"]