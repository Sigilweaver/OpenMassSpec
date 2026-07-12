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
{"output":"sample.mzML","indexed":false,"spectra":3047,"elapsed_sec":0.812}
```

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

`--json` switches the output to a single JSON object suitable for
piping into `jq`.

## Exit codes

| Code | Meaning                                                          |
| ---- | ---------------------------------------------------------------- |
| 0    | Success.                                                         |
| 1    | I/O error, or `convert` failed (unsupported format / vendor).    |
| 2    | `validate`: input format was not recognised (clap errors also).  |
| 3    | `validate`: the conformance harness rejected the input.          |
