# Format detection

`openmassspec_io::detect_format(path)` inspects the filesystem entry at
`path` and returns a `Detected { path, format }` if and only if the
signature of a supported vendor matches. The detection rules are:

| Vendor  | Path kind | Signature                                                        |
| ------- | --------- | ----------------------------------------------------------------- |
| Bruker  | Directory | Contains `analysis.tdf` **and** `analysis.tdf_bin`.               |
| Agilent | Directory | Contains `AcqData/MSScan.bin`.                                    |
| Waters  | Directory | Contains `_HEADER.TXT`.                                           |
| Thermo  | File      | First 18 bytes match the Finnigan header (see below).             |
| SCIEX   | File      | `.wiff` extension (case-insensitive) with a sibling `<name>.wiff.scan` file. |

For directories, the checks run in the order above (Bruker, then
Agilent, then Waters) and stop at the first match. Bruker and Agilent
bundles are both commonly named `<run>.d/`, so they are disambiguated
by contents, not by the directory name - detection never inspects the
extension for directory formats.

For files, the checks run Thermo then SCIEX, and stop at the first
match.

**Thermo detection is content-based, not extension-based.** A `.raw`
suffix is not sufficient (and not required): `detect_format` opens the
file and checks whether bytes 2 through 17 equal the UTF-16LE string
`Finnigan`, which is the Thermo Finnigan file signature. This is the
one vendor whose detection reads file content; every other signature
is a directory-entry or extension check, which keeps detection cheap
even for stat-heavy filesystems.

## Edge cases

- A directory that does **not** match any of the Bruker / Agilent /
  Waters content checks is returned as `None`, regardless of its name
  or extension. We do not heuristically descend looking for alternate
  bundle layouts.
- A symlink to a Thermo `.raw` file is treated as a regular file; the
  signature check reads through the link to the target's content.
- A `.wiff` file with no sibling `.wiff.scan` file is **not** detected
  as SCIEX - `detect_format` returns `None` even though the extension
  matches, because the reader needs the paired scan file.
- Casing: the SCIEX `.wiff` extension match is case-insensitive, but
  directory-bundle entry names are checked exactly as the vendor
  writes them (`analysis.tdf`, not `Analysis.TDF`; `AcqData`, not
  `acqdata`).

## CLI behavior

```sh
vendor2mzml info /not/a/vendor/file.txt
```

exits with status 1 and prints `error: ... does not look like a
supported vendor format`. There is no fallback to peeking file
contents - if detection fails, the call fails.
