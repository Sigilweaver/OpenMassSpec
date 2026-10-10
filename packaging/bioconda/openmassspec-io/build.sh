#!/bin/bash
set -euxo pipefail

export RUST_BACKTRACE=1

# Third-party license texts for every crate linked into the binary;
# shipped via about/license_file in meta.yaml.
cargo-bundle-licenses --format yaml --output THIRDPARTY.yml

# The source tarball carries no Cargo.lock, so --locked is not passed.
cargo install -v --locked --no-track \
    --root "${PREFIX}" \
    --path crates/openmassspec-io-cli
