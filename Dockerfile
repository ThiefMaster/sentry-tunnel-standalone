FROM rust:alpine AS builder

WORKDIR /build
COPY . /build
RUN cargo build --release


FROM alpine:latest

RUN set -ex && \
	addgroup -S app && \
	adduser -S -G app -h /app app

COPY --from=builder /build/target/release/sentry-tunnel-standalone /usr/local/bin/sentry-tunnel-standalone
WORKDIR /app
USER app
CMD ["sentry-tunnel-standalone"]
EXPOSE 8008
