# Shimadzu LabSolutions

Shimadzu LabSolutions `.qgd` (GC-MS) and `.lcd` (LC-MS) files are read
by [`openszraw`](https://github.com/Sigilweaver/OpenSZRaw), a pure-Rust
reader for the CFBF-container LabSolutions formats, clean-room
reverse-engineered with no Shimadzu SDK or software dependency. It
covers GC-MS (full-scan profile and MRM/targeted acquisition) and
LC-MS across both the IT-TOF and QTOF (LCMS-9030) instrument families.

## Detection

A path is treated as Shimadzu when:

- the filename has a `.qgd` or `.lcd` extension (case-insensitive), and
- its first 8 bytes are the CFBF/OLE2 container signature.

Unlike SCIEX's `.wiff`/`.wiff.scan` pair, Shimadzu raw files are
self-contained - there is no sibling file to corroborate against, so
the magic-byte check is the only content-level signal used.
`openszraw::reader::Reader` further distinguishes `.qgd` GC-MS from
`.lcd` IT-TOF from `.lcd` QTOF internally, from the file's own CFBF
stream layout, never from the filename alone - so a single
`shimadzu` feature and `VendorFormat` variant covers all three.

## Covered features

- `.qgd` GC-MS: full-scan profile and MRM/targeted acquisition.
- `.lcd` IT-TOF: run-length-encoded profile spectra, calibrated to
  physical m/z from the file's own embedded TOF tuning data.
- `.lcd` QTOF (LCMS-9030): centroid spectra.
- Indexed mzML with native-id format `Shimadzu Biotech nativeID format`
  (`MS:1000929`) or `Shimadzu Biotech QTOF nativeID format`
  (`MS:1002898`), depending on variant.

## Known gaps

- QQQ (triple-quadrupole) `.lcd` files use a distinct `TLM Raw Data`
  storage that is not yet decoded - see [OpenSZRaw issue
  #5](https://github.com/Sigilweaver/OpenSZRaw/issues/5).
- PDA/UV chromatogram streams (`PDA 3D Raw Data`, `LSS Raw Data`) are
  not decoded or exposed - see [OpenSZRaw issue
  #2](https://github.com/Sigilweaver/OpenSZRaw/issues/2).
- IT-TOF peaks at or below the calibration intercept have no m/z. They are
  left out of the spectrum, counted in `extra`, and the spectrum carries the
  warning `openszraw.uncalibrated_peaks_dropped`. Single-quad peaks that are
  out of m/z order are sorted with their intensities
  (`openszraw.mz_reordered`).
- IT-TOF per-channel polarity/MS-level and QTOF MS2 precursor m/z are
  not yet resolved. See [OpenSZRaw's known-limitations
  doc](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/06-known-limitations.md)
  for the full, evidence-backed list.

## Tested instruments

The conformance suite is tested against a real corpus spanning IT-TOF,
QTOF (LCMS-9030), and GC-MS/GC-MS-MS instrument families across PRIDE,
MassIVE, and MetaboLights accessions - see [OpenSZRaw's
CORPUS.md](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CORPUS.md).

## See also

- [OpenSZRaw on GitHub](https://github.com/Sigilweaver/OpenSZRaw).
