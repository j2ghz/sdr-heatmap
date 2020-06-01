# ------------------------------------------------------------------------------
# Cargo Build Stage
# ------------------------------------------------------------------------------

FROM rust:latest as cargo-build

RUN apt-get update

RUN apt-get install musl-tools -y

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/sdr-heatmap

# COPY Cargo.toml Cargo.toml

# RUN mkdir src/ src/benches/

# RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs

# RUN touch src/benches/bench.rs

# RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

# RUN rm -f target/x86_64-unknown-linux-musl/release/deps/sdr-heatmap*

COPY . .

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------

FROM alpine:latest

COPY --from=cargo-build /usr/src/sdr-heatmap/target/x86_64-unknown-linux-musl/release/sdr-heatmap /usr/local/bin/sdr-heatmap

ENTRYPOINT ["sdr-heatmap"]
CMD ["-h"]
