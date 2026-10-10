# openmassspec

`openmassspec` is a thin Python metapackage that bundles the OpenMassSpec vendor reader stack:

| Vendor | Format         | Underlying package |
|--------|----------------|--------------------|
| Thermo | `.raw` file    | `opentfraw`        |
| Bruker | `.d/` bundle   | `opentimstdf`      |
| Waters | `.raw/` dir    | `openwraw`         |

## Install

Install just what you need:

```bash
pip install openmassspec[thermo]
pip install openmassspec[bruker]
pip install openmassspec[waters]
```

Or install every supported vendor reader:

```bash
pip install openmassspec[all]
```

## Usage

```python
import openmassspec

kind = openmassspec.detect("/data/sample.raw")     # "thermo" | "bruker" | "waters" | None
run  = openmassspec.open_run("/data/sample.raw")    # vendor-specific reader object
```

`open_run` raises `ImportError` if the matching vendor extra is not installed and
`ValueError` if the format cannot be detected.

## License

Apache-2.0. See [`LICENSE`](https://github.com/Sigilweaver/OpenMassSpec/blob/main/LICENSE).
