# Vendor-reader parity matrix

A cross-vendor view of which fields each reader actually populates versus
which are hardcoded to `None`/a placeholder. Each vendor's own doc page
(see the Vendors section) covers its format in depth; this page exists so a
gap - like a field being `None` for every spectrum of one vendor while every
other vendor populates it - doesn't sit unnoticed the way
[OpenWRaw#8](https://github.com/Sigilweaver/OpenWRaw/issues/8) did before
this table existed.

Last verified: 2026-07-25 (chromatograms and FAIMS rows, by grepping
`iter_chromatograms`/`faims_cv` across all six vendor crates; all other
rows last verified 2026-07-18 (Shimadzu column) / 2026-07-15 (everything
else) - see git history for those verifications).

**Legend:** yes = populated from real decoded data · partial = populated in
some cases, see note · no = hardcoded `None`/placeholder, see note · N/A =
the field doesn't apply to this vendor's instrument class.

| Field | Thermo<br/>(OpenTFRaw) | Bruker<br/>(OpenTimsTDF) | Waters<br/>(OpenWRaw) | Agilent<br/>(OpenARaw) | SCIEX<br/>(OpenSXRaw) | Shimadzu<br/>(OpenSZRaw) |
|---|---|---|---|---|---|---|
| Precursor target/selected m/z | yes | yes (diaPASEF: target = selected = isolation m/z) | no ([#8](https://github.com/Sigilweaver/OpenWRaw/issues/8)) | partial: no `selected_mz`, format doesn't carry it | partial: DDE-cycle heuristic, "unknown" edge case | partial: `.qgd` MRM has real target/selected m/z; `.lcd` QTOF MS2 has none decoded ([#1002998 DDA stream](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/06-known-limitations.md) undecoded); `.lcd` IT-TOF has no precursor concept at all |
| Precursor charge | yes | partial: PASEF yes, diaPASEF no (DIA has no single charge) | no (no precursor at all, [#8](https://github.com/Sigilweaver/OpenWRaw/issues/8)) | no | no | no |
| Precursor CE (NCE vs eV distinguished) | yes | yes, always eV (correct for the format - `ce_is_nce` fixed `false`) | N/A (no precursor) | partial: value real, NCE/eV never distinguished | no | no |
| Polarity | yes | yes | yes | no: documented unrecoverable from format | no | no: documented unrecoverable from format (no polarity bit found in any of the three on-disk variants) |
| Filter string / equivalent | yes | N/A | N/A | N/A | N/A | N/A |
| Raw ion mobility (native units) | N/A (no IMS) | yes (scalar + per-peak) | partial: per-peak `drift_time_ms` populated for IMS scans, scalar `inv_mobility` left `None` | N/A | N/A | N/A (no IMS) |
| Calibrated CCS | N/A | no ([#14](https://github.com/Sigilweaver/OpenTimsTDF/issues/14)) | no ([#10](https://github.com/Sigilweaver/OpenWRaw/issues/10)) | N/A | N/A | N/A |
| Chromatograms (`iter_chromatograms`) | yes | no override | yes (closed [#9](https://github.com/Sigilweaver/OpenWRaw/issues/9)) | no override | no override | partial: LC detector (UV/RID) channels only, only confirmed populated on IT-TOF `.lcd` files; `PDA 3D Raw Data`/`LSS Raw Data` still not decoded - [OpenSZRaw#2](https://github.com/Sigilweaver/OpenSZRaw/issues/2) |
| Instrument model / CV resolution | yes, real lookup table | yes, real lookup table (5 of 10 entries had wrong PSI-MS accessions, fixed [2026-07-15](https://github.com/Sigilweaver/OpenTimsTDF/issues/15)) | yes for models with a unique CV term; 3 Synapt variants fall back to the generic term rather than guess between HDMS/MS ([#11](https://github.com/Sigilweaver/OpenWRaw/issues/11)) | yes, real lookup + documented fallback | no: hardcoded placeholder, investigated and documented as currently unresolvable from the format | partial: `.lcd` IT-TOF and QTOF each resolve a real specific PSI-MS instrument term; `.qgd` falls back to the generic "Shimadzu instrument model" term - no per-file model string found decoded anywhere in the corpus |
| Acquisition start timestamp | yes | yes (RFC 3339-validated) | yes (format bug fixed [2026-07-15](https://github.com/Sigilweaver/OpenWRaw/issues/11) - previously emitted `"14-Jan-2021 16:20:52"`, not valid RFC 3339) | yes | yes (OLE SummaryInformation stream, graceful `None` fallback) | no |
| Ion injection time | yes | yes | no | no | no | no |
| FAIMS compensation voltage | yes: wired into `to_msc_record` (closed [#27](https://github.com/Sigilweaver/OpenTFRaw/issues/27), shipped in 1.3.4) | N/A | N/A | N/A | N/A | N/A (Shimadzu instruments have no FAIMS interface) |

## Shared-layer gaps (affect every vendor equally)

These aren't per-vendor rows above because they're bugs in
`openmassspec-core`'s writer, not in any one reader's decode path.

- **Chromatograms had no path to output at all**, regardless of vendor,
  until `write_mzml`/`write_indexed_mzml` were wired to call
  `iter_chromatograms` ([OpenMassSpecCore#1](https://github.com/Sigilweaver/OpenMassSpecCore/issues/1),
  fixed in 1.2.0). Thermo, Waters, and Shimadzu now override
  `iter_chromatograms` with real data (see the row above); Bruker,
  Agilent, and SCIEX still don't.
- **`start_timestamp` was decoded by every vendor but silently dropped by
  the writer** ([OpenMassSpecCore#2](https://github.com/Sigilweaver/OpenMassSpecCore/issues/2),
  fixed in 1.2.0).

## Other known gaps not captured by the table above

- [OpenTimsTDF#13](https://github.com/Sigilweaver/OpenTimsTDF/issues/13) -
  PRM-PASEF frames (`msms_type=10`) are decoded but skipped in the mzML
  projection entirely (not a field-level gap - those spectra never appear
  in output at all).
- [OpenSXRaw#7](https://github.com/Sigilweaver/OpenSXRaw/issues/7) - the
  `ms_level` flag is wrong for SWATH/DDA-cycled acquisitions.
- OpenTFRaw's `PrecursorInfo.intensity` is hardcoded `None` in the
  `to_msc_record` conversion (`crates/opentfraw/src/mzml.rs`) even though
  every other precursor field is populated from real data. Not yet
  tracked by an issue.

## Keeping this current

Update this table (and the two sections above) whenever a linked issue
closes, a new gap is found, or a vendor crate gains a feature this table
says it lacks. `openmassspec-core` changes that affect every vendor at
once (the "shared-layer gaps" section) are the highest-leverage places to
check first.
