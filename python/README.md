# openproteo

`openproteo` is a thin Python metapackage that bundles the OpenProteo vendor reader stack:

| Vendor | Format         | Underlying package |
|--------|----------------|--------------------|
| Thermo | `.raw` file    | `opentfraw`        |
| Bruker | `.d/` bundle   | `opentimstdf`      |
| Waters | `.raw/` dir    | `openwraw`         |

## Install

Install just what you need:

```bash
pip install openproteo[thermo]
pip install openproteo[bruker]
pip install openproteo[waters]
```

Or install every supported vendor reader:

```bash
pip install openproteo[all]
```

## Usage

```python
import openproteo

kind = openproteo.detect("/data/sample.raw")     # "thermo" | "bruker" | "waters" | None
run  = openproteo.open_run("/data/sample.raw")    # vendor-specific reader object
```

`open_run` raises `ImportError` if the matching vendor extra is not installed and
`ValueError` if the format cannot be detected.

## License

Apache-2.0. See [`LICENSE`](../LICENSE).
