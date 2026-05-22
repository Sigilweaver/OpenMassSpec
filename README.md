# OpenProteo

[![CI](https://github.com/Sigilweaver/OpenProteo/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenProteo/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/openproteo-io.svg)](https://crates.io/crates/openproteo-io)
[![PyPI](https://img.shields.io/pypi/v/openproteo.svg)](https://pypi.org/project/openproteo/)
[![docs](https://img.shields.io/badge/docs-sigilweaver.app-blue)](https://sigilweaver.app/openproteo/docs/)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)

> **One stack. Three vendors. Open Rust.**
>
> OpenProteo is the open-source Rust stack for proteomics raw-file
> access. Read Thermo, Bruker, and Waters acquisitions through a single
> API, convert them to PSI-MS [mzML 1.1.0](https://www.psidev.info/mzML)
> with the canonical writer, and stream them straight into Arrow for
> downstream analytics. No vendor SDKs, no Windows-only DLLs, no
> binary blobs in your release pipeline.

## The stack

| Layer | Crate | What it does |
| --- | --- | --- |
| Umbrella | [`openproteo-io`](crates/openproteo-io) | Feature-gated re-exports + `detect_format` + `convert_to_mzml` |
| CLI | [`openproteo-io-cli`](crates/openproteo-io-cli) | `vendor2mzml` one-shot binary |
| Python | [`openproteo`](python) | Metapackage exposing the converter from Python |
| Shared core | [openproteo-core](https://github.com/Sigilweaver/OpenProteoCore) | `SpectrumRecord`, Arrow batch, mzML writer |
| Thermo `.raw` | [opentfraw](https://github.com/Sigilweaver/OpenTFRaw) | Finnigan reader |
| Bruker `.d/` | [opentimstdf](https://github.com/Sigilweaver/OpenTimsTDF) | timsTOF TDF reader |
| Waters `.raw/` | [openwraw](https://github.com/Sigilweaver/OpenWRaw) | MassLynx bundle reader |

Current pinned stack lives in [STACK.md](STACK.md).

## Install

### CLI

Pre-built `vendor2mzml` binaries land on the GitHub
[Releases](https://github.com/Sigilweaver/OpenProteo/releases) page.

Or build from source:

```sh
cargo install openproteo-io-cli --features all
```

### Rust library

```toml
[dependencies]
openproteo-io = { version = "1.0", features = ["all"] }
```

Vendor features are independent (`thermo`, `bruker`, `waters`) so you
only compile what you ship.

### Python

```sh
pip install openproteo
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
use openproteo_io::{detect_format, convert_to_mzml};

let fmt = detect_format("sample.raw")?;
println!("detected: {fmt:?}");
convert_to_mzml("sample.raw", "sample.mzML", /* indexed */ true)?;
```

### From Python

```python
import openproteo

openproteo.to_mzml("sample.raw", "sample.mzML", indexed=True)
```

## Documentation

Full reference, conversion semantics, and the per-vendor parser notes
live at [**sigilweaver.app/openproteo/docs**](https://sigilweaver.app/openproteo/docs/).

The source for that site is in [`docs/`](docs/) (Docusaurus). See
[docs/README.md](docs/README.md) for the build commands.

## Contributing

Bug reports and PRs are welcome on any of the five repos. See
[SECURITY.md](SECURITY.md) for the security policy.

This umbrella ships releases via [`scripts/release-stack.sh`](scripts/release-stack.sh)
which gates on the downstream [ProLance](https://github.com/Sigilweaver/ProLance)
truth-test before tagging.

## License

[Apache-2.0](LICENSE). Each vendor crate carries its own header and
upstream attribution; this repo only orchestrates them.
