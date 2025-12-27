# BUILD
FROM rust:1.92-slim-trixie as builder

RUN apt-get update && apt-get install -y pkg-config libfuse3-dev

WORKDIR /usr/src/app


RUN cargo new --bin mc-anvil-db
WORKDIR /usr/src/app/mc-anvil-db

COPY ./Cargo.toml ./Cargo.lock ./

RUN cargo build --release

RUN rm src/*.rs

COPY ./src ./src


# touch is needed to update the file timestamp and force cargo to rebuild it
RUN touch src/main.rs
RUN rm ./target/release/deps/mc_anvil_db*
RUN cargo build --release

# RUNTIME
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y fuse3 tini ca-certificates && rm -rf /var/lib/apt/lists/*
RUN sed -i 's/#user_allow_other/user_allow_other/' /etc/fuse.conf

COPY --from=builder /usr/src/app/mc-anvil-db/target/release/mc-anvil-db /usr/local/bin/mc-anvil-db

COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENV RUST_LOG=info

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
