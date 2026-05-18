# Quickstart: Python

```sh
pip install 'openproteo-io[arrow]'
```

## Detect and convert

```python
import openproteo_io as op

det = op.detect_format("sample.raw")
print(det.vendor, det.path)        # 'thermo' /path/to/sample.raw

op.to_mzml("sample.raw", "sample.mzML", indexed=True)
```

## Iterate spectra (numpy)

`iter_spectra` yields `Spectrum` objects whose peak arrays are
zero-copy numpy views over Rust-owned buffers. Each peak array can be
read exactly once (the buffer is moved into numpy on first access).

```python
import openproteo_io as op

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
import openproteo_io as op
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
import openproteo_io as op
import pandas as pd

table = op.read_arrow("sample.raw").read_all()
df = table.to_pandas()
print(df.head())
```
