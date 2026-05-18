# Arrow schema

`openproteo-core`'s `arrow` feature exposes a single record-batch
schema that is identical across all vendors. One row = one spectrum;
peak arrays live in two `LargeList<Float>` columns alongside scalar
metadata columns.

## Schema (flat)

| Column                  | Arrow type                | Notes                                            |
| ----------------------- | ------------------------- | ------------------------------------------------ |
| `index`                 | `UInt64`                  | 0-based, strictly increasing.                    |
| `native_id`             | `Utf8`                    | Vendor native id (e.g. `controllerType=0 ...`).  |
| `ms_level`              | `UInt32`                  | 1, 2, ...                                        |
| `polarity`              | `UInt8`                   | 0 = positive, 1 = negative, 255 = unknown.       |
| `retention_time_sec`    | `Float64`                 | Seconds.                                         |
| `scan_window_low_mz`    | `Float64`                 | Optional, null if unknown.                       |
| `scan_window_high_mz`   | `Float64`                 |                                                   |
| `precursor_mz`          | `Float64`                 | Null for MS1.                                    |
| `precursor_charge`      | `Int32`                   | Null when not assigned.                          |
| `precursor_isolation_lo`| `Float64`                 |                                                   |
| `precursor_isolation_hi`| `Float64`                 |                                                   |
| `activation`            | `Utf8`                    | `HCD`, `CID`, `ETD`, ...                         |
| `analyzer`              | `Utf8`                    | `Orbitrap`, `TOF`, ...                           |
| `scan_mode`             | `Utf8`                    |                                                   |
| `mz`                    | `LargeList<Float64>`      | Ascending peaks.                                 |
| `intensity`             | `LargeList<Float32>`      | Same length as `mz`.                             |
| `inv_mobility_per_peak` | `LargeList<Float32>` or null | Present on Bruker TDF when mobility is enabled. |

`SpectrumBatchBuilder::new(Option<MobilityArrayKind>)` toggles the
final column. Pass `None` for instruments without ion mobility (every
column is materialized but stays null).

## Building a batch

```rust,no_run
use openproteo_core::arrow::{spectrum_record_schema, SpectrumBatchBuilder};
use openproteo_core::SpectrumSource;

let mut src = opentimstdf::mzml::TdfSource::open("sample.d")?;
let mut b = SpectrumBatchBuilder::new(Some(openproteo_core::MobilityArrayKind::InverseK0));
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
