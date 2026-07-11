# OpenMassSpec

[![CI](https://github.com/Sigilweaver/OpenMassSpec/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenMassSpec/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.20470595.svg)](https://doi.org/10.5281/zenodo.20470595)
[![crates.io](https://img.shields.io/crates/v/openmassspec-io.svg)](https://crates.io/crates/openmassspec-io)
[![PyPI](https://img.shields.io/pypi/v/openmassspec.svg)](https://pypi.org/project/openmassspec/)
[![docs.rs](https://img.shields.io/docsrs/openmassspec-io)](https://docs.rs/openmassspec-io)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust MSRV](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Docs](https://img.shields.io/badge/docs-sigilweaver.app-blue.svg)](https://sigilweaver.app/openmassspec/docs/)

> **One stack. Five vendors. Open Rust.**
>
> OpenMassSpec is the open-source Rust stack for mass spectrometry
> raw-file access. Read Thermo, Bruker, Waters, Agilent, and SCIEX
> acquisitions through a single API, convert them to PSI-MS
> [mzML 1.1.0](https://www.psidev.info/mzML) with the canonical writer,
> and stream them straight into Arrow for downstream analytics. No
> vendor SDKs, no Windows-only DLLs, no binary blobs in your release
> pipeline.

## The stack

| Layer | Crate | What it does |
| --- | --- | --- |
| Umbrella | [`openmassspec-io`](crates/openmassspec-io) | Feature-gated re-exports + `detect_format` + `convert_to_mzml` |
| CLI | [`openmassspec-io-cli`](crates/openmassspec-io-cli) | `vendor2mzml` one-shot binary |
| Python | [`openmassspec`](python) | Metapackage exposing the converter from Python |
| Shared core | [openmassspec-core](https://github.com/Sigilweaver/OpenMassSpecCore) | `SpectrumRecord`, Arrow batch, mzML writer |
| Thermo `.raw` | [opentfraw](https://github.com/Sigilweaver/OpenTFRaw) | Finnigan reader |
| Bruker `.d/` | [opentimstdf](https://github.com/Sigilweaver/OpenTimsTDF) | timsTOF TDF reader |
| Waters `.raw/` | [openwraw](https://github.com/Sigilweaver/OpenWRaw) | MassLynx bundle reader |
| Agilent `.d/` | [openaraw](https://github.com/Sigilweaver/OpenARaw) | MassHunter reader |
| SCIEX `.wiff` | [opensxraw](https://github.com/Sigilweaver/OpenSXRaw) | legacy `.wiff`/`.wiff.scan` reader |

Current pinned stack lives in [STACK.md](STACK.md).

## Install

### CLI

Pre-built `vendor2mzml` binaries land on the GitHub
[Releases](https://github.com/Sigilweaver/OpenMassSpec/releases) page.

Or build from source:

```sh
cargo install openmassspec-io-cli --features all
```

### Rust library

```toml
[dependencies]
openmassspec-io = { version = "1.0", features = ["all"] }
```

Vendor features are independent (`thermo`, `bruker`, `waters`,
`agilent`, `sciex`) so you only compile what you ship.

### Python

```sh
pip install openmassspec
```

## Use it

### Convert a file

```sh
vendor2mzml /data/sample.raw /tmp/sample.mzML --indexed
```

`vendor2mzml` sniffs the format from the path (or directory layout for
Bruker `.d/` and Waters `.raw/` bundles), routes through the matching
vendor crate, and writes indexed PSI-MS mzML 1.1.0.

### From Rust

```rust
use openmassspec_io::{detect_format, convert_to_mzml};

let fmt = detect_format("sample.raw")?;
println!("detected: {fmt:?}");
convert_to_mzml("sample.raw", "sample.mzML", /* indexed */ true)?;
```

### From Python

```python
import openmassspec

openmassspec.to_mzml("sample.raw", "sample.mzML", indexed=True)
```

## Documentation

Full reference, conversion semantics, and the per-vendor parser notes
live at [**sigilweaver.app/openmassspec/docs**](https://sigilweaver.app/openmassspec/docs/).

The source for that site is in [`docs/`](docs/) (Docusaurus). See
[docs/README.md](docs/README.md) for the build commands.

## Contributing

Bug reports and PRs are welcome on any of the five repos. See
[SECURITY.md](SECURITY.md) for the security policy.

This umbrella ships releases via [`scripts/release-stack.sh`](scripts/release-stack.sh)
which gates on the downstream [SpecLance](https://github.com/Sigilweaver/SpecLance)
truth-test before tagging.

## License

[Apache-2.0](LICENSE). Each vendor crate carries its own header and
upstream attribution; this repo only orchestrates them.
