FROM docker.io/rust:slim-bookworm as builder

RUN apt update \
    && apt install -y openssl libssl-dev ca-certificates pkg-config \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

WORKDIR /app/src

COPY ./ ./

RUN cargo test

RUN cargo build --release


FROM debian:stable-slim

WORKDIR /app

RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt upgrade -y \
    && apt autoremove -y \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

EXPOSE 80 443

COPY --from=builder /app/src/target/release/cache-registry ./

CMD ["/app/cache-registry"]
