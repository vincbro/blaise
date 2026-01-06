
# BUILD
FROM rust:1.92-bullseye AS builder
WORKDIR /usr/src/ontrack

# Copy lib
COPY Cargo.toml Cargo.lock ./
COPY src ./src 

# Copy server
RUN mkdir -p crates/server
COPY crates/server/Cargo.toml crates/server/start_logo.txt ./crates/server/
COPY crates/server/src ./crates/server/src

# Build
RUN cargo build -r -p server

# RUNTIME

FROM debian:bookworm-slim

WORKDIR /app

# Copy build output
COPY --from=builder /usr/src/ontrack/target/release/ontrack-server /usr/local/bin/

EXPOSE 3000

# ENV
ENV GTFS_DATA_PATH=/app/GTFS.zip

ENTRYPOINT ["ontrack-server"]
