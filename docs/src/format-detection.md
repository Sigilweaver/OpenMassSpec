# Format detection

`openproteo_io::detect_format(path)` inspects the filesystem entry at
`path` and returns a `Detected { path, format }` if and only if the
signature of a supported vendor matches. The detection rules are:

| Vendor | Path kind | Signature                                                |
| ------ | --------- | -------------------------------------------------------- |
| Thermo | File      | Path ends in `.raw` (case-insensitive) **and** is a file. |
| Bruker | Directory | Path ends in `.d/`, contains `analysis.tdf` and `analysis.tdf_bin`. |
| Waters | Directory | Path ends in `.raw/`, contains `_HEADER.TXT`.            |

The Thermo and Waters check share the `.raw` suffix but differ by
path kind: Thermo is a regular file, Waters is a bundle directory.
Detection does **not** open the file or parse content - only the
filename and (for directory formats) the presence of one or two
required entries. This keeps detection cheap even for stat-heavy
filesystems.

## Edge cases

- A `.raw` directory that does **not** contain `_HEADER.TXT` is
  returned as `None`. We do not heuristically descend looking for
  alternate Waters layouts.
- A symlink to a `.raw` file is treated as a regular file; we do not
  resolve through to its target before checking the suffix.
- Casing: the suffix match is case-insensitive but the directory and
  file names inside a Bruker / Waters bundle are checked exactly as
  the vendor writes them (`analysis.tdf`, not `Analysis.TDF`).

## CLI behavior

```sh
vendor2mzml info /not/a/vendor/file.txt
```

exits with status 1 and prints `error: ... does not look like a
supported vendor format`. There is no fallback to peeking file
contents - if detection fails, the call fails.
