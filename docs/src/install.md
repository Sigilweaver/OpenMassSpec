# Install

## From source (Rust)

OpenProteo targets Rust 2021, MSRV 1.75. The umbrella workspace lives
at the `OpenProteo` repository and pulls the vendor crates in via
path dependencies; for a stand-alone build, clone all five repos side
by side:

```text
.
+-- OpenProteoCore/
+-- OpenProteo/
+-- OpenTFRaw/
+-- OpenTDF/
+-- OpenWRaw/
```

Then build the CLI:

```sh
cd OpenProteo
cargo build --release -p openproteo-io-cli
./target/release/vendor2mzml --help
```

## Python (PyPI)

The Python bindings are distributed as `openproteo-io`. Wheels are
abi3-py39, so a single wheel covers Python 3.9 and newer.

```sh
pip install openproteo-io          # core
pip install 'openproteo-io[arrow]' # with pyarrow zero-copy bridge
```

## Pre-built binaries

Release builds of `vendor2mzml` are attached to each GitHub release
for:

- `linux-x86_64`
- `linux-aarch64`
- `macos-x86_64`
- `macos-aarch64`
- `windows-x86_64`

Download the archive for your platform from the [Releases](https://github.com/nathanriley/OpenProteo/releases)
page, extract, and put `vendor2mzml` on your `PATH`.
