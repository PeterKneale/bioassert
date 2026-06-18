FROM rust:1.96-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/bioassert-core/Cargo.toml crates/bioassert-core/Cargo.toml
COPY crates/bioassert-file/Cargo.toml crates/bioassert-file/Cargo.toml
COPY crates/bioassert-delimited/Cargo.toml crates/bioassert-delimited/Cargo.toml
COPY crates/bioassert-engine/Cargo.toml crates/bioassert-engine/Cargo.toml

RUN mkdir -p src crates/bioassert-core/src crates/bioassert-file/src \
        crates/bioassert-delimited/src crates/bioassert-engine/src \
    && echo "fn main() {}" > src/main.rs \
    && for d in crates/*/src; do echo "" > $d/lib.rs; done \
    && cargo build --release \
    && rm -rf src crates/*/src

COPY src ./src
COPY crates ./crates
RUN touch src/main.rs crates/*/src/*.rs && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bioassert /usr/local/bin/bioassert

ENTRYPOINT ["bioassert"]
