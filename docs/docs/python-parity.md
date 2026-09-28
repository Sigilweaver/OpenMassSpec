# Python field parity

The shared Python contract follows `openmassspec-core`'s
`SpectrumRecord`, `PrecursorInfo`, `RunMetadata`, and `ChromatogramRecord`.
Every interpreted field in those records is available through
`openmassspec_io` and the top-level `openmassspec` re-exports. The
standalone vendor bindings expose the same records and additional decoded
vendor fields. Unknown bytes and unverified interpretations are outside
this contract.

## Shared API

| Rust record | Python access | Units and representation |
| --- | --- | --- |
| `SpectrumRecord` | `iter_spectra(path)` | One `Spectrum` at a time; `mz` is NumPy float64 and `intensity` is NumPy float32. |
| `PrecursorInfo` | `Spectrum.precursor` | Dictionary, or `None` for spectra without a precursor. |
| `RunMetadata` | `run_info(path)` | One `RunInfo`; CV terms are `{accession, name}` dictionaries. |
| `ChromatogramRecord` | `read_chromatograms(path)` | `Chromatogram` objects with NumPy float32 time and intensity arrays. |
| `SpectrumRecord` in Arrow | `read_arrow(path)` | 34 columns, including `acquisition_event_id` and `extra`. |

`Spectrum` exposes `index`, `scan_number`, `native_id`, `ms_level`,
`polarity`, `scan_mode`, `analyzer`, `acquisition_event_id`, `filter`,
`retention_time_sec`, `low_mz`, `high_mz`, `ion_injection_time_ms`,
`inv_mobility`, `faims_cv`, `precursor`, `mz`, `intensity`,
`inv_mobility_per_peak`, and `extra`. `reported_total_ion_current`,
`reported_base_peak_mz`, and `reported_base_peak_intensity` preserve the
nullable source values. The existing `total_ion_current`, `base_peak_mz`,
and `base_peak_intensity` properties compute a fallback from peaks when the
source value is absent.

`precursor` contains `target_mz`, `selected_mz`, `isolation_width`,
`charge`, `intensity`, `collision_energy`, `ce_is_nce`,
`precursor_native_id`, `activation`, `analyzer`, and `ccs`.

`RunInfo` exposes the source file name, source file format, native ID
format, instrument term and serial number, parser software name and
version, acquisition software name and version, start timestamp, mobility
array kind, analyzers, and `extra`. `Chromatogram` exposes index, ID, CV
type, precursor and product m/z, `time_sec`, and `intensity`.

Retention and chromatogram time are in seconds. Ion injection time is in
milliseconds. Collision energy follows `ce_is_nce`: normalized percent
when true, electron volts when false. Per-peak mobility follows the run's
`mobility_array_kind`. `extra` uses namespaced string keys such as
`opentfraw.scan_segment` and `openszraw.cycle_index`.

## Standalone vendor bindings

| Vendor | Canonical records | Additional decoded data |
| --- | --- | --- |
| Thermo `opentfraw` | `RawFile.iter_records()`, `run_info()`, `read_chromatograms()` | `scan()`, `iter_scans()`, profile and centroid arrays, sample and controller information, scan parameters, status and error logs. |
| Bruker `opentimstdf` | `Reader.iter_records()`, `run_info()`, `read_chromatograms()` | Frames and peaks, calibration, DIA/PASEF/PRM windows, targets, and precursors. |
| Waters `openwraw` | `RawReader.iter_records()`, `read_record()`, `run_info()`, `read_chromatograms()` | Function and header data, calibration, instrument parameters, scan index, all named status channels, IMS drift-time arrays. |
| Agilent `openaraw` | `iter_spectra(path)`, `RawReader.read_spectrum()`, `run_info()`, `read_chromatograms()` | `scan_index()` and `device_info()` with native MSScan and device fields. |
| SCIEX `opensxraw` | `iter_spectra(path)`, `RawReader.read_spectrum()`, `run_info()`, `read_chromatograms()` | `list_samples()`, `open_sample()`, `scan_index()`, instrument and calibration data, ion source settings, DDE precursor m/z values. |
| Shimadzu `openszraw` | `iter_spectra(path)`, `RawReader.read_spectrum()`, `run_info()`, `read_chromatograms()` | Variant, TTFL calibration, and physical scan index fields. |

The three smaller readers return `Spectrum` objects from their native
bindings. Thermo, Bruker, and Waters return dictionaries from
`iter_records()`. Native enum strings use the same lowercase spelling as
the umbrella binding. Native lists are converted to Python floats; cast
intensity values to float32 for exact comparisons with umbrella NumPy
arrays. All iterators are bounded. The legacy eager `RawReader` API for
Agilent, SCIEX, and Shimadzu remains available for random access.

For multi-sample SCIEX WIFF files, obtain a sample name with
`opensxraw.list_samples(path)` and pass it to `opensxraw.open_sample()`
or `openmassspec.open_run(path, sample=name)`. The direct umbrella
`iter_spectra(path)` path uses the single-sample reader.

## Validation

`crates/openmassspec-io-py/tests/test_parity.py` checks shared fields for
all six formats, compares first spectrum values with the installed native
binding, and checks Arrow's vendor fields on all six fixtures. Set
`OPENMASSSPEC_THERMO_RAW`, `OPENMASSSPEC_BRUKER_D`,
`OPENMASSSPEC_WATERS_RAW`, `OPENMASSSPEC_AGILENT_D`,
`OPENMASSSPEC_SCIEX_WIFF`, and `OPENMASSSPEC_SHIMADZU_RAW` to public
corpus acquisitions. Missing fixtures or native packages skip only their
dependent cases.
