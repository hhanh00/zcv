FROM ubuntu:latest AS builder

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.cargo/bin:${PATH}"

RUN apt-get update && \
    apt-get install -y curl build-essential libssl-dev pkg-config golang-go && \
    apt-get clean

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN go install github.com/cometbft/cometbft/cmd/cometbft@v0.38

WORKDIR /zcv
COPY Cargo.toml .
COPY zcvlib/ ./zcvlib/
COPY setup/ ./setup/

RUN cargo build --release

FROM ubuntu:latest
COPY --from=builder /root/go/bin/cometbft /cometbft
COPY --from=builder /zcv/target/release/vote-cometbft /vote-cometbft
