---
slug: /
title: Introduction
---

# OpenMassSpec

OpenMassSpec is a pure-Rust mass-spectrometry I/O stack. It reads native
vendor acquisitions from Thermo Fisher, Bruker, Waters, Agilent, and
SCIEX instruments and emits standards-compliant mzML, Arrow record
batches, or native Rust / Python data structures - without any vendor
SDK, runtime, or binary blob.

## What is in the box

| Component        | Purpose                                                  |
| ---------------- | -------------------------------------------------------- |
| `openmassspec-core`  | Shared schema, mzML writer, conformance harness, Arrow.  |
| `openmassspec-io`    | Umbrella crate: auto-detects vendor format and dispatches. |
| `openmassspec-io-cli`| `vendor2mzml` binary: one-shot conversion + introspection. |
| `openmassspec-io-py` | PyO3 bindings (`openmassspec-io` on PyPI).                |
| `opentfraw`        | Thermo Finnigan `.raw` reader (Rust 2021, MSRV 1.75).    |
| `opentimstdf`      | Bruker `.d/` (TDF) reader.                              |
| `openwraw`         | Waters MassLynx `.raw/` reader.                          |
| `openaraw`         | Agilent MassHunter `.d/` reader.                         |
| `opensxraw`        | SCIEX legacy `.wiff`/`.wiff.scan` reader.                |

## Design goals

1. **Pure Rust, no vendor SDK.** No Thermo .NET assemblies, no Bruker
   shared library, no MassLynx COM server. The reader stack is fully
   forbidden from `unsafe_code`.
2. **mzML byte-stability.** The same input on the same OpenMassSpec
   release always produces byte-identical mzML. Conformance tests pin
   this against the PSI-MS controlled vocabulary.
3. **One schema across vendors.** Every reader yields the same
   `SpectrumRecord` shape; Arrow batches share a single schema across
   Thermo / Bruker / Waters / Agilent / SCIEX.
4. **Streaming where possible.** Spectra are produced as an iterator;
   the mzML writer never buffers the full run.

## Where to start

- New to the project? Read the [Quickstart: vendor2mzml](./quickstart-cli.md).
- Embedding in a Rust pipeline? See [Quickstart: Rust](./quickstart-rust.md).
- Working in Python / pandas / Arrow? See [Quickstart: Python](./quickstart-python.md).
- Want the design rationale? See [Architecture](./design-architecture.md).
