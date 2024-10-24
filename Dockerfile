FROM rust:1.82-slim-bullseye as builder

WORKDIR /usr/src/app
COPY . .

RUN apt update && apt install pkg-config libssl-dev -y
RUN rustup component add rustfmt
RUN cargo build --release

RUN cp target/release/ibc-watcher /ibc-watcher

FROM rust:1.82-slim-bullseye
WORKDIR /usr/src/app
COPY --from=builder /ibc-watcher /usr/bin/ibc-watcher

ENTRYPOINT ["/usr/bin/ibc-watcher", "start"]
