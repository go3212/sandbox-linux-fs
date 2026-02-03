# Build stage
FROM rust:1.84-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ripgrep \
    findutils \
    coreutils \
    grep \
    sed \
    gawk \
    diffutils \
    tree \
    file \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r linuxfs && useradd -r -g linuxfs -m linuxfs

# Create data directory
RUN mkdir -p /data && chown linuxfs:linuxfs /data

COPY --from=builder /app/target/release/linux-fs /usr/local/bin/linux-fs

USER linuxfs
EXPOSE 8080
VOLUME ["/data"]

ENV DATA_DIR=/data
ENV HOST=0.0.0.0
ENV PORT=8080

CMD ["linux-fs"]
