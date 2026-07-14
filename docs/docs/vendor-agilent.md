# Agilent MassHunter

Agilent `.d/` directories are read by [`openaraw`](https://github.com/Sigilweaver/OpenARaw),
a pure-Rust reader for the MassHunter `AcqData` on-disk layout, clean-
room reverse-engineered with no Agilent SDK or MassHunter runtime.

## Detection

A path is treated as Agilent when:

- it is a directory, and
- it contains `AcqData/MSScan.bin`.

Bruker TDF bundles are also commonly named `<run>.d/`, so `detect_format`
checks for the Bruker signature (`analysis.tdf` + `analysis.tdf_bin`)
first; a bundle only falls through to the Agilent check once that
fails. See [Format detection](./format-detection.md) for the full
ordering.

## Covered features

- Q-TOF acquisitions: profile and centroid MS1/MS2.
- QQQ (MRM) acquisitions, including per-transition precursor and
  product native IDs.
- Indexed mzML with native-id format `Agilent MassHunter nativeID
  format` (`MS:1002848`).

## Known gaps

- `polarity` and the MS2 `selected_mz` (distinct from the isolation-
  window `target_mz`) are always `None`. Both were checked directly
  against the corpus - including a confirmed mixed-polarity run in
  `PXD031771` - and no recoverable field was found in `MSScan.bin` or
  `MSPeriodicActuals.bin`. See [OpenARaw's known-limitations
  notes](https://github.com/Sigilweaver/OpenARaw/blob/main/docs/format/06-known-limitations.md)
  for the byte-level investigation.
- A handful of PRIDE-hosted `.d` uploads are structurally malformed
  (missing `AcqData` entirely, or are macOS `__MACOSX` artifacts) and
  are excluded from the conformance corpus rather than misreported.

## Tested instruments

The conformance suite is validated against 332 of 338 real-world PRIDE
`.d` datasets, covering Q-TOF (profile and centroid) and QQQ (MRM)
acquisitions; the remaining 6 are the malformed uploads above, not
reader gaps.

## See also

- [OpenARaw on GitHub](https://github.com/Sigilweaver/OpenARaw).
