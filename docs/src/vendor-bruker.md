# Bruker timsTOF TDF

Bruker `.d/` bundles are read by [`opentimstdf`](https://github.com/nathanriley/OpenTDF),
which parses the SQLite metadata in `analysis.tdf` and the binary
frame data in `analysis.tdf_bin` without calling Bruker's `libtimsdata`.

## Detection

A path is treated as Bruker when:

- the directory name ends in `.d` (case-insensitive), and
- the directory contains both `analysis.tdf` and `analysis.tdf_bin`.

## Covered features

- TIMS-on and TIMS-off acquisitions (timsTOF Pro, fleX, Pro 2, HT).
- MS1, PASEF MS/MS, dia-PASEF.
- Per-peak inverse ion mobility array (Arrow `inv_mobility_per_peak`).
- Frame-merged MS1 spectra (one row per frame) plus per-precursor
  PASEF MS2 spectra.
- Indexed mzML with native-id format `Bruker TDF nativeID format`.

## Known gaps

- MALDI / imaging acquisitions (`MaldiFrameInfo`-based) are out of
  scope for the mzML writer in 0.x; the data is reachable via the
  lower-level `opentimstdf` API.
- IMS-MS quadrupole-window calibration is reported but not validated
  against vendor-side calibration curves.

## Tested instruments

The conformance suite exercises timsTOF Pro, fleX, Pro 2, and HT
acquisitions from public PRIDE datasets.
