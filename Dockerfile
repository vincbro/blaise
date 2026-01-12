# BUILD
FROM rust:1.92-bullseye AS builder
WORKDIR /usr/src/blaise

# METADATA
LABEL org.opencontainers.image.title="Blaise Server"
LABEL org.opencontainers.image.description="An easy-to-use, fully local engine for public transit data with a strong focus on performance."
LABEL org.opencontainers.image.authors="Vincent Brodin"
LABEL org.opencontainers.image.source="https://github.com/vincbro/blaise"
LABEL org.opencontainers.image.licenses="MIT"

# Copy lib
COPY Cargo.toml Cargo.lock ./
COPY src ./src 
COPY benches ./benches 

# Copy server
RUN mkdir -p crates/server
COPY crates/server/Cargo.toml crates/server/start_logo.txt ./crates/server/
COPY crates/server/src ./crates/server/src

# Build
RUN cargo build -r -p server

# RUNTIME

FROM debian:bookworm-slim

# Install CA certificates
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy build output
COPY --from=builder /usr/src/blaise/target/release/blaise-server /usr/local/bin/

EXPOSE 3000

# ENV
ENV GTFS_DATA_PATH=/app/GTFS.zip

ENTRYPOINT ["blaise-server"]
