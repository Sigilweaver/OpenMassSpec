# SCIEX WIFF

SCIEX legacy `.wiff`/`.wiff.scan` files are read by [`opensxraw`](https://github.com/Sigilweaver/OpenSXRaw),
a pure-Rust reader for the CFBF-container `.wiff` format, clean-room
reverse-engineered with no SCIEX SDK or software dependency. It covers
the TripleTOF and QTRAP instrument families.

## Detection

A path is treated as SCIEX when:

- the filename has a `.wiff` extension (case-insensitive), and
- a sibling `<name>.wiff.scan` file exists alongside it.

A `.wiff` file with no paired `.wiff.scan` file is **not** detected -
`detect_format` returns `None` even though the extension matches,
because the reader needs the paired scan file to decode spectra.

## Covered features

- TripleTOF and QTRAP acquisitions.
- CFBF stream catalog and `.wiff.scan` block/token-stream decoding.
- TOF m/z calibration and MS2 precursor m/z.
- Indexed mzML with native-id format `SCIEX nativeID format`
  (`MS:1000823`).

## Known gaps

- `.wiff2`, SCIEX's newer self-contained format, is **not** supported.
  It was investigated in depth and found to use proprietary, non-
  standard page encryption that could not be resolved from the
  ciphertext alone; `.wiff2` support is deferred pending new
  information. See [OpenSXRaw's format
  writeups](https://github.com/Sigilweaver/OpenSXRaw/blob/main/docs/format/03-wiff2-container.md)
  for the investigation.
- The reader currently reports every spectrum as profile-mode / TOFMS
  analyzer regardless of actual instrument family - QTRAP records are
  nominal-mass, not true TOF. This is a simplification, not yet
  instrument-aware.

## Tested instruments

The conformance suite is tested against a real TripleTOF 5600 corpus
file (2228 scans decoded).

## See also

- [OpenSXRaw on GitHub](https://github.com/Sigilweaver/OpenSXRaw).
