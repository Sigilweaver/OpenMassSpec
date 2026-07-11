# openmassspec-io (Python)

Python bindings for the [`openmassspec-io`](https://github.com/Sigilweaver/OpenMassSpec)
Rust crate. Detect a Thermo / Bruker / Waters / Agilent / SCIEX acquisition on disk, convert
it to mzML, or stream spectra as zero-copy NumPy arrays / pyarrow record
batches.

```python
import openmassspec_io as opio

fmt = opio.detect("sample.raw")        # -> "thermo" | "bruker" | "waters" | None
opio.to_mzml("sample.raw", "sample.mzML", indexed=True)

for spec in opio.iter_spectra("sample.raw"):
    print(spec.native_id, spec.ms_level, spec.mz.shape)

# Optional Arrow stream (requires pyarrow):
reader = opio.read_arrow("sample.raw", batch_size=1024)
for batch in reader:
    print(batch.num_rows)
```

Build from source:

```
pip install maturin
maturin develop --release
```
