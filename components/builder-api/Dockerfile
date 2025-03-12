FROM rust:1.79.0 as builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libarchive-dev \
    musl-tools \
    musl-dev \
    g++ \
    libssl-dev \
    cmake \
    make \
    curl \
    wget \
    protobuf-compiler \
    libpq-dev \
    libsasl2-dev \
    build-essential \
    g++-multilib \
    gcc-multilib \
    libpq5 \
    libgssapi-krb5-2 \
    libkrb5-dev \
    libldap2-dev \
    libnss-wrapper \
    openssl \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN curl -LO https://musl.cc/x86_64-linux-musl-cross.tgz \
    && tar -xvzf x86_64-linux-musl-cross.tgz -C /usr/local \
    && rm x86_64-linux-musl-cross.tgz

RUN ln -sf /usr/local/x86_64-linux-musl-cross/bin/x86_64-linux-musl-gcc /usr/bin/musl-gcc \
    && ln -sf /usr/local/x86_64-linux-musl-cross/bin/x86_64-linux-musl-g++ /usr/bin/musl-g++

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /workspace

COPY components /workspace/components
COPY Cargo.toml Cargo.lock /workspace/

RUN if ! command -v protoc; then echo "Error: protoc not installed" && exit 1; fi

RUN cargo fetch

ENV PATH="/usr/local/x86_64-linux-musl-cross/bin:$PATH"
ENV CC=musl-gcc
ENV CXX=musl-g++
ENV RUSTFLAGS="-C target-feature=+crt-static \
    -C link-arg=-L/usr/lib/x86_64-linux-gnu \
    -C link-arg=-lpq \
    -C link-arg=-lz \
    -C link-arg=-lgssapi_krb5 \
    -C link-arg=-lssl \
    -C link-arg=-lcrypto \
    -C link-arg=-lldap \
    -C link-arg=-Wl,--no-as-needed \
    -C link-arg=-Wl,--copy-dt-needed-entries"

ENV OPENSSL_STATIC=1
ENV LIBZMQ_STATIC=1
ENV SQLITE3_STATIC=1
ENV PQ_LIB_STATIC=1
ENV OPENSSL_DIR="/usr"
ENV OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu"
ENV OPENSSL_INCLUDE_DIR="/usr/include"
ENV PKG_CONFIG_SYSROOT_DIR=/usr
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_ALLOW_STATIC=1
ENV PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig"

RUN echo "/usr/local/x86_64-linux-musl-cross/lib" > /etc/ld.so.conf.d/musl.conf \
    echo "/usr/lib/x86_64-linux-gnu" > /etc/ld.so.conf.d/postgresql.conf \
    && ldconfig

RUN mkdir -p ~/.cargo && \
    echo "[target.x86_64-unknown-linux-musl]" >> ~/.cargo/config && \
    echo "rustflags = [\"-C\", \"target-feature=+crt-static\", \"-C\", \"link-arg=-L/usr/lib/x86_64-linux-gnu\", \"-C\", \"link-arg=-lpq\", \"-C\", \"link-arg=-lz\", \"-C\", \"link-arg=-lgssapi_krb5\"]" >> ~/.cargo/config


RUN ldd /usr/lib/x86_64-linux-gnu/libpq.so && \
ldd /usr/lib/x86_64-linux-gnu/libssl.so* && \
ldd /usr/lib/x86_64-linux-gnu/libcrypto.so* && \
ldd /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so

RUN ln -s /usr/lib/x86_64-linux-gnu/libcrypto.so.3 /usr/lib/libcrypto.so && \
    ln -s /usr/lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/libssl.so

WORKDIR /workspace/components/builder-api
RUN cargo build --release --target x86_64-unknown-linux-musl --bin bldr-api \
    && file /workspace/target/x86_64-unknown-linux-musl/release/bldr-api \
    && strip /workspace/target/x86_64-unknown-linux-musl/release/bldr-api

FROM scratch

COPY --from=builder /workspace/target/x86_64-unknown-linux-musl/release/bldr-api /builder-api

EXPOSE 9636

ENTRYPOINT ["/builder-api"]
CMD ["start", "--config", "/etc/builder-api.toml"]