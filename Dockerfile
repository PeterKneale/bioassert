# syntax=docker/dockerfile:1

# Multi-stage build producing a statically linked (musl) binary on a minimal base.
# Builds natively per target platform under Buildx/QEMU (amd64 + arm64).

FROM rust:1.96-alpine AS builder

# musl-dev provides the static C runtime/linker bits cargo needs on Alpine.
RUN apk add --no-cache musl-dev

WORKDIR /src
COPY . .

RUN cargo build --locked --release

# Final image: just the static binary
FROM alpine

# bash required for tools such as nextflow that expect a POSIX shell with some common features.
RUN apk add --no-cache bash

# OCI labels are injected at build time by docker/metadata-action in CI.
COPY --from=builder /src/target/release/bioassert /usr/local/bin/bioassert

ENTRYPOINT ["/usr/local/bin/bioassert"]

