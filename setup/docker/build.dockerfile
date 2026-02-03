FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.cargo/bin:${PATH}"

RUN apt-get update && \
    apt-get install -y curl build-essential libssl-dev pkg-config golang-go && \
    apt-get clean

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

WORKDIR /zcv
COPY Cargo.toml .
COPY zcv.toml .
COPY zcvlib/ ./zcvlib/
COPY setup/ ./setup/

RUN go install github.com/cometbft/cometbft/cmd/cometbft@v0.38
RUN cargo build --release
