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
pip install openmassspec[all]       # all standalone vendor packages
```

`openmassspec-io` (pulled in by the base install) has every vendor -
including SCIEX - compiled in, so reading and mzML conversion work for
every supported format out of the box. Each extra above additionally
installs the standalone per-vendor package for direct, vendor-specific
access. SCIEX has no standalone Python package yet, so there is no
`sciex` extra; `.wiff` reading is available from the base install.

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

op.to_mzml("sample.raw", "sample.mzML", indexed=True)
```

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

kind = op.detect("sample.raw")     # 'thermo' / 'bruker' / 'waters' / 'agilent' / 'sciex' / None
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
zero-copy numpy views over Rust-owned buffers. Each peak array can be
read exactly once (the buffer is moved into numpy on first access).

```python
import openmassspec_io as op

for s in op.iter_spectra("sample.raw"):
    if s.ms_level == 1:
        mz   = s.mz             # numpy.ndarray, dtype=float64
        intensity = s.intensity # numpy.ndarray, dtype=float32
        print(s.index, s.retention_time_sec, mz.size)
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
