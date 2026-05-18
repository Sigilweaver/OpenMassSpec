# Conformance contract

`openproteo-core` ships a small conformance harness that codifies the
invariants every vendor source is expected to satisfy. The harness is
the same set of checks the `vendor2mzml convert --validate` flag
applies before writing mzML.

## Invariants

Given an iterator of `SpectrumRecord`, the harness verifies, for
every record:

1. **Strictly increasing `index`.** Indices start at 0 and increment
   by 1 across the run.
2. **Non-negative `retention_time_sec`** and monotonically
   non-decreasing per-control-list (per scan trace within a vendor
   that interleaves traces).
3. **Peak-array length agreement.** `mz.len() == intensity.len()`.
   For ion-mobility records, `inv_mobility_per_peak`, when present,
   has the same length.
4. **`ms_level >= 1`.** No record may declare itself as level zero.
5. **Sorted `mz` per spectrum.** The peak list is ascending in
   `mz`. Equal `mz` values are permitted (some vendors emit zero-
   intensity flanking points).
6. **No NaN.** Neither `mz` nor `intensity` contains NaN; intensity
   is non-negative.

## API

```rust,no_run
use openproteo_core::conformance::{assert_iter_invariants, assert_source_invariants};
```

- `assert_source_invariants(&mut src)` consumes the source's
  `iter_spectra()` to completion and returns `Ok(count)` or the first
  `ConformanceError` encountered.
- `assert_iter_invariants(iter)` works against any
  `IntoIterator<Item = SpectrumRecord>` - convenient when the vendor
  layer is not yet a `SpectrumSource`.

## Errors

`ConformanceError` is a small enum with one variant per failure
class. Each variant carries the `index` of the offending spectrum and
a human-readable summary; the `Display` impl is what
`vendor2mzml convert --validate` prints before exiting with status 3.

## Stability guarantee

The invariants listed here are part of the OpenProteo 0.x contract.
Adding new invariants is a semver-minor change (a previously-accepted
file may start failing); relaxing an invariant is a semver-major
change. The exact wording of `Display` is **not** part of the API.
