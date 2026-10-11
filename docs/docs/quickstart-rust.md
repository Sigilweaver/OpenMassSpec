# Quickstart: Rust

Add the umbrella crate to your `Cargo.toml`:

```toml
[dependencies]
openmassspec-io   = { path = "../OpenMassSpec/crates/openmassspec-io", features = ["all"] }
openmassspec-core = { path = "../OpenMassSpecCore" }
```

The `all` feature pulls in every vendor. To trim binary size you can
opt in to one vendor at a time: `features = ["thermo"]`,
`features = ["bruker"]`, `features = ["waters"]`,
`features = ["agilent"]`, `features = ["sciex"]`,
`features = ["shimadzu"]`.

## Convert a file to mzML

```rust,no_run
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input  = Path::new("sample.raw");
    let output = Path::new("sample.mzML");

    let detected = openmassspec_io::detect_format(input)
        .ok_or("not a recognized vendor format")?;
    let opts = openmassspec_io::ConvertOptions::new().with_indexed(true);
    let summary = openmassspec_io::convert_to_mzml(&detected, output, &opts)?;
    println!("{} spectra written", summary.spectra_written);
    Ok(())
}
```

## Unreadable spectra

Every vendor source yields `Result` records: a spectrum or chromatogram
the reader cannot decode is an `Err`, never silently dropped.
`convert_to_mzml`, `collect`, and `stream` stop at the first one and
return it as `openmassspec_io::Error::Reader`. To write everything that
can be read and leave the rest out, select the skip policy and report
the count to your users:

```rust,no_run
use std::path::Path;
use openmassspec_io::core::RecordErrorPolicy;
use openmassspec_io::ConvertOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detected = openmassspec_io::detect_format(Path::new("sample.raw"))
        .ok_or("not a recognized vendor format")?;
    let opts = ConvertOptions::new().with_record_error_policy(RecordErrorPolicy::Skip);
    let summary = openmassspec_io::convert_to_mzml(&detected, Path::new("sample.mzML"), &opts)?;
    if summary.skipped() > 0 {
        eprintln!("warning: {} unreadable records skipped", summary.skipped());
    }
    Ok(())
}
```

## Reader-specific errors

Every reader failure, whether opening the input or decoding a record, is
`Error::Reader { vendor, source }`, where `source` is the reader crate's
own error boxed as `dyn std::error::Error`. The reader crates are not
re-exported, so `openmassspec-io`'s API does not change when a reader
releases a new major version. To inspect reader-specific detail, depend
on that reader crate directly and downcast `Error::reader_error()`:

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

`Error::reader_vendor()` returns the vendor name (`"thermo"`, ...) without
downcasting.

## Centroid on the way out

`ConvertOptions::with_centroid`, `collect_centroided`, and
`stream_centroided` peak-pick every profile-mode spectrum first
(spectra already tagged centroid pass through unchanged) via
`openmassspec_core::Centroided`, with an optional `min_intensity`
noise floor. Plain `collect` / `convert_to_mzml` never centroid - this
is always opt-in:

```rust,no_run
use std::path::Path;
use openmassspec_io::ConvertOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detected = openmassspec_io::detect_format(Path::new("sample.raw"))
        .ok_or("not a recognized vendor format")?;
    let opts = ConvertOptions::new()
        .with_indexed(true)
        .with_centroid(true)
        .with_centroid_min_intensity(500.0);
    openmassspec_io::convert_to_mzml(&detected, Path::new("sample.mzML"), &opts)?;
    Ok(())
}
```

## Iterate spectra without writing mzML

```rust,no_run
use openmassspec_core::SpectrumSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detected = openmassspec_io::detect_format(std::path::Path::new("sample.raw"))
        .ok_or("not a vendor format")?;
    let (records, meta) = openmassspec_io::collect(&detected)?;
    println!("{} spectra from {}", records.len(), meta.instrument.name);
    for s in records.iter().take(5) {
        println!(
            "idx={} ms={} rt={:.2}s peaks={}",
            s.index, s.ms_level, s.retention_time_sec, s.mz.len()
        );
    }
    Ok(())
}
```

For long runs you usually want the streaming variant, which visits one
spectrum at a time:

```rust,no_run
let detected = openmassspec_io::detect_format(std::path::Path::new("sample.d")).unwrap();
let meta = openmassspec_io::stream(&detected, |s| {
    // process one spectrum at a time
    Ok(())
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`with_source` hands you the vendor source as a
`&mut dyn SpectrumSource` for anything the convenience functions do
not cover; `iter_spectra` yields `Result` items:

```rust,no_run
let detected = openmassspec_io::detect_format(std::path::Path::new("sample.d")).unwrap();
openmassspec_io::with_source(&detected, |src| {
    for s in src.iter_spectra() {
        let s = s?;
        // ...
    }
    Ok(())
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Validate

```rust,no_run
use openmassspec_core::conformance::assert_iter_invariants;

let detected = openmassspec_io::detect_format(std::path::Path::new("sample.raw")).unwrap();
let (records, _) = openmassspec_io::collect(&detected)?;
let n = assert_iter_invariants(records.iter().cloned())?;
println!("conformance ok: {n} spectra");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Arrow

Enable the `arrow` feature on `openmassspec-io`, which re-exports the
`openmassspec-arrow` crate as `openmassspec_io::arrow`, and build a
record batch:

```rust,no_run
use openmassspec_io::arrow::SpectrumBatchBuilder;

let detected = openmassspec_io::detect_format(std::path::Path::new("sample.d")).unwrap();
let meta = openmassspec_io::metadata_only(&detected)?;
let mut b = SpectrumBatchBuilder::new(meta.mobility_array_kind);
openmassspec_io::stream(&detected, |s| {
    b.push(&s);
    Ok(())
})?;
let batch = b.finish()?;
println!("{} rows x {} cols", batch.num_rows(), batch.num_columns());
# Ok::<(), Box<dyn std::error::Error>>(())
```
