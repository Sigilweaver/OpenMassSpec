# Quickstart: Rust

Add the umbrella crate to your `Cargo.toml`:

```toml
[dependencies]
openproteo-io   = { path = "../OpenProteo/crates/openproteo-io", features = ["all"] }
openproteo-core = { path = "../OpenProteoCore" }
```

The `all` feature pulls in every vendor. To trim binary size you can
opt in to one vendor at a time: `features = ["thermo"]`,
`features = ["bruker"]`, `features = ["waters"]`.

## Convert a file to mzML

```rust,no_run
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input  = Path::new("sample.raw");
    let output = Path::new("sample.mzML");

    let detected = openproteo_io::detect_format(input)
        .ok_or("not a recognized vendor format")?;
    openproteo_io::convert_to_mzml(detected, output, /* indexed = */ true)?;
    Ok(())
}
```

## Iterate spectra without writing mzML

```rust,no_run
use openproteo_core::SpectrumSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detected = openproteo_io::detect_format(std::path::Path::new("sample.raw"))
        .ok_or("not a vendor format")?;
    let (records, meta) = openproteo_io::collect(detected)?;
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
use openproteo_core::SpectrumSource;

let mut src = opentimstdf::mzml::TdfSource::open("sample.d")?;
for s in src.iter_spectra() {
    // process one spectrum at a time
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Validate

```rust,no_run
use openproteo_core::conformance::assert_iter_invariants;

let detected = openproteo_io::detect_format(std::path::Path::new("sample.raw")).unwrap();
let (records, _) = openproteo_io::collect(detected)?;
let n = assert_iter_invariants(records.iter().cloned())?;
println!("conformance ok: {n} spectra");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Arrow

Enable the `arrow` feature on `openproteo-core` and build a record
batch directly:

```rust,no_run
use openproteo_core::arrow::SpectrumBatchBuilder;
use openproteo_core::SpectrumSource;

let mut src = opentimstdf::mzml::TdfSource::open("sample.d")?;
let mut b = SpectrumBatchBuilder::new(None);
for s in src.iter_spectra() {
    b.push(&s);
}
let batch = b.finish()?;
println!("{} rows x {} cols", batch.num_rows(), batch.num_columns());
# Ok::<(), Box<dyn std::error::Error>>(())
```
