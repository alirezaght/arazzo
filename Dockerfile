FROM rust:latest AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY arazzo-core/Cargo.toml ./arazzo-core/
COPY arazzo-exec/Cargo.toml ./arazzo-exec/
COPY arazzo-store/Cargo.toml ./arazzo-store/
COPY arazzo-cli/Cargo.toml ./arazzo-cli/

COPY arazzo-core/src ./arazzo-core/src
COPY arazzo-exec/src ./arazzo-exec/src
COPY arazzo-store/src ./arazzo-store/src
COPY arazzo-store/postgres ./arazzo-store/postgres
COPY arazzo-cli/src ./arazzo-cli/src

RUN cargo build --release --bin arazzo-cli && \
    strip /build/target/release/arazzo-cli

FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder --chown=nonroot:nonroot /build/target/release/arazzo-cli /usr/local/bin/arazzo

ENTRYPOINT ["/usr/local/bin/arazzo"]
