# Packaging

Distribution channels for `vendor2mzml` (crate `openmassspec-io-cli`)
beyond the GitHub release binaries and PyPI.

| Channel | Files | Status |
| --- | --- | --- |
| Docker (ghcr.io) | [`../Dockerfile`](../Dockerfile), [`../.github/workflows/docker.yml`](../.github/workflows/docker.yml) | Built and smoke-tested on PRs; pushed on `v*` tags |
| bioconda | [`bioconda/openmassspec-io/`](bioconda/openmassspec-io/) | Draft only, not submitted |

## Docker

The `Dockerfile` is a two-stage build:

1. `rust:1-slim-trixie` runs `cargo build --release -p openmassspec-io-cli`.
2. `gcr.io/distroless/cc-debian13:nonroot` holds the binary at
   `/usr/local/bin/vendor2mzml` (entrypoint) and the license at
   `/usr/share/doc/vendor2mzml/LICENSE`. The working directory is
   `/data`, and the default command is `--help`.

Both base images can be overridden with `--build-arg RUST_IMAGE=...` and
`--build-arg RUNTIME_IMAGE=...`.

The runtime image has no shell and runs as the distroless `nonroot`
user. To write into a bind-mounted host directory, pass
`--user "$(id -u):$(id -g)"`:

```sh
docker build -t vendor2mzml .
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/data" \
  vendor2mzml convert /data/sample.raw /data/sample.mzML --indexed
```

### Workflow

`.github/workflows/docker.yml` runs on pull requests to `main` and on
`v*` tag pushes:

1. Builds the image (linux/amd64) with the GitHub Actions build cache.
2. Downloads the Agilent fixture `180814-Sample19.d` (PXD030293, PRIDE,
   CC0, about 960 KB zipped), the same fixture `ci.yml` uses.
3. Smoke test: runs `--version` and `--help`, converts the fixture to
   indexed mzML, checks it with `xmllint --noout`, asserts an
   `indexedmzML` root and at least one `<spectrum>`, then runs
   `vendor2mzml validate` on the output.
4. On `v*` tags only: logs in to ghcr.io with `GITHUB_TOKEN` and pushes
   `ghcr.io/sigilweaver/vendor2mzml:<version>` (tag without the leading
   `v`) and `:latest`.

No secrets other than `GITHUB_TOKEN` are used. After the first push, set
the package visibility to public under the organization's Packages
settings if it should be pullable without authentication.

## bioconda

`bioconda/openmassspec-io/` holds a draft recipe that follows bioconda's
Rust conventions:

- `build.sh` runs `cargo-bundle-licenses` to write `THIRDPARTY.yml`, then
  `cargo install --no-track --root "$PREFIX" --path crates/openmassspec-io-cli`.
  `--locked` is not passed because the source tarball has no
  `Cargo.lock`. Add it if a lockfile is committed later.
- Build requirements are `compiler('rust')`, plus `compiler('c')` and
  `stdlib('c')`, because `zstd-sys` (through opentimstdf) compiles
  bundled C.
- `run_exports` pins the package with `pin_subpackage(name, max_pin="x")`.
- `about/license_file` lists `LICENSE` and `THIRDPARTY.yml`.
- The test runs `vendor2mzml --help` and `--version`, converts the SCIEX
  fixture `Rcor2KOESC1.wiff` + `.wiff.scan` (PXD022088, PRIDE, CC0, about
  2.5 MB, also used by `ci.yml`), fetched as extra sources into
  `test-data/`, and then runs `vendor2mzml validate` on the result.
- `extra/additional-platforms` adds `linux-aarch64` and `osx-arm64`.

The draft has placeholder (all-zero) `sha256` values, and nothing has
been submitted to bioconda or conda-forge.

### Submission steps

1. Wait until openaraw 0.3.0 is published, the workspace dependency is
   updated, and the release tag (for example `v2.0.0`) is pushed.
   Confirm that `cargo install --git https://github.com/Sigilweaver/OpenMassSpec --tag v2.0.0 openmassspec-io-cli`
   builds.
2. Set `version` in `meta.yaml` to the tag without the leading `v`.
3. Fill in the three `sha256` values:

   ```sh
   curl -sL https://github.com/Sigilweaver/OpenMassSpec/archive/refs/tags/v2.0.0.tar.gz | sha256sum
   curl -sL https://ftp.pride.ebi.ac.uk/pride/data/archive/2020/12/PXD022088/Rcor2KOESC1.wiff | sha256sum
   curl -sL https://ftp.pride.ebi.ac.uk/pride/data/archive/2020/12/PXD022088/Rcor2KOESC1.wiff.scan | sha256sum
   ```

4. Confirm the `recipe-maintainers` GitHub handle(s) in `meta.yaml`.
5. Fork [bioconda/bioconda-recipes](https://github.com/bioconda/bioconda-recipes),
   create a branch, and copy `packaging/bioconda/openmassspec-io/` to
   `recipes/openmassspec-io/`.
6. Optionally lint and build locally from the bioconda-recipes checkout
   (needs `bioconda-utils` and Docker):

   ```sh
   bioconda-utils lint recipes config.yml --packages openmassspec-io
   bioconda-utils build recipes config.yml --docker --mulled-test --packages openmassspec-io
   ```

7. Open a pull request against `bioconda/bioconda-recipes`. When its CI
   is green, comment `@BiocondaBot please add label` to request review.
8. After it merges, the bioconda autobump bot opens version-bump PRs
   for new GitHub release tags. If a bump changes the build, update this
   draft too.

Notes:

- The conda package is named `openmassspec-io` but ships only the
  `vendor2mzml` binary, not the Python module of the same name on PyPI.
- When the package is live, update the "bioconda (pending)" entry in the
  top-level README's Install section with the
  `conda install -c conda-forge -c bioconda openmassspec-io` command.
