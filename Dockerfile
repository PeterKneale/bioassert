# syntax=docker/dockerfile:1

# Multi-stage build producing a statically linked (musl) binary on a minimal scratch base.
# Builds natively per target platform under Buildx/QEMU (amd64 + arm64).

FROM rust:1.96-alpine AS builder

# musl-dev provides the static C runtime/linker bits cargo needs on Alpine.
RUN apk add --no-cache musl-dev

WORKDIR /src
COPY . .

# musl targets link the C runtime statically by default; set it explicitly for clarity.
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --locked --release

# Final image: just the static binary on scratch (no shell, no libc).
FROM scratch

# OCI labels are injected at build time by docker/metadata-action in CI.
COPY --from=builder /src/target/release/bioassert /usr/local/bin/bioassert

ENTRYPOINT ["/usr/local/bin/bioassert"]

