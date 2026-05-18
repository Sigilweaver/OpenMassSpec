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

## Validate before writing

```sh
vendor2mzml convert sample.raw sample.mzML --validate
```

This collects every spectrum and runs the conformance harness from
`openproteo-core` (monotonic indices, non-negative retention times,
sane peak counts, peak arrays in agreement, polarity / MS-level
consistency). On any violation, no mzML is written and the process
exits with status 3.

The collected records are reused for the actual write, so `--validate`
costs one decode pass, not two.

## Profile a run

```sh
vendor2mzml convert sample.raw sample.mzML --profile json
```

emits one JSON object on stderr after the conversion completes:

```json
{"output":"sample.mzML","indexed":false,"validated":false,"spectra":3047,"elapsed_sec":0.812}
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

| Code | Meaning                                                       |
| ---- | ------------------------------------------------------------- |
| 0    | Success.                                                      |
| 1    | I/O error, unsupported format, or vendor crate failure.       |
| 2    | Argument parsing error (clap).                                |
| 3    | `--validate` was passed and the conformance harness rejected. |
