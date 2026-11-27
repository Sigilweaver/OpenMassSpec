# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

OpenMassSpec 3.0.0 builds on `openmassspec-core` 4 and the six reader majors
released against it: `opentfraw` 4.0.0, `opentimstdf` 4.0.0, `openwraw`
5.0.0, `openaraw` 2.0.0, `opensxraw` 0.5.0, `openszraw` 0.4.0. Python 3.10 is
the new minimum (wheels are abi3-py310).

### API breaks

- Python, `Spectrum.precursor` and the other record dicts: `collision_energy`
  and `ce_is_nce` are gone. `collision_energies` is a list of
  `{"value", "unit"}` dicts, one per energy (stepped energies have several),
  and `unit` is `"ev"`, `"nce"` or `None` when the file does not state it.
  Core 3 wrote `ce_is_nce: false` for energies whose unit the reader did not
  know; they are now `None`, not eV.
- Python: `Spectrum.retention_time_sec` is `None` when the reader could not
  state one (it was `0.0`). `Spectrum.analyzer`, `precursor["analyzer"]` and
  `RunInfo.analyzers` use the stack-wide labels (`"tof"`, not `"tofms"`).
  `Spectrum.base_peak_mz` and `base_peak_intensity` are `None` on a spectrum
  with no peaks (they were `0.0`).
- Python `to_mzml` returns `spectra_invalid`, `chromatograms_invalid`,
  `warning_counts`, `issues`, `issues_truncated` and
  `spectrum_count_mismatch` as well. It raises a `RuntimeWarning` for every
  warning code it wrote.
- Python `read_arrow` batches follow the schema of `openmassspec-arrow` 4:
  `precursor_ce_is_nce` is replaced by `precursor_collision_energy_unit`,
  and `precursor_collision_energies`, `scan_windows` and `warnings` are new.
  Arrow files written by 3.x have no schema version and are rejected.
- Rust: depends on `openmassspec-core` 4 and `openmassspec-arrow` 4, which
  `openmassspec_io::core` and `openmassspec_io::arrow` re-export. The Arrow
  builder is at `openmassspec_io::arrow::arrow_60::SpectrumBatchBuilder` and
  `push` returns a `BatchError`. `SpectrumRecord::retention_time_sec` is an
  `Option`, and a precursor holds `collision_energies`.
- The crates and Python packages are version 3.0.0.

### Behavior changes

Files that convert today can fail or read differently:

- `--centroid` (and `centroid=True`, `ConvertOptions::with_centroid`) picks
  peaks only from spectra declared profile. A spectrum with no declared scan
  mode is an error unless its format declares that it never states one (see
  the next item), and a "profile" spectrum that is a sparse peak list is an
  error.
- Formats that never state a value convert with warnings instead of an
  error. SCIEX `.wiff` never states the scan mode: each spectrum is written
  with neither the profile nor the centroid term and the warning
  `unknown_scan_mode` (it used to be written as "profile spectrum", which was
  a guess). Agilent Q-TOF scan records never state the collision energy
  unit: each MS/MS spectrum is written with a unit-less collision energy
  and the warning `unknown_collision_energy_unit` (Core 3 wrote eV). The run
  carries one `unstated_by_source` warning. `--centroid` leaves SCIEX
  spectra unchanged. No `--allow` is needed for either.
- Empty spectra carry the warning `empty_spectrum` and no base peak or m/z
  range (they were written with 0.0). Warnings are userParams in mzML
  (`openmassspec: warning CODE`), `Spectrum.warnings` in Python, and the
  `warnings` column in Arrow. `vendor2mzml validate` flags an mzML spectrum
  with no peaks as empty, so third-party mzML files with empty scans still
  validate.
- `<scanWindowList>` is written only where the reader states the instrument's
  acquisition range. Core 3 wrote the observed m/z range. OpenTimsTDF now
  states the acquisition range on every spectrum; the others write no
  window.
- The writer rejects malformed records (unsorted or non-finite m/z, array
  length mismatch, `ms_level` 0, a duplicate native id, a missing retention
  time in a run that has them, a collision energy with no unit that the
  format does not declare): `convert` counts them as skipped and lists the
  causes, `--strict` stops on them. These cannot be allowed with `--allow`.
- `vendor2mzml convert` exits 4 when records were skipped or the reader's
  spectrum count could not be reconciled, and prints the cause of each of the
  first skipped records. It used to exit 0 and print only a count. Warnings
  on records do not change the exit status.
- Reader changes that alter decoded values (see each reader's changelog):
  Thermo reads the collision energy unit from the file (NCE or eV; a unit the
  file does not state stays unknown), lists every collision energy step, and
  keeps a charge with no selected m/z in `extra` with a warning; Waters
  leaves out the collision energy of ramped and two-cell functions (the raw
  values stay in `extra`, with a warning) and drops a non-finite sample of a
  chromatogram with a flag; Shimadzu IT-TOF leaves out the peaks at or below
  the calibration intercept (counted, with a warning) and Shimadzu single
  quad files have their peaks sorted by m/z (counted, with a warning);
  Agilent MRM spectra no longer carry a `mrm_channel=N` precursor reference.
- `vendor2mzml info` no longer prints a retention time range of 0.0 for a
  run with no retention times; it omits the range and prints `no rt: N`.

### Added

- `vendor2mzml convert --spool-dir DIR` and Python `to_mzml(spool_dir=...)`:
  where the writer keeps encoded spectra once they pass 64 MiB, a file about
  as large as the output. The default is the output's directory (the Python
  default used the system temp dir, often a small RAM-backed `/tmp`).
- `vendor2mzml info` prints the checks a format declares it never states
  (`never stated:`), and a count per warning code; JSON adds `unstated`,
  `warning_counts` and `spectra_without_rt`. Python `RunInfo.unstated` and
  `RunInfo.warnings`, and `Spectrum.warnings`.
- `ConvertOptions::with_leniency` (also applies to centroiding);
  `vendor2mzml convert --allow CHECK` and `info --allow CHECK` (repeatable);
  Python `allow=` on `to_mzml`, `iter_spectra` and `read_arrow`;
  `collect_centroided_with` and `stream_centroided_with`. `--allow` is for a
  file that fails a strict check the format does not declare it never states:
  `unknown_scan_mode`, `unknown_collision_energy_unit` or
  `dangling_precursor_ref` (the reference is dropped).

