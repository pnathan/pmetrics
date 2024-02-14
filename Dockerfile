# sourced from https://www.21analytics.ch/blog/docker-from-scratch-for-rust-applications/
FROM docker.io/library/rust:1.72-alpine as builder
RUN apk add --no-cache musl-dev sqlite-static openssl-dev openssl-libs-static pkgconf git libpq-dev

# Set `SYSROOT` to a dummy path (default is /usr) because pkg-config-rs *always*
# links those located in that path dynamically but we want static linking, c.f.
# https://github.com/rust-lang/pkg-config-rs/blob/54325785816695df031cef3b26b6a9a203bbc01b/src/lib.rs#L613
ENV SYSROOT=/dummy

# The env vars tell libsqlite3-sys to statically link libsqlite3.
ENV SQLITE3_STATIC=1 SQLITE3_LIB_DIR=/usr/lib/

# The env var tells pkg-config-rs to statically link libpq.
ENV LIBPQ_STATIC=1

WORKDIR /wd
# copy . /wd/
RUN mkdir -p /wd/pmetrics

COPY src/ /wd/src
COPY Cargo.toml /wd/Cargo.toml
COPY Cargo.lock /wd/Cargo.lock
COPY .cargo /wd/.cargo
RUN find /wd/
COPY vendor/ /wd/vendor

RUN cargo build --bins --release --offline

FROM scratch
ARG version=unknown
ARG release=unreleased
EXPOSE 1337
LABEL version=${version} \
      release=${release}

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /wd/target/debug/pmetrics /opt/pmetrics
ENTRYPOINT ["/opt/pmetrics", "server", "--server-type", "http", "--port", "1337"]
