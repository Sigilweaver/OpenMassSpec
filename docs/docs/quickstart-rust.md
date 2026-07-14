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
`features = ["agilent"]`, `features = ["sciex"]`.

## Convert a file to mzML

```rust,no_run
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input  = Path::new("sample.raw");
    let output = Path::new("sample.mzML");

    let detected = openmassspec_io::detect_format(input)
        .ok_or("not a recognized vendor format")?;
    openmassspec_io::convert_to_mzml(detected, output, /* indexed = */ true)?;
    Ok(())
}
```

## Centroid on the way out

`collect`, `convert_to_mzml`, and `convert_to_mzml_writer` each have a
`_centroided` sibling that peak-picks every profile-mode spectrum first
(spectra already tagged centroid pass through unchanged) via
`openmassspec_core::Centroided`. They take an extra
`min_intensity: Option<f32>` noise floor. Plain `collect` /
`convert_to_mzml` never centroid - this is always opt-in:

```rust,no_run
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detected = openmassspec_io::detect_format(Path::new("sample.raw"))
        .ok_or("not a recognized vendor format")?;
    openmassspec_io::convert_to_mzml_centroided(
        detected,
        Path::new("sample.mzML"),
        /* indexed = */ true,
        /* min_intensity = */ Some(500.0),
    )?;
    Ok(())
}
```

## Iterate spectra without writing mzML

```rust,no_run
use openmassspec_core::SpectrumSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detected = openmassspec_io::detect_format(std::path::Path::new("sample.raw"))
        .ok_or("not a vendor format")?;
    let (records, meta) = openmassspec_io::collect(detected)?;
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

For long runs you usually want the streaming variant. Open the vendor
source directly and drive `iter_spectra` yourself - this is what
`convert_to_mzml` does internally:

```rust,no_run
use openmassspec_core::SpectrumSource;

let mut src = opentimstdf::mzml::TdfSource::open("sample.d")?;
for s in src.iter_spectra() {
    // process one spectrum at a time
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Validate

```rust,no_run
use openmassspec_core::conformance::assert_iter_invariants;

let detected = openmassspec_io::detect_format(std::path::Path::new("sample.raw")).unwrap();
let (records, _) = openmassspec_io::collect(detected)?;
let n = assert_iter_invariants(records.iter().cloned())?;
println!("conformance ok: {n} spectra");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Arrow

Enable the `arrow` feature on `openmassspec-core` and build a record
batch directly:

```rust,no_run
use openmassspec_core::arrow::SpectrumBatchBuilder;
use openmassspec_core::SpectrumSource;

let mut src = opentimstdf::mzml::TdfSource::open("sample.d")?;
let mut b = SpectrumBatchBuilder::new(None);
for s in src.iter_spectra() {
    b.push(&s);
}
let batch = b.finish()?;
println!("{} rows x {} cols", batch.num_rows(), batch.num_columns());
# Ok::<(), Box<dyn std::error::Error>>(())
```
