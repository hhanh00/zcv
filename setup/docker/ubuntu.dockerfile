FROM rust AS builder

WORKDIR /zcv
COPY . .
RUN apt-get update
RUN apt-get install -y \
  golang-go \
  perl \
  libssl-dev \
  pkg-config \
  build-essential
RUN cargo b --release
RUN go install github.com/cometbft/cometbft/cmd/cometbft@v0.38

FROM ubuntu
COPY --from=builder /zcv/target/release/vote-cometbft /bin/vote-cometbft
COPY --from=builder /root/go/bin/cometbft /bin/vote-cometbft
ENTRYPOINT [ "/bin/sh" ]