## [2.1.2] - 2026-11-27

### Fixed

- Thermo: TSQ Vantage files whose scans do not state their peak count are read
  correctly (`opentfraw` 3.1.1, Sigilweaver/OpenTFRaw#100). With `opentfraw`
  3.1.0 (OpenMassSpec 2.1.1) the reader assumed a peak count of `data_size - 1`
  for these scans, so some decoded misaligned and a default `vendor2mzml
  convert` wrote wrong m/z values for them and skipped others. The reader now
  checks the count against the scan index and, when it disagrees, uses the one
  count that agrees. In the public PXD020246 acquisition (`TSQVantage.RAW`,
  `iota_sp_1-1.RAW` on PRIDE), 1,969 of 35,098 scans were affected (1,911 with
  unsorted or non-finite m/z, which 2.1.1 wrote to the mzML, 4 that failed to
  decode and were skipped, 54 with ascending but wrong values).
  `vendor2mzml convert` now writes all 35,098 spectra (2.1.1 wrote 35,094 and
  skipped 4), `validate` passes on the file and on the mzML
  (2.1.1 failed with `scan data record ending at offset 0x5a7dcb does not
  match a known layout`), and the 1,969 recovered scans carry the spectrum
  userParam `opentfraw.peak_count_recovered` in the mzML. The other 33,129
  scans decode identically. A scan for which no single count agrees would carry
  `opentfraw.peak_count_unverified`; PXD020246 has none. The `opentfraw` floor
  is 3.1.1 in `Cargo.toml` and in the `thermo` and `all` extras of
  `openmassspec`.

## [2.1.1] - 2026-11-26

### Fixed

- Waters: Encoding D scans that contain the intensity word `0x20000000` are
  decoded (`openwraw` 4.0.2, Sigilweaver/OpenWRaw#70), so `vendor2mzml
  convert`, `validate`, the Python API and `read_arrow` no longer refuse such
  files. With `openwraw` 4.0.1 (OpenMassSpec 2.1.0) these scans failed with
  `intensity word 0x20000000 has no mantissa`. The word is flag bit 29 with no
  mantissa; it now becomes a kept point with intensity 0.0. The public
  PXD053170 bundles `20231113_NSE_Sample_High.raw` (302 of 1677 function 1
  scans) and `20231113_NSE_Sample_NDS.raw` (282 of 1677) contain it, and no
  other bundle in the public corpus does. Other mantissa-zero words are still
  parse errors. Across 81 bundles (253,279 scans) the 584 scans above go from
  error to decoded and no other scan changes. The `openwraw` floor is 4.0.2 in
  `Cargo.toml` and in the `waters` and `all` extras of `openmassspec`.

## [2.1.0] - 2026-11-23

### Fixed

- Thermo `.raw`: profile scans are now exported as profile data. They were
  read as the reader's centroid peak list and labelled profile, and
  `--centroid` re-centroided that list. Affects every Thermo `.raw` file with
  profile-mode scans (for example Q Exactive and other Orbitrap acquisitions
  run in profile mode); scans acquired as centroid are unchanged. Output for
  Thermo profile files grows by about 2x (PXD068962, Q Exactive UHMR: 199,722
  to 1,157,028 points, 16.4 MB to 31.3 MB of mzML), and `--centroid` now
  keeps 199,687 peaks instead of 66,200. Python `iter_spectra` and
  `read_arrow` return the same points, so the `mz` and `intensity` arrays of
  profile scans are longer in the same way (PXD068962: 199,722 to 1,157,028
  points, about 5.8 times as many).
- Thermo `.raw`: every file is now read with the full scan packet
  (`opentfraw` reads the profile data, or the centroid list when the scan has
  none), including files with centroid scans only, whose output is unchanged.
  Cost, release build, three runs each: PXD068962 (all profile) converts in
  about 1.3 s before and after, with peak memory 44 MiB before and 59 MiB
  after. PXD031322 (Orbitrap Fusion Lumos, 150,020 of 152,328 scans
  centroid) converts in 46 to 50 s before and 47 to 52 s after, with peak
  memory 745 MiB before and after; its output grows from 988 MB to 1,121 MB
  because of its 2,308 profile scans.
- Thermo `.raw`: profile bins whose m/z cannot be converted (unknown
  calibration layout, m/z NaN) are dropped, and the number dropped per
  spectrum is recorded in the spectrum `extra` key
  `opentfraw.unconverted_profile_bins` (present only when it is above 0).
  None of the 3,047 spectra in PXD068962 has any.
- Thermo `.raw`: `opentfraw` 3.1.0 (3.0.0 before) corrects what is decoded
  from these files:
  - Multiplexed Q Exactive and Exploris scan events are read at their own
    record boundaries, with every precursor. On the public PXD021080 Q
    Exactive HF and HF-X PRM files, MS2 scans, most of which read as MS1
    before, now report the right MS order, scan filter and every precursor.
  - Orbitrap Exploris targeted-MS2 scan events that carry the name of an
    inclusion-list entry are read at their own record boundaries. No public
    file with such events is available, so this is covered by unit tests only.
  - TSQ SRM files in which transitions share a product m/z report the correct
    `precursor_mz` and scan filter strings for each transition.
  - Source CID energy is read from the method file as a compound-file stream.
    Where the method's sectors are not adjacent, `sid=<energy>` and the source
    CID extra fields were left out and now appear.
  - A scan-event reaction with a negative or non-finite precursor m/z is a
    decode error (`InvalidPrecursorMz`) instead of being dropped, so
    `convert` and `validate` can fail on Thermo files that passed before.
- Waters Encoding D intensities are decoded with the correct float layout
  (`openwraw` 4.0.1). The 4-byte intensity word was read as 16.16 fixed
  point, which gives roughly the logarithm of the intensity, so every peak and
  the noise floor landed between about 80 and 1000. The word is the compressed
  float that Encoding E uses (21-bit mantissa, 5-bit exponent,
  `mantissa * 2^(exponent - 21)`), and a word the format cannot produce is a
  parse error. Every Encoding D function is affected: SYNAPT G2, G2-S, G2-Si
  and XS, Vion, Xevo G2-XS and G3, and the non-lock-mass functions of mixed
  bundles. Intensities, TIC and base peak change in mzML output and in the
  Python API; m/z values and point counts are unchanged. Evidence: public
  PXD058812 `ITEM_M11_7_His_tag_01.raw`, scan 0, base peak 30594 at m/z 650.997
  (989.88 before), TIC 894,725 (4,056,044 before). Some scans in some files
  hit that parse error, so `convert` and `validate` can fail on Waters files
  that passed before; see Known issues in the README.
- `vendor2mzml validate` on an mzML file fails when fewer spectra can be read
  than the `<spectrumList count>` declares (a truncated file, or a spectrum
  that cannot be parsed), and when the file declares no count. These files
  were reported ok with the spectra that could be read.
- Python `Spectrum`: `mz`, `intensity` and `inv_mobility_per_peak` return the
  same array on every access. The second read used to return an empty array,
  and `total_ion_current`, `base_peak_mz`, `base_peak_intensity` and the
  `n_peaks` in `repr` depended on which arrays had been read first. They are
  now computed once, from the full peak list.
- `openmassspec`: when `openmassspec_io` fails to import, the failure is kept
  and each reader function (`to_mzml`, `iter_spectra`, `read_chromatograms`,
  `read_arrow`, `run_info`, `detect_format`, `detect`) raises an
  `ImportError` chained to it, naming the missing or mismatched module. The
  functions were set to `None`, so the call failed with `'NoneType' object is
  not callable` and the cause was lost. A wheel built without Arrow raises the
  same way from `read_arrow`.
- `vendor2mzml convert` finishes the gzip stream and syncs the output file
  explicitly, and reports an error from either step. The gzip trailer and the
  final buffer flush used to run when the writer was dropped, where an error
  could not be reported.
- `vendor2mzml convert` and `info` report an input path that cannot be read
  (missing, permission denied) as that I/O error. They said the path does not
  look like a supported vendor format.
- `vendor2mzml validate` reads gzipped mzML (`.mzML.gz`), recognised by the gzip
  magic bytes, including files made of several gzip members and files with
  trailing zero padding. It read 0 spectra from the compressed bytes and
  reported ok. Truncated or corrupt gzip input fails with the stream error,
  which names what was found: the file ending inside a member (a missing
  trailer included), a CRC or size mismatch, or bytes after the last member
  that are not zero padding.
- `vendor2mzml validate` accepts an mzML file that has no `<spectrumList>`
  (chromatograms only) when it ends with `</mzML>`. A spectrum list with no
  count still fails, and more spectra than the list declares is reported as
  that, not as truncation.
- `vendor2mzml validate` reports an input path that cannot be read (missing,
  permission denied) as that I/O error, as `convert` and `info` do. It said
  the path is not a recognized vendor format or mzML file.
- `vendor2mzml validate` exit codes change for scripts. A missing path exits 1
  (a missing path not named `*.mzml` exited 2). An mzML file
  with more spectra than its list declares, or with no count, exits 1 (it
  exited 0). A `.mzML.gz` is read and checked like any mzML file, so it can
  exit 3 on a conformance failure and 1 on a bad gzip stream, where it exited
  0 with 0 spectra.
- `vendor2mzml validate` checks the ending of an mzML file that declares 0
  spectra: a file cut off after the empty spectrum list or inside the run
  fails instead of reporting ok.

### Changed

- Builds on `openmassspec-core` 3.0.0 and the readers `opentfraw` 3.1.0,
  `opentimstdf` 3.0.1, `openwraw` 4.0.1, `openaraw` 1.0.0, `opensxraw` 0.4.0
  and `openszraw` 0.3.0. The `opentfraw` floor is 3.1.0 and the `openwraw`
  floor is 4.0.1, in `Cargo.toml` and in the `thermo`, `waters` and `all`
  extras of the `openmassspec` Python package, so a new install gets the
  readers this release was tested with. `opentimstdf`
  3.0.1 is a dependency update with no change to decoded data (see Security).

### Security

- Updated `opentimstdf` to 3.0.1, which moves `rusqlite` 0.40.2 to 0.42.0
  and `libsqlite3-sys` 0.38.2 to 0.40.0 in the lockfile, fixing
  RUSTSEC-2026-0459. The `bruker` Python extra now requires
  `opentimstdf>=3.0.1,<4`. No code or API changes.
- Updated the transitive `zstd` 0.14.3, `zstd-safe` 8.2.0 and `zstd-sys`
  2.1.2 (via `opentimstdf`) in the lockfile, fixing
  RUSTSEC-2026-0391, RUSTSEC-2026-0398, RUSTSEC-2026-0403 and
  RUSTSEC-2026-0410, plus the `zstd-safe` unsoundness warnings
  RUSTSEC-2026-0395 through -0397, -0399 through -0402 and -0404 through
  -0406. Lockfile-only change; no code or API changes.

## [2.0.0] - 2026-10-11

### Breaking (Rust)

- Requires `openmassspec-core` 3.0.0 and the reader releases built on it:
  `opentfraw` 3.0.0, `opentimstdf` 3.0.0, `openwraw` 4.0.0, `openaraw`
  1.0.0, `opensxraw` 0.4.0, `openszraw` 0.3.0. Core is re-exported as
  `openmassspec_io::core`.
- The `vendor` module (re-exports of the six reader crates) is removed.
  The readers are private dependencies, and no reader type is part of the
  public API, so a reader major release no longer forces a new
  `openmassspec-io` major. Depend on a reader crate directly to use it.
- The per-reader `Error` variants (`Thermo`, `Bruker`, `Waters`,
  `Agilent`, `Sciex`, `Shimadzu`) are replaced by
  `Error::Reader { vendor, source }`, with the reader's error boxed as
  `BoxError`. It covers both open failures and unreadable records
  (which were `Error::Core`). `Error::reader_error()` returns the boxed
  error for `downcast_ref`, `Error::reader_vendor()` the vendor name, and
  `Error::reader(vendor, err)` builds one. `Error::Core` now holds only
  writer and conformance errors from core.
- The `arrow` feature now depends on `openmassspec-arrow` 3.0.0 and
  `openmassspec_io::arrow` re-exports that crate (it was
  `openmassspec_core::arrow`). Arrow types appear in the public API only
  through this re-export.
- Every function that took `Detected` by value takes `&Detected`:
  `collect`, `collect_centroided`, `stream`, `stream_centroided`,
  `metadata_only`, `collect_chromatograms`, `convert_to_mzml`,
  `convert_to_mzml_writer`.
- `convert_to_mzml(&Detected, &Path, &ConvertOptions)` and
  `convert_to_mzml_writer(&Detected, &mut W, &ConvertOptions)` replace the
  `indexed: bool` argument with `ConvertOptions` and return
  `openmassspec_core::WriteSummary` instead of `()`.
  `convert_to_mzml_centroided` and `convert_to_mzml_writer_centroided` are
  removed; use `ConvertOptions::with_centroid` and
  `with_centroid_min_intensity`.
- `collect`, `collect_centroided`, `stream`, `stream_centroided`, and
  `collect_chromatograms` return an error (`Error::Reader`) for the first
  spectrum or chromatogram the reader cannot decode. Previously the
  readers dropped such records without notice.
- `convert_to_mzml` stops at the first unreadable record by default.
  `ConvertOptions::with_record_error_policy(RecordErrorPolicy::Skip)`
  leaves such records out and counts them in the returned `WriteSummary`.
- `Error`, `VendorFormat`, `Detected`, and `VecSource` are
  `#[non_exhaustive]`. Build `Detected` with `Detected::new`; build
  `VecSource` with `VecSource::new`; match `Error` and `VendorFormat` with
  a wildcard arm.
- `VecSource` implements the core 3.0 `SpectrumSource` (yields `Ok`
  records).

### Breaking (CLI)

- `vendor2mzml convert` leaves out spectra and chromatograms the reader
  cannot decode, prints the skipped count on stderr as a warning, and
  exits 0 when the output was written. `--strict` stops at the first
  unreadable record, exits 1, keeps the partial output file, and says on
  stderr that it is incomplete. `info` and `validate` stop at the first
  unreadable spectrum.
- `convert --profile` output reports `spectra`, `spectra_skipped`,
  `chromatograms`, and `chromatograms_skipped` from the write, and drops
  the `validated` field (it was always `false`; `spectra` was always 0).

### Changed (Python)

- `to_mzml` takes `skip_unreadable=True`: unreadable records are left out,
  a `RuntimeWarning` reports the count, and the call returns a dict with
  `spectra_written`, `spectra_skipped`, `chromatograms_written`, and
  `chromatograms_skipped` (it returned `None`). `skip_unreadable=False`
  raises `RuntimeError` at the first one.
- `iter_spectra`, `read_arrow`, and `read_chromatograms` raise
  `RuntimeError` on an unreadable record instead of omitting it.
- The metapackage extras and the CI parity install require the new reader
  releases: `opentfraw>=3.0.0,<4`, `opentimstdf>=3.0.0,<4`,
  `openwraw>=4.0.0,<5`, `openaraw>=1.0.0,<2`, `opensxraw>=0.4.0,<0.5`,
  `openszraw>=0.3.0,<0.4`.

### Added

- `ConvertOptions` (indexing, centroiding, compression, record-error
  policy) and `with_source`, which opens the vendor source for a
  `Detected` and passes it to a closure as `&mut dyn SpectrumSource`.
- `Detected::new` to pair a path with a format without probing.
- `convert_to_mzml` flushes its output file and reports a flush failure
  instead of losing it on drop.
- Container image for `vendor2mzml`: release tags publish
  `ghcr.io/sigilweaver/vendor2mzml:<version>` and `:latest`, built from
  the `Dockerfile` with `--locked` and smoke-tested on every pull request.
- `Cargo.lock` is committed, so the CLI, wheels and image build against
  the same dependency versions.
- A bioconda recipe draft under `packaging/bioconda` (not yet submitted).
- Python now exposes the complete shared spectrum and precursor fields, full
  run metadata, chromatograms, and the new Arrow vendor metadata columns for
  all six parsers. `open_run` accepts a SCIEX sample name.
- Real-file Python parity tests compare the umbrella and native bindings.
- Arrow batches now stream through a bounded buffer instead of materializing
  the complete run before returning a `RecordBatchReader`.
- `acquisition_event_id` and `extra` columns in the Arrow schema.

### Changed

- Arrow data uses arrow 60. Rust code exchanging Arrow data with this
  crate must use arrow 60.
- `read_arrow` raises if decoding fails partway through, instead of ending
  the stream early.
- `mzdata` 0.65 -> 0.67 (CLI round-trip checks only).
- The `openmassspec-io` crate ships its own `LICENSE` copy, and the
  `openmassspec` metapackage uses PEP 639 license metadata. README links
  are absolute so they resolve on crates.io and PyPI.
- `openmassspec-io-cli` is marked `publish = false`; install `vendor2mzml`
  from a release tag, the release binaries, or the container image.
- `vendor2mzml convert` writes exact `<spectrumList count>` values and
  consecutive `index` attributes (from `openmassspec-core` 3.0). The
  writer holds encoded spectra until the run is read to the end, spilling
  past 64 MiB to a temporary file; `convert` puts that file in the output
  file's directory instead of the system temp dir, and removes it when
  the conversion ends.

### Fixed

- `vendor2mzml validate` no longer panics on mzML spectra without binary
  arrays (the writer omits them for empty spectra), and reports a
  spectrum with only one of the two arrays as an error. It now reads the
  isolation window target m/z and the precursor `spectrumRef`, so Agilent
  Q-TOF MS2 spectra are no longer flagged as missing a precursor.

## [1.5.5] - 2026-09-17

### Fixed

- Re-pinned `opentfraw` to 1.4.1, which corrects Orbitrap profile m/z
  positions by applying Thermo's per-chunk calibration correction in the m/z
  domain rather than as a pre-conversion frequency offset.

## [1.5.4] - 2026-08-12

### Added

- Added Agilent (`.d`, MassHunter) and SCIEX (`.wiff`) smoke/centroid/
  stream-vs-collect test coverage to
  `crates/openmassspec-io/tests/vendor2mzml.rs`, which previously only
  exercised Thermo, Waters, Bruker, and Shimadzu even though both vendors
  are already wired into `openmassspec-io` behind the `agilent`/`sciex`
  feature flags. `.github/workflows/ci.yml`'s corpus-download step now
  also fetches the small Agilent QTOF (PXD030293, ~960 KB zipped) and
  SCIEX (PXD022088, ~2.5 MB `.wiff`+`.wiff.scan` pair) fixtures already
  validated by OpenARaw#28 and OpenSXRaw#33 respectively, so these six
  new tests run for real in CI rather than skipping. (Sigilweaver/OpenMassSpec#25)

### Changed

- Re-pinned the vendor readers and `openmassspec-core` to their latest
  releases: openmassspec-core 1.5.0, opentfraw 1.4.0, opentimstdf 1.3.3,
  openwraw 1.2.9, openaraw 0.1.7, opensxraw 0.2.5, openszraw 0.1.4. The
  core bump adds the defaulted `extra` / `acquisition_event_id` record
  fields (projected as mzML `userParam`s); no reader populates them yet,
  so output is unchanged.
- Wired real corpus fixtures into CI for `crates/openmassspec-io/tests/vendor2mzml.rs`
  and `crates/openmassspec-io-cli/tests/cli_centroid.rs`, which previously
  always skipped in CI because they checked hardcoded sibling-checkout
  paths (e.g. `../../../SpecLance/corpus/...`) that never exist there.
  `.github/workflows/ci.yml`'s `rust` job now downloads the same six
  small Thermo/Waters/Bruker/Shimadzu (PRIDE, MetaboLights, and MassIVE)
  fixtures already used elsewhere in this suite-wide effort into a
  gitignored repo-root `corpus/` dir (Linux leg only, to avoid tripling
  the ~140 MB download across the 3-OS matrix), and both test files now
  look there first, falling back to the old sibling-checkout paths for
  local dev. (Sigilweaver/OpenMassSpec#24)

## [1.5.3] - 2026-07-29

### Fixed

- Adapted the `cli_validate` test fixture's `RunMetadata` literal to
  `openmassspec-core` 1.4.0's new `acquisition_software_name`/
  `acquisition_software_version` fields (`None, None` - synthetic
  fixture, no real vendor data to source a value from). Bumped the
  stale `openmassspec-core = "1.2.0"` workspace floor to `"1.4.0"`, and
  the six vendor-crate floors (`opentfraw`, `opentimstdf`, `openwraw`,
  `openaraw`, `opensxraw`, `openszraw`) to the releases that actually
  compile against it, now that all six have published their own
  core-1.4.0 adaptations. (Sigilweaver/OpenMassSpec#22)
- Bumped the workspace `rust-version` (and the MSRV CI job) from 1.88 to
  1.95. `opentimstdf`'s `rusqlite ^0.40` dependency (pulled in only by
  the optional `bruker`/`all` feature) now resolves to
  `libsqlite3-sys 0.38.x`, whose bundled-build path uses the
  `cfg_select!` macro stabilized in Rust 1.95 - the MSRV check job was
  failing on `main` as a result. No older `libsqlite3-sys` release
  compatible with `rusqlite ^0.40` predates this, and downgrading
  `opentimstdf`'s own `rusqlite` pin or dropping the `bundled` feature
  (trading it for a system-sqlite3 build dependency on every platform)
  were judged worse tradeoffs than raising the floor. Default-feature
  builds (no Bruker support) were never actually affected; this only
  changes the declared floor for the `bruker`/`all` feature combination
  the `--all-features` CI job tests.
- Python's `openmassspec.detect()`/`open_run()` now delegate to the
  Rust `openmassspec_io.detect_format` binding instead of
  reimplementing the vendor structural checks by hand. The two
  implementations had drifted: the Python side never checked
  Shimadzu's CFBF/OLE2 magic bytes the way `detect_format` does (so a
  renamed/garbage `.qgd`/`.lcd` file was wrongly reported as
  Shimadzu), and it matched a wider, undocumented set of Waters
  sentinel files (`_FUNCTNS.INF`, `_extern.inf`, `_HEADER.TXT`) than
  the canonical `_HEADER.TXT`-only check. There is now a single source
  of truth for format detection. Fixed by @Nabejo. Closes #19.
- `vendor2mzml`'s unrecognized-format error message and the `convert`
  subcommand's `input` help text now list all six supported vendors
  (Agilent, SCIEX, and Shimadzu were missing from both). `scripts/
  release-stack.sh`'s header comment now describes the current
  eight-repo stack instead of the stale five-repo one from before
  OpenARaw/OpenSXRaw/OpenSZRaw landed. Text-only change, no behavior
  change. Fix contributed by @Nabejo. Closes #18.

## [1.5.2] - 2026-07-20

### Security

- Bumped `opentfraw` 1.3.4 -> 1.3.5 and `openaraw` 0.1.2 -> 0.1.3,
  picking up allocation-cap fixes for untrusted length-prefixed fields
  in both vendor decoders (memory-exhaustion/DoS on crafted or corrupt
  input). Bumped `opensxraw` 0.2.1 -> 0.2.2 for the same class of fix in
  `read_scan_block`.

### Fixed

- Bumped `openwraw` 1.2.5 -> 1.2.6: precursor `target_mz`/
  `collision_energy` and chromatogram export (pump pressure, flow rate,
  temperature) are now populated instead of always empty/`None`.
- Bumped `openszraw` 0.1.0 -> 0.1.1: `start_timestamp` is now populated
  from CFBF directory-entry creation times for all three Shimadzu
  on-disk variants.

## [1.5.1] - 2026-07-20

### Security

- Bumped `mzdata` from 0.63.5 to 0.65.4 in `openmassspec-io-cli`, pulling
  in `mzdata`'s own quick-xml 0.30 -> 0.41 upgrade
  ([mobiusklein/mzdata#53](https://github.com/mobiusklein/mzdata/pull/53))
  and clearing RUSTSEC-2026-0194/0195. No API changes needed on our side;
  `mzml_reader.rs`'s usage of `MzMLReader`/`RawSpectrum`/`prelude` traits
  is unaffected by the intervening 0.64.x/0.65.x releases. The `cargo
  audit` CI job no longer needs to `--ignore` either advisory. Closes #4.

## [1.5.0] - 2026-07-18

### Added

- Sixth vendor: Shimadzu LabSolutions `.qgd` (GC-MS) and `.lcd` (LC-MS,
  IT-TOF/QTOF) files, via a new `shimadzu` feature backed by
  [`openszraw`](https://github.com/Sigilweaver/OpenSZRaw) 0.1.0.
  `detect_format` identifies Shimadzu files by `.qgd`/`.lcd` extension
  plus a CFBF/OLE2 magic-byte check (no sibling file to corroborate
  against, unlike SCIEX's `.wiff`/`.wiff.scan` pair). All three on-disk
  Shimadzu variants (`.qgd`, `.lcd` IT-TOF, `.lcd` QTOF) share one
  `VendorFormat::ShimadzuLabSolutions` and one `shimadzu` feature, since
  `openszraw::reader::Reader` already auto-detects between them from
  the file's own CFBF stream layout. Wired into every dispatch path
  (`convert_to_mzml`, `collect`, `stream`, `metadata_only`, and their
  `_centroided` variants), the `openmassspec` Python metapackage's
  `detect`/`open_run`, and the `all` meta-feature.
- Added `OpenSZRaw` to `scripts/release-stack.sh`'s tracked repos and
  `STACK.md`'s pin table.

### Fixed

- `crates/openmassspec-io/tests/vendor2mzml.rs`'s `smoke()`/
  `centroid_smoke()` helpers derived their temp output filename from
  `VendorFormat::name()` + process id alone, which collided whenever
  more than one smoke test shared a vendor name (as Shimadzu's three
  on-disk variants now do) - tests run in parallel by default, so two
  tests racing on the same temp path caused spurious "No such file or
  directory" failures. Fixed by adding a sanitized per-input-file tag
  to the temp filename.

## [1.4.0] - 2026-07-13

### Added

- `openmassspec-io` gains `stream`/`stream_centroided` (visit each spectrum
  via a callback as it's decoded, instead of buffering the whole run into
  a `Vec` like `collect`/`collect_centroided` do) and `metadata_only` (read
  run-level metadata without decoding any spectra at all). Memory is now
  bounded by whatever the caller retains, not by acquisition size.
- The Python binding's `iter_spectra` now decodes on a background thread
  and hands spectra across one at a time through a bounded channel,
  instead of eagerly collecting every spectrum before the first one
  reaches Python. `run_info` uses `metadata_only` and no longer decodes
  any spectra, and `read_arrow` builds its Arrow batches incrementally
  via `stream`/`stream_centroided` instead of materializing the whole run
  as `SpectrumRecord`s first. Closes #3.
- Bumped `openwraw` to 1.2.3 and `opentimstdf` to 1.2.5: both crates'
  `iter_spectra` previously buffered (and in Waters' case, also cloned)
  the whole run into a `Vec` before yielding anything, regardless of how
  it was consumed. Without this bump, `stream`/`stream_centroided` would
  call through to an eager `iter_spectra` for the Waters and Bruker
  backends specifically, silently defeating the memory-bounding this
  release adds for those two vendors.

### Security

- Upgraded `pyo3`/`numpy` from 0.28 to 0.29 and `arrow` from 58 to 59 in
  `openmassspec-io-py`, clearing RUSTSEC-2026-0176 and RUSTSEC-2026-0177.
  Required bumping the `openmassspec-core` dependency to 1.1.1 (its
  `arrow` feature needed the matching arrow bump, since only one pyo3
  version can be linked into a given binary). The `cargo audit` CI job
  no longer needs to `--ignore` those two advisories; it still ignores
  RUSTSEC-2026-0194/0195 (quick-xml), which is a transitive dependency
  via `mzdata` and blocked upstream - `mzdata` pins `quick-xml = "^0.30"`
  even on its `main` branch. Tracked in #4.

### Versioning note

- This release jumps straight to **1.4.0**, not the normal next
  increment. Pre-rename (OpenProteo) this repo's tag lineage reached
  `v1.3.0`; post-rename versioning restarted at `1.0.0` and had not yet
  caught back up (was at 1.2.0/1.2.1), so every release through `1.3.x`
  needed the `-openmassspec` tag-suffix workaround to avoid colliding
  with the old plain-numbered tags. This jump clears the old lineage in
  one step; every release after this one is a normal bump with no
  suffix check needed for this package.

## [1.2.1] - 2026-07-12

### Fixed

- The `openmassspec` metapackage's `python/pyproject.toml` version was
  left at 1.1.0 during the 1.2.0 cut, so its PyPI publish step silently
  no-opped (`skip-existing`) instead of uploading. No functional change;
  version-only fix so the metapackage actually reaches PyPI at parity
  with `openmassspec-io` 1.2.0.

## [1.2.0] - 2026-07-12

### Added

- Optional centroiding across the whole stack, built on
  `openmassspec-core` 1.1.0's new `Centroided` adapter (local-maxima peak
  picking; already-centroid spectra pass through unchanged). Off by
  default everywhere - profile data is never silently discarded.
  - Rust: `collect_centroided`, `convert_to_mzml_centroided`, and
    `convert_to_mzml_writer_centroided` in `openmassspec-io`, each taking
    a `min_intensity: Option<f32>` noise floor alongside the existing
    `collect` / `convert_to_mzml` / `convert_to_mzml_writer`.
  - CLI: `vendor2mzml convert --centroid [--centroid-min-intensity <f32>]`
    and `vendor2mzml info --centroid [--centroid-min-intensity <f32>]`.
  - Python: `centroid` / `centroid_min_intensity` keyword arguments on
    `to_mzml`, `run_info`, `iter_spectra`, and `read_arrow`.

## [1.1.0] - 2026-07-11

### Added

- Agilent and SCIEX support, so the umbrella now covers all five vendor
  readers in the stack through one API:
  - New `agilent` feature (pulls in [`openaraw`](https://github.com/Sigilweaver/OpenARaw))
    and `sciex` feature (pulls in [`opensxraw`](https://github.com/Sigilweaver/OpenSXRaw)).
    Both are included in the `all` meta-feature.
  - `detect_format` now recognizes Agilent MassHunter `.d/` bundles (by
    `AcqData/MSScan.bin`) and SCIEX legacy `.wiff` files (by a `.wiff`
    extension with a sibling `.wiff.scan`).
  - `VendorFormat::AgilentMassHunter` and `VendorFormat::SciexWiff`
    variants; `convert_to_mzml` / `convert_to_mzml_writer` / `collect`
    dispatch to them.
  - `vendor2mzml` CLI handles both automatically (built with `all`).
  - Python: `.wiff` and Agilent `.d/` read through the base
    `openmassspec` install (the binding compiles in every vendor). New
    `openmassspec[agilent]` extra installs the standalone `openaraw`
    package; SCIEX has no standalone Python package yet, so there is no
    `sciex` extra (reading still works from the base install).

## [1.0.1] - 2026-07-11

### Fixed

- `opentfraw`/`opentimstdf`/`openwraw` workspace dependency requirements
  were left at their pre-rename minimums (`1.2.0`/`1.0.6`/`1.0.5`), which
  a fresh consumer could satisfy with an old vendor-crate version still
  depending on `openproteo-core` - a different (incompatible) trait from
  `openmassspec-core`, breaking the build. Bumped to `1.3.1`/`1.2.3`/`1.2.2`,
  the first versions of each that depend on `openmassspec-core`.

## [1.0.0] - 2026-07-10

Renamed from OpenProteo / `openproteo-io` / `openproteo`. The vendor
raw-file readers this stack wraps (Thermo, Bruker, Waters, with Agilent
and SCIEX joining the suite) are used as much in metabolomics and
lipidomics as in proteomics, so the umbrella naming moves from
proteomics-specific to general mass spectrometry. `openmassspec-core`
(the shared foundation crate) made the same move; see its own
CHANGELOG for that side of the rename.

No API or behavioral changes from `openproteo-io`/`openproteo` 1.3.0.
Version reset to 1.0.0 to reflect that these are new package identities
on crates.io and PyPI (the old `openproteo-io`/`openproteo` names stay
published and frozen at 1.3.0, they are not superseded in place). See
[OpenProteo's CHANGELOG](https://github.com/Sigilweaver/OpenProteo/blob/main/CHANGELOG.md)
for pre-rename history.

### Changed

- Repo renamed OpenProteo -> OpenMassSpec.
- Crates renamed `openproteo-io` -> `openmassspec-io`,
  `openproteo-io-cli` -> `openmassspec-io-cli`,
  `openproteo-io-py` -> `openmassspec-io-py`.
- PyPI packages renamed `openproteo-io` -> `openmassspec-io` (bindings),
  `openproteo` -> `openmassspec` (facade).
- Dependency on the shared core crate updated to `openmassspec-core` 1.0.0.

## [1.3.0] - 2026-07-06

### Added

- `openproteo-io` (Python): `read_polars()`, a thin wrapper over the
  existing zero-copy `read_arrow()` API returning a Polars `DataFrame`
  directly. Gated behind a new optional `polars` extra
  (`pip install openproteo-io[polars]`).

### Fixed

- `CITATION.cff`: corrected the abstract, which claimed the Python
  bindings "integrate with Polars, PyArrow, and Pandas" as a blanket
  statement; now names the actual `read_arrow()`/`read_polars()` APIs
  directly.

## [1.2.1] - 2026-07-04

### Changed

- `publish.yml`: crates.io publish step now uses `continue-on-error: true`
  so re-triggered tag runs (e.g. after a pyproject.toml fix) do not fail
  the whole workflow when the crate version was already published.
- Bundle `LICENSE` into both package sdists: `openproteo-io` via a maturin
  `include`, `openproteo` via setuptools `license-files`. Enables
  source-based installs and conda-forge packaging (which build from
  source). Added `RELEASING.md`, the release SOP.

## [1.2.0] - 2026-07-02

### Added

- `Spectrum.scan_mode` (Python): `"centroid"`, `"profile"`, or `None`.
  Populated by all three vendor adapters; previously decoded in
  `SpectrumRecord` but not surfaced to Python.
- `Spectrum.analyzer` (Python): mass analyzer family as a lowercase
  string (`"itms"`, `"tqms"`, `"sqms"`, `"tofms"`, `"ftms"`,
  `"sector"`) or `None`. Populated by all three vendor adapters.
- `Spectrum.filter` (Python): Thermo-style scan filter string, or
  `None` for Bruker and Waters files.
- `Spectrum.ion_injection_time_ms` (Python): ion injection /
  accumulation time in milliseconds, or `None`.
- `Spectrum.low_mz` / `Spectrum.high_mz` (Python): observed m/z
  range endpoints, or `None`.
- `RunInfo` class and `run_info(path)` function (Python): returns
  run-level metadata (instrument name and CV accession, source file
  name, acquisition timestamp, parser software name and version)
  without iterating spectra.

### Changed

- `opentfraw` dependency bumped from `1.0.6` to `1.2.0`. Picks up
  the Exploris scan-event calibration fix (profile m/z and MS2
  precursor now decoded correctly on Exploris instruments) and the
  new per-peak FT label, scan-parameters, profile, and
  created-timestamp APIs contributed by @oskarsari.

## [1.1.0] - 2026-05-31

### Added

- `CITATION.cff`: author identity (Nathan Riley + ORCID) and a
  scaffolded `identifiers:` block ready for the Zenodo concept DOI.

### Changed

- Workspace MSRV raised from `1.87` to `1.88` (mzdata uses
  `slice_as_chunks`).
- CI: maturin-develop step now provisions a venv before invoking
  maturin, so wheel builds succeed on Ubuntu runners with PEP 668.
- Cloudflare Pages deploy moved off the `wrangler` GitHub Action;
  the Cloudflare GitHub App now handles deploys.
- Docusaurus navbar adds a Core link to docs.rs/openproteo-core
  (WP15).
- Workspace metadata hygiene (WP13): authors, repository, homepage,
  documentation, readme, keywords, categories declared once under
  `[workspace.package]` and inherited.
- README badge block unified across the Sigilweaver portfolio.

## [1.0.3] - 2026-05-22

CI / workflow correctness release. No runtime behaviour changes.

### Changed

- Workspace MSRV raised from `1.85` to `1.87` to correctly declare the
  minimum Rust version required by `mzdata 0.63.x` (`slice_as_chunks`
  was stabilised in Rust 1.87.0). The `1.85` declaration shipped in
  `1.0.2` was incorrect.
- `release.yml`: removed stale multi-repo checkout steps that were
  left over from the path-dependency era. All vendor-crate deps resolve
  from crates.io so only the umbrella repo needs to be checked out.
- `wheels.yml` deleted; the wheel build and PyPI publish are now
  handled entirely by `publish.yml`.
- CI `MSRV check` updated from `1.85` to `1.87` to match.

## [1.0.2] - 2026-05-22

Test + packaging release. No runtime behaviour changes; the umbrella
crates, the Python metapackage, and the `openproteo-io` Python wheel
are now publishable from a single tag, and the `openproteo`
metapackage drift bug is fixed.

### Added

- PyPI publish wiring in `.github/workflows/publish.yml`:
  - maturin wheel matrix (linux x86_64/aarch64, macos x86_64/aarch64,
    windows x86_64) + sdist for `openproteo-io`,
  - pure-Python sdist + wheel build for the `openproteo` metapackage,
  - PyPI uploads gated on the `pypi` GitHub environment via OIDC
    trusted publishing (no API tokens in the repo).
- Expanded `python/tests/test_metapackage.py`: dispatch tests for
  `open_run()` (thermo / bruker / waters with monkeypatched vendor
  modules), `__version__` vs installed-metadata drift check,
  immutable-`VENDORS` and full `__all__` surface check.

### Changed

- `openproteo.__version__` now derives from
  `importlib.metadata.version("openproteo")` rather than a hard-coded
  literal. This fixes the silent `0.2.0` vs `1.0.x` drift the previous
  literal could produce.
- Workspace pin: `openproteo-core = 1.0.1` (docs-only patch upstream).
- Workspace MSRV raised from `1.85` to `1.87`. `mzdata 0.63.x` uses
  `slice_as_chunks`, which was stabilised in Rust 1.87.0 (May 2025).
  The previous MSRV was chosen for `arrow-58.x`; 1.87 supersedes that.
- `STACK.md` regenerated for the new pin set.

### Removed

- `STRATEGY.md` (internal planning artifact; no longer tracked).

## [1.0.1] - 2026-05-21

Maintenance release. Pins the stack at `openproteo-core = 1.0.0`,
`opentfraw = 1.0.6`, `opentimstdf = 1.0.6`, `openwraw = 1.0.5`. Raises
the workspace MSRV to 1.85 (required by transitive `arrow` 58.x).

### Changed

- Workspace `rust-version` bumped from `1.75` to `1.85`. CI MSRV job
  updated to match.
- All sibling-repo deps now resolve from crates.io. CI no longer
  multi-checks-out the vendor repos; cargo pulls them as ordinary
  registry crates.
- Docusaurus site moved from `docs-site/` to `docs/`. The legacy
  `docs/CORPUS.md` and `docs/RELEASE.md` notes were folded into
  inline docstrings and removed from the tree (see git history for
  the originals).
- Root `README.md` rewritten as a proper project landing page.

## [1.0.0] - 2026-05-18

First stable release of the OpenProteo umbrella. Pins the stack at
`openproteo-core = 0.1.0`, `opentfraw = 1.0.5`, `opentimstdf = 1.0.5`,
`openwraw = 1.0.4`. The library crate `openproteo-io = 1.0.0` is
published to crates.io; the `vendor2mzml` CLI ships as a binary
release artifact only.

### Added

- Shared corpus schema and fetcher (STRATEGY P3 #9). New
  `docs/CORPUS.md` documents the per-repo `sources.json` /
  `manifest.json` shape. New `scripts/fetch_corpus.py` is a
  vendor-agnostic port of OpenTFRaw's fetcher, parameterized by
  `--sources`, `--corpus-dir`, `--manifest`, and `--ext-pattern`.
  OpenTFRaw now wraps it via a small shim; OpenTimsTDF and OpenWRaw
  carry stub `sources.json` files awaiting directory-bundle fetch
  support.
- README now leads with a stack callout naming the umbrella, the
  three vendor readers (`opentfraw`, `opentimstdf`, `openwraw`),
  shared core `openproteo-core`, and downstream consumer ProLance.
- ProLance truth-test gate. New `scripts/truthtest-prolance.sh` runs
  `cargo build --workspace` and `cargo test --features vendors` (and
  optionally `--with-corpus` for the full mzML -> Lance roundtrip) in
  the sibling ProLance checkout. `scripts/release-stack.sh` gains
  `--gate-prolance` and `--gate-with-corpus` flags that invoke the
  gate before tagging the umbrella; a non-zero exit aborts tag
  creation. Documented in `docs/RELEASE.md`.
- Typed `openproteo_io::Error` enum with `thiserror`-based variants
  (`UnsupportedFormat`, `FeatureDisabled`, `Io`, `Core`, feature-gated
  `Thermo`/`Bruker`/`Waters`, `Mzml`). Replaces `Box<dyn Error>` in
  `convert_to_mzml`, `convert_to_mzml_writer`, `collect`, and internal
  helpers. `openproteo-io-cli::mzml_reader` and `openproteo-io-py`'s
  internal helpers now use the same `openproteo_io::Result` alias.
- Coordinated stack release scheme. `scripts/release-stack.sh` reads
  pinned versions across the five-repo stack, aggregates per-repo
  `CHANGELOG.md` entries into combined release notes, and can create +
  push an annotated SemVer tag on the umbrella (dry-run by default;
  `--apply` gates all mutations).
- `STACK.md` pin-table snapshot of the current stack versions.
- `docs/RELEASE.md` documenting the umbrella SemVer tag scheme,
  per-repo bump rules, and the release procedure.
- `openproteo-io` 0.1.0 lib crate: vendor-feature gates (`thermo`,
  `bruker`, `waters`, `all`), `detect_format()` runtime probe, and
  `convert_to_mzml()` one-shot conversion that defers to the matching
  vendor crate.
- `openproteo-io-cli` 0.1.0 binary `vendor2mzml`: takes an input vendor
  path and an output mzML path, auto-detects the format, supports
  `--indexed`.
- End-to-end smoke test that converts Thermo / Bruker / Waters fixtures
  to mzML when present, skipping silently otherwise.
- `RENAME-TBD.md` flagging this as the working name pending the final
  decision (candidate: `OpenProteo`).

### Fixed

- Silenced unused-import warning for `openproteo_core::SpectrumSource`
  inside `openproteo_io::collect()` when no vendor features are
  enabled.

### Notes

- STRATEGY P0 #1 (route ProLance through `openproteo-io`) marked DONE.
  Shipped in ProLance `develop` commits `aece8f6` (single vendor
  ingester via `openproteo_io::collect`) and `708dbc3` (mzML writer
  delegates to `openproteo-core`).
- STRATEGY P3 #11 (ProLance integration tests as the stack truth test)
  marked DONE via the new `--gate-prolance` flag.
- STRATEGY P1 #3 rewritten and marked DONE: docs unification is
  rejected in favor of cross-linking. Each repo keeps independent
  docs to preserve legal isolation across the reverse-engineered
  parsers and to keep parser-specific docs unmuddied by umbrella
  scope. Stack callouts now live in all five stack repos plus the
  downstream ProLance consumer.
- STRATEGY P3 #9 (shared corpus + manifest) marked DONE: schema and
  fetcher shipped; actual corpus files remain per-repo and
  out-of-tree.
