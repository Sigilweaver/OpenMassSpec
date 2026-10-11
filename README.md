# OpenMassSpec

[![CI](https://github.com/Sigilweaver/OpenMassSpec/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenMassSpec/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.20470595.svg)](https://doi.org/10.5281/zenodo.20470595)
[![crates.io](https://img.shields.io/crates/v/openmassspec-io.svg)](https://crates.io/crates/openmassspec-io)
[![PyPI](https://img.shields.io/pypi/v/openmassspec.svg)](https://pypi.org/project/openmassspec/)
[![docs.rs](https://img.shields.io/docsrs/openmassspec-io)](https://docs.rs/openmassspec-io)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/Sigilweaver/OpenMassSpec/blob/main/LICENSE)
[![Rust MSRV](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Docs](https://img.shields.io/badge/docs-sigilweaver.app-blue.svg)](https://sigilweaver.app/openmassspec/docs/)

> **One stack. Six vendors. Open Rust.**
>
> OpenMassSpec is the open-source Rust stack for mass spectrometry
> raw-file access. Read Thermo, Bruker, Waters, Agilent, SCIEX, and
> Shimadzu acquisitions through a single API, convert them to PSI-MS
> [mzML 1.1.0](https://www.psidev.info/mzML) with the canonical writer,
> and stream them straight into Arrow for downstream analytics. No
> vendor SDKs, no Windows-only DLLs, no binary blobs in your release
> pipeline.

## The stack

| Layer | Crate | What it does |
| --- | --- | --- |
| Umbrella | [`openmassspec-io`](https://github.com/Sigilweaver/OpenMassSpec/tree/main/crates/openmassspec-io) | Feature-gated vendor readers behind `detect_format` + `convert_to_mzml` |
| CLI | [`openmassspec-io-cli`](https://github.com/Sigilweaver/OpenMassSpec/tree/main/crates/openmassspec-io-cli) | `vendor2mzml` one-shot binary |
| Python | [`openmassspec`](https://github.com/Sigilweaver/OpenMassSpec/tree/main/python) | Metapackage exposing the converter from Python |
| Shared core | [openmassspec-core](https://github.com/Sigilweaver/OpenMassSpecCore) | `SpectrumRecord`, Arrow batch, mzML writer |
| Thermo `.raw` | [opentfraw](https://github.com/Sigilweaver/OpenTFRaw) | Finnigan reader |
| Bruker `.d/` | [opentimstdf](https://github.com/Sigilweaver/OpenTimsTDF) | timsTOF TDF reader |
| Waters `.raw/` | [openwraw](https://github.com/Sigilweaver/OpenWRaw) | MassLynx bundle reader |
| Agilent `.d/` | [openaraw](https://github.com/Sigilweaver/OpenARaw) | MassHunter reader |
| SCIEX `.wiff` | [opensxraw](https://github.com/Sigilweaver/OpenSXRaw) | legacy `.wiff`/`.wiff.scan` reader |
| Shimadzu `.qgd`/`.lcd` | [openszraw](https://github.com/Sigilweaver/OpenSZRaw) | LabSolutions GC-MS/LC-MS reader |

Current pinned stack lives in [STACK.md](https://github.com/Sigilweaver/OpenMassSpec/blob/main/STACK.md).

## Install

### pip

```sh
pip install openmassspec
```

Installs the Python API (`openmassspec.to_mzml`). The `vendor2mzml`
command-line tool is not part of the Python package.

### cargo

The `vendor2mzml` CLI (crate `openmassspec-io-cli`) is not published to
crates.io; install it from a release tag of this repository:

```sh
cargo install --git https://github.com/Sigilweaver/OpenMassSpec --tag v2.0.0 openmassspec-io-cli
```

The Rust library is on crates.io:

```toml
[dependencies]
openmassspec-io = { version = "2.0", features = ["all"] }
```

Vendor features are independent (`thermo`, `bruker`, `waters`,
`agilent`, `sciex`, `shimadzu`) so you only compile what you ship.

Pre-built `vendor2mzml` binaries for Linux, macOS, and Windows are
attached to each GitHub
[release](https://github.com/Sigilweaver/OpenMassSpec/releases).

### Docker

The [`Dockerfile`](https://github.com/Sigilweaver/OpenMassSpec/blob/main/Dockerfile) builds an image whose entrypoint is
`vendor2mzml`:

```sh
docker build -t vendor2mzml .
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/data" \
  vendor2mzml convert /data/sample.raw /data/sample.mzML --indexed
```

The image runs as a non-root user; `--user` makes the output file owned
by you. Release tags (`v*`) publish the image to
`ghcr.io/sigilweaver/vendor2mzml` as `<version>` and `latest` (see
[`.github/workflows/docker.yml`](https://github.com/Sigilweaver/OpenMassSpec/blob/main/.github/workflows/docker.yml)).

### bioconda (pending)

Not yet available. A recipe draft lives in
[`packaging/bioconda/`](https://github.com/Sigilweaver/OpenMassSpec/tree/main/packaging/bioconda); see
[`packaging/README.md`](https://github.com/Sigilweaver/OpenMassSpec/blob/main/packaging/README.md) for its status.

## Use it

### Convert a file

```sh
vendor2mzml convert /data/sample.raw /tmp/sample.mzML --indexed
```

`vendor2mzml` sniffs the format from the path (or directory layout for
Bruker `.d/` and Waters `.raw/` bundles), routes through the matching
vendor crate, and writes indexed PSI-MS mzML 1.1.0.

A spectrum or chromatogram the reader cannot decode is left out of the
output, and `convert` prints how many were skipped as a warning on
stderr; it still exits 0 when the file was written. Pass `--strict` to
stop at the first unreadable record instead: it exits 1, keeps the
partial output file, and says on stderr that the file is incomplete.

### From Rust

```rust
use std::path::Path;
use openmassspec_io::{convert_to_mzml, detect_format, ConvertOptions};

let detected = detect_format(Path::new("sample.raw")).expect("supported format");
println!("detected: {}", detected.format.name());
let summary = convert_to_mzml(
    &detected,
    Path::new("sample.mzML"),
    &ConvertOptions::new().with_indexed(true),
)?;
println!("{} spectra", summary.spectra_written);
```

The library stops at the first unreadable record by default; see
`ConvertOptions::with_record_error_policy` to skip instead.

The vendor reader crates are private dependencies of `openmassspec-io`:
no reader type is part of its API, so a reader's major release does not
force a new `openmassspec-io` major. A reader failure is
`Error::Reader { vendor, source }`. To inspect the reader's own error,
add that reader crate as a dependency and downcast:

```rust,ignore
// Cargo.toml: opentfraw = "3" alongside openmassspec-io.
match openmassspec_io::collect(&detected) {
    Ok((records, _meta)) => println!("{} spectra", records.len()),
    Err(err) => {
        match err.reader_error().and_then(|e| e.downcast_ref::<opentfraw::Error>()) {
            Some(thermo_err) => eprintln!("opentfraw: {thermo_err:?}"),
            None => eprintln!("error: {err}"),
        }
    }
}
```

### From Python

```python
import openmassspec

summary = openmassspec.to_mzml("sample.raw", "sample.mzML", indexed=True)
```

`to_mzml` skips unreadable records by default, issues a `RuntimeWarning`
with the count, and returns the written and skipped counts; pass
`skip_unreadable=False` to raise instead. `iter_spectra`, `read_arrow`,
and `read_chromatograms` always raise on an unreadable record.

## Documentation

Full reference, conversion semantics, and the per-vendor parser notes
live at [**sigilweaver.app/openmassspec/docs**](https://sigilweaver.app/openmassspec/docs/).

The source for that site is in [`docs/`](https://github.com/Sigilweaver/OpenMassSpec/tree/main/docs) (Docusaurus). See
[docs/README.md](https://github.com/Sigilweaver/OpenMassSpec/blob/main/docs/README.md) for the build commands.

## Contributing

Bug reports and PRs are welcome on any of the five repos. See
[SECURITY.md](https://github.com/Sigilweaver/OpenMassSpec/blob/main/SECURITY.md) for the security policy.

This umbrella ships releases via [`scripts/release-stack.sh`](https://github.com/Sigilweaver/OpenMassSpec/blob/main/scripts/release-stack.sh)
which gates on the downstream [SpecLance](https://github.com/Sigilweaver/SpecLance)
truth-test before tagging.

## License

[Apache-2.0](https://github.com/Sigilweaver/OpenMassSpec/blob/main/LICENSE). Each vendor crate carries its own header and
upstream attribution; this repo only orchestrates them.
