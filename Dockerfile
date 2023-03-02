####################################################################################################
## Builder
####################################################################################################
FROM --platform=$BUILDPLATFORM rust:latest AS builder
ARG TARGETPLATFORM
ARG BUILDPLATFORM

RUN if [ "$BUILDPLATFORM" = "linux/amd64" ]; \
        then rustup target add x86_64-unknown-linux-musl; \
    elif [ "$BUILDPLATFORM" = "linux/arm64" ]; \
        then rustup target add aarch64-unknown-linux-musl; \
    else \
        exit 1; \
    fi

# Create unprivileged user
ENV USER=livefb
ENV UID=1337

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /build/livefb

COPY ./src ./src
COPY ./Cargo.toml ./
COPY ./Cargo.lock ./

RUN apt update && apt install -y musl-tools musl-dev && update-ca-certificates && \
    if [ "$BUILDPLATFORM" = "linux/amd64" ]; \
        then cargo build --target x86_64-unknown-linux-musl --release; \
    elif [ "$BUILDPLATFORM" = "linux/arm64" ]; \
        then ln -s /usr/bin/musl-gcc /usr/bin/aarch64-linux-musl-gcc; \
        cargo build --target aarch64-unknown-linux-musl --release; \
    else \
        exit 1; \
    fi

####################################################################################################
## Final image
####################################################################################################
FROM alpine:latest
ARG TARGETPLATFORM
ARG BUILDPLATFORM
ENV LIVEFEEDBACK_DOCKER true

# Import from builder
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /srv/app

# Copy from build (workaround for multiple arch copy support)
COPY --from=builder /build/livefb/target ./target
RUN if [ $BUILDPLATFORM = linux/amd64 ]; \
        then cp ./target/x86_64-unknown-linux-musl/release/livefeedback ./livefeedback; \
    elif [ $BUILDPLATFORM = linux/arm64 ]; \
        then cp ./target/aarch64-unknown-linux-musl/release/livefeedback ./livefeedback; \
    else \
        exit 1; \
    fi && \
    rm -rf ./x86_64-unknown-linux-musl ./aarch64-unknown-linux-musl

# Use an unprivileged user
USER livefb:livefb

CMD ["/srv/app/livefeedback"]
