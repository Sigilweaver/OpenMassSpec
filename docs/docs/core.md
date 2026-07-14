# `openmassspec-core`

`openmassspec-core` is the shared Rust foundation every vendor parser in
the stack builds on. It defines the vendor-neutral records, the
`SpectrumSource` trait, the canonical mzML 1.1.0 writer, an optional
Apache Arrow bridge, and the cross-vendor conformance harness.

- Crate: [Sigilweaver/OpenMassSpecCore](https://github.com/Sigilweaver/OpenMassSpecCore)
- License: Apache-2.0
- MSRV: 1.75
- `#![forbid(unsafe_code)]`

## Install

```toml
[dependencies]
openmassspec-core = "0.1"

# Optional: zero-copy Arrow RecordBatch builder.
openmassspec-core = { version = "0.1", features = ["arrow"] }
```

## The `SpectrumSource` trait

Every vendor parser (`opentfraw`, `opentimstdf`, `openwraw`, `openaraw`,
`opensxraw`) implements this trait. Anything downstream of a parser -
the canonical mzML writer, the Arrow batch builder, the conformance
harness, the `openmassspec-io` umbrella, the `vendor2mzml` CLI -
operates against `&mut dyn SpectrumSource`.

```rust
use openmassspec_core::{RunMetadata, SpectrumRecord, ChromatogramRecord};

pub trait SpectrumSource {
    fn run_metadata(&self) -> RunMetadata;
    fn iter_spectra<'a>(&'a mut self)
        -> Box<dyn Iterator<Item = SpectrumRecord> + 'a>;
    fn iter_chromatograms<'a>(&'a mut self)
        -> Box<dyn Iterator<Item = ChromatogramRecord> + 'a> {
        Box::new(std::iter::empty())
    }
    fn spectrum_count(&self) -> Option<usize> { None }
}
```

Boxed iterators (rather than RPITIT) keep the trait dyn-compatible so
the rest of the stack can hold `&mut dyn SpectrumSource`.

## Public API

| Symbol                                                      | Module          | Purpose                                                                  |
| ----------------------------------------------------------- | --------------- | ------------------------------------------------------------------------ |
| `SpectrumRecord`                                            | `types`         | Decoded spectrum: id, ms level, polarity, rt, peaks, precursor.          |
| `PrecursorInfo`                                             | `types`         | Selected / isolated precursor, charge, activation, scan window.          |
| `ChromatogramRecord`                                        | `types`         | TIC / BPC / SRM trace.                                                   |
| `RunMetadata`                                               | `types`         | Run-level CV terms: instrument, source format, native id format.         |
| `CvTerm`                                                    | `types`         | A PSI-MS controlled-vocabulary term.                                     |
| `Polarity`, `Analyzer`, `ScanMode`, `MsPower`, `Activation` | `enums`         | Standard enumerations.                                                   |
| `MobilityArrayKind`                                         | `enums`         | Per-peak inverse-mobility / drift-time array kind.                       |
| `SpectrumSource`                                            | `source`        | Trait every parser implements.                                           |
| `write_mzml`                                                | `mzml`          | Stream a `SpectrumSource` to a plain mzML 1.1.0 document.                |
| `write_indexed_mzml`                                        | `mzml`          | Same, with `<indexList>` + SHA-1 footer for byte-offset indexing.        |
| `conformance::assert_source_invariants`                     | `conformance`   | Check a live `SpectrumSource` for cross-vendor invariants.               |
| `conformance::assert_iter_invariants`                       | `conformance`   | Same, but from any `IntoIterator<Item = SpectrumRecord>`.                |
| `arrow::SpectrumBatchBuilder`                               | `arrow` (feat)  | Zero-copy builder for `arrow_array::RecordBatch` from a spectrum stream. |
| `arrow::spectrum_record_schema`                             | `arrow` (feat)  | The canonical Arrow schema.                                              |
| `Error`                                                     | `error`         | Aggregate `thiserror`-based error type.                                  |

## mzML writer

```rust
use openmassspec_core::{write_indexed_mzml, SpectrumSource};

fn export<S: SpectrumSource>(mut src: S, path: &std::path::Path)
    -> std::io::Result<()>
{
    let mut out = std::fs::File::create(path)?;
    write_indexed_mzml(&mut src, &mut out).map_err(std::io::Error::other)?;
    Ok(())
}
```

`write_indexed_mzml` emits a standards-compliant `<indexList>` plus a
SHA-1 footer so downstream tools (`mzML2HDF`, ProteoWizard `msconvert`,
`MzIdentML` builders, ...) can index-jump into the file.

## Conformance harness

The harness enforces the cross-vendor invariants every parser must
satisfy, surfaced as structured `ConformanceError` variants
(`PeakArrayLengthMismatch`, `MobilityArrayLengthMismatch`,
`RetentionTimeNonMonotonic`, `MissingPrecursor`, `IndexSequence`,
`EmptySpectrum`, ...).

```rust
use openmassspec_core::conformance::assert_iter_invariants;

let count = assert_iter_invariants(records)?;
println!("validated {count} spectra");
# Ok::<(), openmassspec_core::conformance::ConformanceError>(())
```

The [`vendor2mzml validate`](./quickstart-cli.md#validate) subcommand
runs this harness on any vendor input or pre-existing mzML.

## Arrow bridge (feature: `arrow`)

```rust
# #[cfg(feature = "arrow")]
# fn _doc() -> arrow_array::RecordBatch {
use openmassspec_core::arrow::SpectrumBatchBuilder;

let mut b = SpectrumBatchBuilder::new();
for s in /* SpectrumSource iter_spectra */ std::iter::empty() {
    b.push(&s);
}
let batch = b.finish();
# batch
# }
```

The canonical schema is documented in
[Arrow schema](./arrow-schema.md).

## Feature flags

| Flag    | Default | Effect                                                  |
| ------- | :-----: | ------------------------------------------------------- |
| `arrow` |    no   | Enables `arrow_array::RecordBatch` building from spectra. |

## Where it sits in the stack

```mermaid
graph TD
    core["openmassspec-core (this crate)"]

    tfraw[opentfraw] --> core
    tdf[opentimstdf] --> core
    wraw[openwraw] --> core
    araw[openaraw] --> core
    sxraw[opensxraw] --> core

    io["openmassspec-io<br/>(umbrella: detect_format, collect, to_mzml)"]
    io --> tfraw
    io --> tdf
    io --> wraw
    io --> araw
    io --> sxraw

    cli[vendor2mzml CLI] --> io
    py["openmassspec (Python metapackage)"] --> io
```
