FROM tailscale/tailscale:latest

RUN apk add --no-cache bash curl tar python3 py3-pip supervisor
RUN mkdir -p /var/lib/tailscale /var/log/supervisor

ARG TARGETPLATFORM

RUN echo "Building for $TARGETPLATFORM" \
    && case "$TARGETPLATFORM" in \
        "linux/amd64") \
            curl -fsSL https://github.com/cometbft/cometbft/releases/download/v1.0.1/cometbft_1.0.1_linux_amd64.tar.gz \
                | tar -xz -C /usr/local/bin cometbft \
            && curl -fsSL https://github.com/hhanh00/zcash-vote-server/releases/download/v1.2.1/zcash-vote-server-x86_64.tgz \
                | tar -xz -C / ;; \
        "linux/arm64") \
            curl -fsSL https://github.com/cometbft/cometbft/releases/download/v1.0.1/cometbft_1.0.1_linux_arm64.tar.gz \
                | tar -xz -C /usr/local/bin cometbft \
            && curl -fsSL https://github.com/hhanh00/zcash-vote-server/releases/download/v1.2.1/zcash-vote-server-aarch64.tgz \
                | tar -xz -C / ;; \
        *) echo "Unsupported platform: $TARGETPLATFORM"; exit 1 ;; \
    esac
