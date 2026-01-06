FROM rust:1.92-bullseye AS builder
WORKDIR /usr/src/ontrack

COPY Cargo.toml Cargo.lock ./
COPY src ./src 

RUN mkdir -p crates/server
COPY crates/server/Cargo.toml crates/server/start_logo.txt ./crates/server/
COPY crates/server/src ./crates/server/src

RUN cargo build -r -p server

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /usr/src/ontrack/target/release/ontrack-server /usr/local/bin/

EXPOSE 3000

ENTRYPOINT ["ontrack-server"]
