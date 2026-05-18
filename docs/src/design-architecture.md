# Architecture

```text
                  +------------------------+
                  |    openproteo-core     |
                  |  - SpectrumRecord      |
                  |  - SpectrumSource trait|
                  |  - mzML writer         |
                  |  - Arrow schema        |
                  |  - Conformance harness |
                  +-----------+------------+
                              ^
            +-----------------+-----------------+
            |                 |                 |
     +------+------+   +------+------+   +------+------+
     |  opentfraw  |   | opentimstdf |   |   openwraw  |
     |   (Thermo)  |   |  (Bruker)   |   |   (Waters)  |
     +------+------+   +------+------+   +------+------+
            |                 |                 |
            +--------+--------+--------+--------+
                     |        |        |
                     v        v        v
                  +-----------------------+
                  |    openproteo-io      |
                  |  - detect_format()    |
                  |  - convert_to_mzml()  |
                  |  - collect()          |
                  |  - VecSource          |
                  +-----------+-----------+
                              |
              +---------------+---------------+
              |                               |
     +--------+--------+             +--------+--------+
     | openproteo-io-cli |           | openproteo-io-py |
     |  (vendor2mzml)    |           |  (PyO3 bindings) |
     +-------------------+           +------------------+
```

## Layering rules

- **`openproteo-core` knows nothing about vendors.** It owns the
  shared schema, the mzML byte format, the Arrow layout, and the
  conformance harness. Anything generic enough to be shared between
  Thermo / Bruker / Waters lives here.
- **Vendor crates know nothing about each other.** Each implements
  `SpectrumSource` and a `write_mzml(path, writer)` helper. They
  depend on `openproteo-core` and zero other vendor crates.
- **`openproteo-io` is the only place that knows the full vendor
  set.** Format detection, dispatch, and feature-gated re-exports
  live here, behind the `thermo`, `bruker`, `waters` features.
- **`openproteo-io-cli` and `openproteo-io-py` depend on
  `openproteo-io`.** They never call vendor crates directly; if they
  need something from a vendor, that something is promoted to the
  umbrella first.

## Streaming model

A `SpectrumSource` exposes `iter_spectra(&mut self) -> Box<dyn
Iterator<Item = SpectrumRecord> + '_>`. Both the mzML writer and the
Arrow batch builder consume this iterator without ever holding more
than a single spectrum in memory at a time (except when the consumer
explicitly buffers, as `--validate` does).

This keeps OpenProteo usable for the multi-gigabyte timsTOF runs that
come out of dia-PASEF experiments without requiring temp files or
mmap tricks.
