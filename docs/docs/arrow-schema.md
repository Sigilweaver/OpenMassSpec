# Arrow schema

`openmassspec-core`'s `arrow` feature exposes a single record-batch
schema that is identical across all vendors. One row = one spectrum;
peak arrays live in `LargeList<Float>` columns alongside scalar
metadata columns. Precursor fields are inlined as nullable scalar
columns; an MS1 spectrum has all `precursor_*` columns null. The
schema is part of this crate's stable public surface: any column
addition is a minor-version bump, any removal or rename is breaking.

## Schema (flat)

| Column                       | Arrow type              | Notes                                                                 |
| ---------------------------- | ------------------------ | ---------------------------------------------------------------------- |
| `index`                       | `UInt32`                 | 0-based, strictly increasing.                                         |
| `scan_number`                 | `UInt32`                 | 1-based, source-stable.                                              |
| `native_id`                   | `Utf8`                   | Vendor native id (e.g. `controllerType=0 ...`).                      |
| `ms_level`                    | `UInt8`                  | 1, 2, ...                                                             |
| `polarity`                    | `Utf8`, nullable         | `"positive"` / `"negative"`.                                         |
| `scan_mode`                   | `Utf8`, nullable         | `"profile"` / `"centroid"`.                                          |
| `analyzer`                    | `Utf8`, nullable         | `"itms"`, `"tqms"`, `"sqms"`, `"tof"`, `"ftms"`, `"sector"`.          |
| `filter`                      | `Utf8`, nullable         | Thermo-style scan filter string, when the vendor populates one.       |
| `retention_time_sec`          | `Float64`                | Seconds.                                                              |
| `total_ion_current`           | `Float64`, nullable      | Vendor-declared TIC, when present.                                    |
| `base_peak_mz`                | `Float64`, nullable      | Vendor-declared base-peak m/z, when present.                          |
| `base_peak_intensity`         | `Float64`, nullable      | Vendor-declared base-peak intensity, when present.                    |
| `low_mz`                      | `Float64`, nullable      | Lowest observed m/z, when the vendor declares one.                    |
| `high_mz`                     | `Float64`, nullable      | Highest observed m/z, when the vendor declares one.                  |
| `ion_injection_time_ms`       | `Float64`, nullable      |                                                                        |
| `inv_mobility`                | `Float64`, nullable      | Mean inverse reduced ion mobility (1/K0) for the spectrum.            |
| `faims_cv`                    | `Float64`, nullable      | FAIMS compensation voltage in volts.                                  |
| `precursor_target_mz`         | `Float64`, nullable      | Isolation-window center m/z. Null for MS1.                            |
| `precursor_selected_mz`       | `Float64`, nullable      | Monoisotopic-resolved precursor m/z.                                  |
| `precursor_isolation_width`   | `Float64`, nullable      | Total isolation width in m/z.                                         |
| `precursor_charge`            | `Int32`, nullable        | Null when not assigned.                                              |
| `precursor_intensity`         | `Float64`, nullable      |                                                                        |
| `precursor_collision_energy`  | `Float64`, nullable      | Interpretation depends on `precursor_ce_is_nce`.                       |
| `precursor_ce_is_nce`         | `UInt8`, nullable        | `1` if `precursor_collision_energy` is normalized (NCE %), else `0`.  |
| `precursor_native_id`         | `Utf8`, nullable         | Native id of the precursor scan, when known.                          |
| `precursor_activation`        | `Utf8`, nullable         | `"cid"`, `"hcd"`, `"etd"`, `"ecd"`, `"uvpd"`, `"pqd"`, `"pd"`, `"sid"`, `"ethcd"`, `"irmpd"`, `"mpid"`. |
| `precursor_analyzer`          | `Utf8`, nullable         | Analyzer that recorded the precursor scan; same value set as `analyzer`. |
| `precursor_ccs`               | `Float64`, nullable      | Collision cross-sectional area of the selected ion, in square angstroms. |
| `mz`                          | `LargeList<Float64>`     | Ascending peaks.                                                      |
| `intensity`                   | `LargeList<Float32>`     | Same length as `mz`.                                                  |
| `inv_mobility_per_peak`       | `LargeList<Float32>`, nullable | Present when the vendor carries a per-peak mobility array (e.g. Bruker TIMS).       |
| `mobility_array_kind`         | `Utf8`, nullable         | `"inverse_reduced_k0"` / `"drift_time_ms"`; interpretation of `inv_mobility_per_peak` for every row in the batch. |

`SpectrumBatchBuilder::new(Option<MobilityArrayKind>)` sets the
`mobility_array_kind` value shared by every row in the batch. Pass
`None` for instruments without ion mobility (`inv_mobility_per_peak`
and `mobility_array_kind` stay null for every row).

## Building a batch

```rust,no_run
use openmassspec_core::arrow::{spectrum_record_schema, SpectrumBatchBuilder};
use openmassspec_core::{MobilityArrayKind, SpectrumSource};
use opentimstdf::mzml::TdfSource;

let mut src = TdfSource::open("sample.d")?;
let mut b = SpectrumBatchBuilder::new(Some(MobilityArrayKind::InverseReducedVsPerCm2));
for s in src.iter_spectra() {
    b.push(&s);
}
let batch = b.finish()?;
assert_eq!(batch.schema(), spectrum_record_schema());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Why `LargeList`?

Peak arrays for a single TDF MS1 frame routinely cross the 2^31 byte
boundary when stored back-to-back, especially in 32-bit float
intensities. `LargeList` (64-bit offsets) avoids the silent truncation
that `List` (32-bit offsets) would otherwise introduce.
