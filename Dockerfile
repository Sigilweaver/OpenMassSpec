# syntax=docker/dockerfile:1
#
# Container image for vendor2mzml (crate openmassspec-io-cli).
#
# Build:   docker build -t vendor2mzml .
# Run:     docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/data" \
#            vendor2mzml convert /data/sample.raw /data/sample.mzML --indexed
#
# The build stage compiles the binary with cargo; the runtime stage is
# distroless (glibc + libgcc only, no shell) and runs as a non-root user.
# Published to ghcr.io/sigilweaver/vendor2mzml by .github/workflows/docker.yml.

ARG RUST_IMAGE=rust:1-slim-trixie
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian13:nonroot

FROM ${RUST_IMAGE} AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release -p openmassspec-io-cli \
    && install -D -m 0755 target/release/vendor2mzml /out/vendor2mzml

FROM ${RUNTIME_IMAGE}
COPY --from=build /out/vendor2mzml /usr/local/bin/vendor2mzml
COPY LICENSE /usr/share/doc/vendor2mzml/LICENSE
WORKDIR /data
ENTRYPOINT ["/usr/local/bin/vendor2mzml"]
CMD ["--help"]
