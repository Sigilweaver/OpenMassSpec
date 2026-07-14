# Install

## From source (Rust)

OpenMassSpec targets Rust 2021, MSRV 1.75. The umbrella workspace lives
at the `OpenMassSpec` repository and pulls the vendor crates in via
path dependencies; for a stand-alone build, clone all seven repos side
by side:

```text
.
+-- OpenMassSpecCore/
+-- OpenMassSpec/
+-- OpenTFRaw/
+-- OpenTimsTDF/
+-- OpenWRaw/
+-- OpenARaw/
+-- OpenSXRaw/
```

Then build the CLI:

```sh
cd OpenMassSpec
cargo build --release -p openmassspec-io-cli
./target/release/vendor2mzml --help
```

## Python (PyPI)

The Python bindings are distributed as `openmassspec-io`. Wheels are
abi3-py39, so a single wheel covers Python 3.9 and newer.

```sh
pip install openmassspec-io          # core
pip install 'openmassspec-io[arrow]' # with pyarrow zero-copy bridge
```

## Pre-built binaries

Release builds of `vendor2mzml` are attached to each GitHub release
for:

- `linux-x86_64`
- `linux-aarch64`
- `macos-x86_64`
- `macos-aarch64`
- `windows-x86_64`

Download the archive for your platform from the [Releases](https://github.com/Sigilweaver/OpenMassSpec/releases)
page, extract, and put `vendor2mzml` on your `PATH`.
