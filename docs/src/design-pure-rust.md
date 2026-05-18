# Why pure Rust

Vendor mass-spectrometry SDKs - Thermo's `RawFileReader.dll`,
Bruker's `libtimsdata`, Waters' `DACServer` - have shaped MS tooling
for two decades, but they bring real costs:

1. **Platform lock-in.** Thermo ships a `.NET` assembly that runs only
   under Mono/Wine on non-Windows hosts; Waters' DAC layer is a
   Windows-only COM server; Bruker ships closed-source binaries with
   per-platform builds that lag.
2. **Auditability.** Anything emitted by a closed-source reader is a
   black box. For regulated environments (FDA-21 CFR Part 11 records,
   clinical pipelines), every binary in the data path is a compliance
   surface.
3. **Performance.** The vendor SDKs were not designed for streaming.
   Several materialize whole frames in memory; some force a process
   per file.
4. **License risk.** Each SDK ships under a vendor EULA that
   restricts redistribution, reverse engineering, and (in some cases)
   benchmarking publication.

OpenProteo's pure-Rust readers address each of these:

- **Single static binary.** `vendor2mzml` is one executable per
  platform with no external runtime dependencies (no .NET, no Wine,
  no SQLite client lib - SQLite is vendored statically by
  `rusqlite`).
- **Audit-friendly.** Every line of vendor parsing is in the open
  under Apache-2.0. `unsafe_code = "forbid"` is enforced workspace-
  wide.
- **Streaming first.** Every reader implements `SpectrumSource`,
  yielding spectra as an iterator.
- **Permissive license.** Apache-2.0 across the stack. No vendor
  redistribution requirements.

## Trade-offs

- **We lag vendor releases.** A new firmware that introduces new
  TDF columns requires a real change to `opentimstdf`, not a free
  pickup from a vendor update.
- **No proprietary acceleration.** Where Thermo's reader can call
  into native FFT / centroiding, OpenProteo does the work in safe
  Rust. For most pipelines the difference is invisible; for raw-data
  hot paths it can matter.
- **Less battle-tested on exotic files.** The vendor SDKs have been
  hit with every weird acquisition ever made. OpenProteo has a
  growing - but smaller - corpus. The conformance harness catches
  most regressions; please open an issue if you find a file we mis-
  parse.
