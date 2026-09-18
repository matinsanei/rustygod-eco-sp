# rustygod-saleor — multi-stage. Runtime is busybox:glibc (~4MB) +
# the ~20MB release binary: total well under the 100MB Phase-2 budget.
# (gcr distroless would be equivalent; unreachable from this network.)
ARG RUST_VERSION=1.89

FROM rust:${RUST_VERSION}-bookworm AS builder
WORKDIR /build
# Layer cache: deps first.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN apt-get update -qq && apt-get install -y -qq protobuf-compiler > /dev/null \
    && cargo build --release -p rustygod-server --bin rustygod-server \
    && strip target/release/rustygod-server

FROM busybox:glibc
# Rust gnu binaries need libgcc_s; busybox:glibc ships libc but not that.
COPY --from=builder /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/
COPY --from=builder /build/target/release/rustygod-server /rustygod-server
ENV RUSTYGOD_ADDR=0.0.0.0:50051 \
    RUSTYGOD_METRICS_ADDR=0.0.0.0:9000 \
    RUST_LOG=info
EXPOSE 50051 9000
ENTRYPOINT ["/rustygod-server"]
