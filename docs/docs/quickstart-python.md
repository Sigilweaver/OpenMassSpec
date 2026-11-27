# Quickstart: Python

## Install

The `openmassspec` metapackage is the single pip install surface for the
stack. The base install brings the vendor-agnostic reader
(`openmassspec-io`); each per-vendor extra layers on a native binding for
direct vendor access.

```sh
pip install openmassspec            # openmassspec_io reader
pip install openmassspec[thermo]    # + opentfraw
pip install openmassspec[bruker]    # + opentimstdf
pip install openmassspec[waters]    # + openwraw
pip install openmassspec[agilent]   # + openaraw
pip install openmassspec[sciex]     # + opensxraw
pip install openmassspec[shimadzu]  # + openszraw
pip install openmassspec[all]       # all standalone vendor packages
```

`openmassspec-io` (pulled in by the base install) has every vendor -
including SCIEX and Shimadzu - compiled in, so reading and mzML
conversion work for every supported format out of the box. Each extra
above additionally installs the standalone per-vendor package for
direct, vendor-specific access.

You can also install `openmassspec-io` directly if you only want the
unified reader without the metapackage shim:

```sh
pip install 'openmassspec-io[arrow]'
```

## Detect and convert

```python
import openmassspec as op

det = op.detect_format("sample.raw")
print(det.vendor, det.path)        # 'thermo' /path/to/sample.raw

summary = op.to_mzml("sample.raw", "sample.mzML", indexed=True)
print(summary["spectra_written"], summary["spectra_skipped"])
```

`to_mzml` leaves out spectra and chromatograms the reader cannot decode
or the writer's checks refuse (`skip_unreadable=True`, the default), and
issues a `RuntimeWarning` with the count. It returns a dict with
`spectra_written`, `spectra_skipped`, `spectra_invalid` (refused by the
writer's checks), `chromatograms_written`, `chromatograms_skipped`,
`chromatograms_invalid`, `warning_counts` (records written per warning
code), `issues` (the first causes, as `{kind, position, record_id, cause}`
dicts), `issues_truncated`, and `spectrum_count_mismatch` (`None`, or
`(announced, accounted_for)`). Pass `skip_unreadable=False` to raise
`RuntimeError` at the first unreadable record. `iter_spectra`,
`read_arrow`, and `read_chromatograms` always raise on an unreadable
record.

A record written with a warning (an empty spectrum, a scan mode or
collision energy unit the format never states) also raises a
`RuntimeWarning` naming the code and the count. `Spectrum.warnings` lists
the warnings the reader set (for example `empty_spectrum`); the ones the
writer adds are in the `warning_counts` of the `to_mzml` result. `to_mzml(..., spool_dir=...)` sets where the writer
keeps encoded spectra once they pass 64 MiB; the default is the output's
directory.

## Allowing a check

`to_mzml`, `iter_spectra` and `read_arrow` take `allow=[...]`, a list of
check names that may continue with a warning instead of failing:
`"unknown_scan_mode"`, `"unknown_collision_energy_unit"` and
`"dangling_precursor_ref"` (an unknown name raises `ValueError`). Formats
that never state the scan mode (SCIEX) or the collision energy unit
(Agilent Q-TOF) need none; `RunInfo.unstated` lists what a format declares.

The functions `detect_format`, `to_mzml`, `iter_spectra`, and the
`Spectrum` class are re-exported from `openmassspec_io`. You can also
import them from `openmassspec_io` directly; the two paths refer to the
same objects.

## Vendor dispatch

For code paths that prefer the native vendor bindings (because they
expose vendor-specific surfaces beyond mzML / Arrow), the metapackage
ships a structural format detector and a dispatcher:

```python
import openmassspec as op

kind = op.detect("sample.raw")     # 'thermo' / 'bruker' / 'waters' / 'agilent' / 'sciex' / 'shimadzu' / None
reader = op.open_run("sample.raw") # opentfraw.RawFile / opentimstdf.Reader / ...
```

`open_run` requires the matching extra to be installed; otherwise it
raises `ImportError`.

## Centroid on the way out

`to_mzml`, `run_info`, `iter_spectra`, and `read_arrow` all take a
`centroid` keyword (default `False`) and a `centroid_min_intensity`
noise floor (default `None`, ignored unless `centroid=True`). Centroiding
is local-maxima peak picking over profile-mode spectra; spectra already
tagged centroid pass through unchanged. It is opt-in everywhere - a
plain call never discards profile data.

```python
import openmassspec_io as op

op.to_mzml("sample.raw", "sample.mzML", centroid=True)
op.to_mzml("sample.raw", "sample.mzML", centroid=True, centroid_min_intensity=500.0)

for s in op.iter_spectra("sample.raw", centroid=True):
    assert s.scan_mode == "centroid"
```

## Iterate spectra (numpy)

`iter_spectra` yields `Spectrum` objects whose peak arrays are
numpy arrays over Rust-owned buffers (the buffer moves into numpy on
first access, and later accesses return the same array).

```python
import openmassspec_io as op

for s in op.iter_spectra("sample.raw"):
    if s.ms_level == 1:
        mz   = s.mz             # numpy.ndarray, dtype=float64
        intensity = s.intensity # numpy.ndarray, dtype=float32
        print(s.index, s.retention_time_sec, mz.size)  # rt is None if unknown
```

For Bruker TDF, ion-mobility scans expose a per-peak inverse-mobility
array via `s.inv_mobility_per_peak`.

## Arrow record batches

```python
import openmassspec_io as op
import pyarrow as pa

reader = op.read_arrow("sample.d")        # pa.RecordBatchReader
schema = reader.schema
table  = reader.read_all()
print(table.num_rows, table.schema.names)
```

The schema is identical across vendors, which makes it easy to land
multi-vendor data sets into the same Arrow / Parquet / DuckDB table.

## Pandas

```python
import openmassspec_io as op
import pandas as pd

table = op.read_arrow("sample.raw").read_all()
df = table.to_pandas()
print(df.head())
```
