# Thermo Finnigan RAW

Thermo `.raw` support is provided by [`opentfraw`](https://github.com/Sigilweaver/OpenTFRaw),
a pure-Rust reader that parses the Finnigan trailer-and-index layout
directly. **No Thermo `.NET` runtime, no `RawFileReader.dll`.**

## Detection

A path is treated as Thermo when:

- the filename ends in `.raw` (case-insensitive), and
- the path is a regular file (not a directory - that is Waters).

## Covered features

- Multi-controller files (MS, PDA, UV, analog).
- Centroided and profile spectra. A profile scan is exported as profile
  data (about twice the mzML size of a centroid list), and `--centroid`
  picks peaks from it.
- Scan-trailer fields: scan filter line, master scan, precursor m/z,
  isolation width, collision energy, polarity, MS level, analyzer.
- Indexed mzML output with native-id format `Thermo nativeID format`.

## Known gaps

- Some experimental scan modes (SIM, MRM with non-linear quad) are
  exposed as plain `SpectrumRecord` without specialized CV terms.
- The `software` element in mzML reports `opentfraw <version>`
  rather than `Xcalibur`. This is intentional: it identifies the tool
  that actually wrote the file.

## Tested instruments

The conformance suite exercises Thermo Q Exactive UHMR, Q Exactive
HF-X, Orbitrap Fusion Lumos, and Exploris 480 corpora. See the
`opentfraw` repository for the full matrix.

## See also

- [OpenTFRaw documentation](https://sigilweaver.app/opentfraw/docs/) -
  full reference, format notes, and changelog for the Thermo reader.
- [OpenTFRaw on GitHub](https://github.com/Sigilweaver/OpenTFRaw).
