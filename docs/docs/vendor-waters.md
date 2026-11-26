# Waters MassLynx

Waters `.raw/` bundles are read by [`openwraw`](https://github.com/Sigilweaver/OpenWRaw),
a pure-Rust reader for the MassLynx on-disk layout (no MassLynx
runtime, no `DACServer.exe`).

## Detection

A path is treated as Waters when:

- the directory name ends in `.raw` (case-insensitive), and
- the directory contains `_HEADER.TXT`.

## Covered features

- Function-multiplexed files: each `_FUNC###.DAT` becomes a contiguous
  index range in the spectrum stream.
- Continuum and centroided spectra.
- Lock-mass / reference scans surfaced as separate functions.
- Drift-time profiles from Synapt G2-Si / G2-XS series (drift bins
  are decoded into `inv_mobility_per_peak` when the function has a
  populated TOF-IMS table).
- Indexed mzML with native-id format `Waters nativeID format`.

## Known gaps

- MSe DDA reconstruction (high/low energy pairing into precursor /
  product) is left to downstream tools; the writer emits the two
  energy traces as independent functions.
- The `_FUNCTNS.INF` ion-mode field is currently mapped to polarity
  only; alternative ion modes (e.g., MS/MS scanning with linked
  quadrupoles) are reported in `scan_mode` but not given a dedicated
  CV term.

## Tested instruments

The conformance suite exercises Waters Xevo G2-XS, Synapt G2-Si, and
Vion IMS QTof corpora.

## See also

- [OpenWRaw documentation](https://sigilweaver.app/openwraw/docs/) -
  full reference, format notes, and changelog for the Waters reader.
- [OpenWRaw on GitHub](https://github.com/Sigilweaver/OpenWRaw).
