# OpenProteo

OpenProteo is a pure-Rust mass-spectrometry I/O stack. It reads native
vendor acquisitions from Thermo Fisher, Bruker, and Waters instruments
and emits standards-compliant mzML, Arrow record batches, or native
Rust / Python data structures - without any vendor SDK, runtime, or
binary blob.

## What is in the box

| Component        | Purpose                                                  |
| ---------------- | -------------------------------------------------------- |
| `openproteo-core`  | Shared schema, mzML writer, conformance harness, Arrow.  |
| `openproteo-io`    | Umbrella crate: auto-detects vendor format and dispatches. |
| `openproteo-io-cli`| `vendor2mzml` binary: one-shot conversion + introspection. |
| `openproteo-io-py` | PyO3 bindings (`openproteo-io` on PyPI).                |
| `opentfraw`        | Thermo Finnigan `.raw` reader (Rust 2021, MSRV 1.75).    |
| `opentimstdf`      | Bruker `.d/` (TDF) reader.                              |
| `openwraw`         | Waters MassLynx `.raw/` reader.                          |

## Design goals

1. **Pure Rust, no vendor SDK.** No Thermo .NET assemblies, no Bruker
   shared library, no MassLynx COM server. The reader stack is fully
   forbidden from `unsafe_code`.
2. **mzML byte-stability.** The same input on the same OpenProteo
   release always produces byte-identical mzML. Conformance tests pin
   this against the PSI-MS controlled vocabulary.
3. **One schema across vendors.** Every reader yields the same
   `SpectrumRecord` shape; Arrow batches share a single schema across
   Thermo / Bruker / Waters.
4. **Streaming where possible.** Spectra are produced as an iterator;
   the mzML writer never buffers the full run.

## Where to start

- New to the project? Read the [Quickstart: vendor2mzml](./quickstart-cli.md).
- Embedding in a Rust pipeline? See [Quickstart: Rust](./quickstart-rust.md).
- Working in Python / pandas / Arrow? See [Quickstart: Python](./quickstart-python.md).
- Want the design rationale? See [Architecture](./design-architecture.md).
