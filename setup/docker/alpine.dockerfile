FROM rust:alpine AS builder

WORKDIR /zcv
COPY . .
RUN apk add perl build-base
RUN cargo b --release
RUN go install github.com/cometbft/cometbft/cmd/cometbft@v0.38

FROM alpine
COPY --from=builder /zcv/target/release/vote-cometbft /bin/vote-cometbft
COPY --from=builder /go/bin/cometbft /bin/vote-cometbft
ENTRYPOINT [ "/bin/sh" ]
