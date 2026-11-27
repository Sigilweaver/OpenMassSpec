# Quickstart: `vendor2mzml`

`vendor2mzml` auto-detects the vendor format of its input and writes
mzML (or a gzipped mzML, or a metadata summary).

## Convert

```sh
# Thermo .raw
vendor2mzml convert sample.raw sample.mzML

# Bruker .d bundle directory
vendor2mzml convert sample.d sample.mzML --indexed

# Waters .raw bundle directory, gzipped output
vendor2mzml convert sample.raw sample.mzML.gz
```

`--indexed` writes the standards-compliant `<indexList>` plus SHA-1
hash, so downstream tools (mzML2HDF, ProteoWizard's `msconvert`,
`MzIdentML` builders) can index-jump into the file.

## Disk space

`<spectrumList count>` must hold the number of spectra actually written,
which is known only after the last one. The writer therefore holds the
encoded spectra until the run is read to the end: in memory up to 64 MiB,
then in a temporary file in the output file's directory. The file is
removed when the conversion ends, successful or not. While a large
conversion runs, that directory needs room for the output plus the
uncompressed spectra (for `.mzML.gz` output the spool is not gzipped).
`--spool-dir DIR` puts the file elsewhere, for example when the output
goes to a small or network disk.

## Unreadable spectra

A spectrum or chromatogram the vendor reader cannot decode is left out
of the output by default, and so is one the writer's checks refuse
(mismatched arrays, NaN or infinite values, a duplicate native id, a
collision energy with no unit...). `convert` prints the count and the
cause of the first few on stderr and exits 4, because the file was
written but is not the whole run:

```text
warning: skipped 12 record(s) (12 spectra, 0 chromatograms; 0 spectra and 0 chromatograms were readable but failed the writer's checks)
  Spectrum #4012 (controllerType=0 controllerNumber=1 scan=4013): scan data record ending at offset 0x5a7dcb does not match a known layout
```

Exit 4 also means the reader announced more spectra than were written or
skipped. Exit 0 means nothing was left out.

`--strict` stops at the first unreadable record, prints its error, and
exits 1. The partial output file is kept, and stderr says it is
incomplete:

```text
error: thermo reader: scan data record ending at offset 0x5a7dcb does not match a known layout
  sample.mzML is incomplete
```

```sh
vendor2mzml convert --strict sample.raw sample.mzML
```

`info` and `validate` always stop at the first unreadable spectrum and
exit 1.

## Warnings and `--allow`

Some values a file does not state. Core 4 never guesses them: a record
without one is refused unless the caller allows the check, and then the
record is written without the value and carries a warning. `--allow CHECK`
(repeatable) allows one:

| Check | What it lets through |
| ----- | -------------------- |
| `unknown_scan_mode` | A spectrum with no profile or centroid flag is written with neither term; `--centroid` leaves it unchanged. |
| `unknown_collision_energy_unit` | A collision energy with no unit is written as a unit-less `userParam`. |
| `dangling_precursor_ref` | A precursor that names a spectrum not in the output loses the reference. |
| `spectrum_count_shortfall` | A reader that announced more spectra than it yielded ends with a warning and exit 4 instead of an error. |

A format that never states the scan mode or the collision energy unit in
any file says so itself (the reader declares it), and its files convert
without `--allow`: SCIEX `.wiff` (scan mode) and Agilent Q-TOF
`.d` (collision energy unit). Every affected spectrum carries a warning
`userParam` (`openmassspec: warning unknown_scan_mode`), the run carries
one `openmassspec: warning unstated_by_source`, and `convert` prints one
stderr line per warning code with its count. Warnings do not change the
exit status. `vendor2mzml info` lists the checks a format declares as
`never stated`.

An empty spectrum (no peaks) carries `openmassspec: warning empty_spectrum`
and no base peak or m/z range. Warnings that readers define themselves are
namespaced (`opentfraw.charge_without_selected_mz`).

## Centroid on the way out

```sh
vendor2mzml convert sample.raw sample.mzML --centroid
vendor2mzml convert sample.raw sample.mzML --centroid --centroid-min-intensity 500
```

`--centroid` centroids every profile-mode spectrum before writing
(local-maxima peak picking; spectra already tagged centroid pass
through unchanged). It is off by default - a plain `convert` never
discards profile data. `--centroid-min-intensity` discards picked peaks
below that height and is ignored unless `--centroid` is set. `info`
takes the same two flags, for a peak-count summary post-centroiding
without writing mzML.

## Validate

```sh
vendor2mzml validate sample.raw
vendor2mzml validate sample.mzML
vendor2mzml validate --json sample.d
```

`validate` accepts any input `convert` accepts (Thermo `.raw`, Bruker
`.d`, Waters `.raw`) plus pre-existing mzML files (`.mzML`, `.mzML.gz`,
read via `mzdata`). It runs the conformance harness from
`openmassspec-core`:

- monotonic spectrum indices,
- non-negative, non-decreasing retention times,
- equal-length m/z and intensity arrays,
- equal-length mobility arrays (if present),
- MS-level / polarity sanity,
- precursor presence on MSn spectra.

`--json` emits a single JSON object suitable for `jq`:

```json
{"ok":true,"input":"sample.mzML","kind":"mzML","spectrum_count":3047,"elapsed_sec":0.214}
```

On failure, `ok` is `false` and `error_kind` names the conformance
variant (for example, `PeakArrayLengthMismatch`).

## Profile a run

```sh
vendor2mzml convert sample.raw sample.mzML --profile json
```

emits one JSON object on stderr after the conversion completes:

```json
{"output":"sample.mzML","indexed":false,"spectra":3047,"spectra_skipped":0,"chromatograms":2,"chromatograms_skipped":0,"elapsed_sec":0.812}
```

`spectra` and `chromatograms` count what was written; the `_skipped`
fields count unreadable records left out.

Use `--profile text` for a human-readable variant.

## Inspect without converting

```sh
vendor2mzml info sample.d
```

```text
vendor:        bruker
path:          sample.d
instrument:    timsTOF Pro (MS:1003005)
source format: Bruker TDF format (MS:1002817)
native id:     Bruker TDF nativeID format (MS:1002818)
software:      opentimstdf 1.0.4
spectra:       24425
  ms1:         1132
  ms2:         23293
  pos:         24425
rt range:      240.291 - 900.132 s (4.00 - 15.00 min)
decode time:   2.05 s
```

A spectrum with no retention time is not counted in `rt range`; `info`
prints `no rt: N` when there are any, and omits the range when there are
none. `info` also prints `never stated:` (the checks the format declares it
cannot state) and one `warning:` line per warning code the reader set.

`--json` switches the output to a single JSON object suitable for
piping into `jq`.

## Exit codes

| Code | Meaning                                                          |
| ---- | ---------------------------------------------------------------- |
| 0    | Success. For `convert`, nothing was left out; warnings on records do not change it. |
| 1    | I/O error, unsupported format, an unreadable spectrum in `info`/`validate`, or `convert --strict` hit one. |
| 2    | `validate`: input format was not recognised (clap errors also).  |
| 3    | `validate`: the conformance harness rejected the input.          |
| 4    | `convert`: the output was written, but records were skipped or the reader's spectrum count could not be reconciled. |
