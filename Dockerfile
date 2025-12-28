# syntax=docker/dockerfile:1
# BUILD
FROM rust:1.92-slim-trixie as builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y pkg-config libfuse3-dev git

WORKDIR /usr/src/app/mc-anvil-db

COPY ./ ./

# Use Cache Mounts to speed up builds
# cache/registry: crates.io index and sources
# cache/git: git repositories
# target: compiled artifacts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/app/mc-anvil-db/target \
    cargo build --release && \
    cp target/release/mc-anvil-db /usr/local/bin/mc-anvil-db

# RUNTIME
FROM debian:trixie-slim

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y fuse3 tini ca-certificates && rm -rf /var/lib/apt/lists/*
RUN sed -i 's/#user_allow_other/user_allow_other/' /etc/fuse.conf

COPY --from=builder /usr/local/bin/mc-anvil-db /usr/local/bin/mc-anvil-db

COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENV RUST_LOG=info

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
