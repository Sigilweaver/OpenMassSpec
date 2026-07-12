# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [1.2.0] - 2026-07-12

### Added

- Optional centroiding across the whole stack, built on
  `openmassspec-core` 1.1.0's new `Centroided` adapter (local-maxima peak
  picking; already-centroid spectra pass through unchanged). Off by
  default everywhere - profile data is never silently discarded.
  - Rust: `collect_centroided`, `convert_to_mzml_centroided`, and
    `convert_to_mzml_writer_centroided` in `openmassspec-io`, each taking
    a `min_intensity: Option<f32>` noise floor alongside the existing
    `collect` / `convert_to_mzml` / `convert_to_mzml_writer`.
  - CLI: `vendor2mzml convert --centroid [--centroid-min-intensity <f32>]`
    and `vendor2mzml info --centroid [--centroid-min-intensity <f32>]`.
  - Python: `centroid` / `centroid_min_intensity` keyword arguments on
    `to_mzml`, `run_info`, `iter_spectra`, and `read_arrow`.

## [1.1.0] - 2026-07-11

### Added

- Agilent and SCIEX support, so the umbrella now covers all five vendor
  readers in the stack through one API:
  - New `agilent` feature (pulls in [`openaraw`](https://github.com/Sigilweaver/OpenARaw))
    and `sciex` feature (pulls in [`opensxraw`](https://github.com/Sigilweaver/OpenSXRaw)).
    Both are included in the `all` meta-feature.
  - `detect_format` now recognizes Agilent MassHunter `.d/` bundles (by
    `AcqData/MSScan.bin`) and SCIEX legacy `.wiff` files (by a `.wiff`
    extension with a sibling `.wiff.scan`).
  - `VendorFormat::AgilentMassHunter` and `VendorFormat::SciexWiff`
    variants; `convert_to_mzml` / `convert_to_mzml_writer` / `collect`
    dispatch to them.
  - `vendor2mzml` CLI handles both automatically (built with `all`).
  - Python: `.wiff` and Agilent `.d/` read through the base
    `openmassspec` install (the binding compiles in every vendor). New
    `openmassspec[agilent]` extra installs the standalone `openaraw`
    package; SCIEX has no standalone Python package yet, so there is no
    `sciex` extra (reading still works from the base install).

## [1.0.1] - 2026-07-11

### Fixed

- `opentfraw`/`opentimstdf`/`openwraw` workspace dependency requirements
  were left at their pre-rename minimums (`1.2.0`/`1.0.6`/`1.0.5`), which
  a fresh consumer could satisfy with an old vendor-crate version still
  depending on `openproteo-core` - a different (incompatible) trait from
  `openmassspec-core`, breaking the build. Bumped to `1.3.1`/`1.2.3`/`1.2.2`,
  the first versions of each that depend on `openmassspec-core`.

## [1.0.0] - 2026-07-10

Renamed from OpenProteo / `openproteo-io` / `openproteo`. The vendor
raw-file readers this stack wraps (Thermo, Bruker, Waters, with Agilent
and SCIEX joining the suite) are used as much in metabolomics and
lipidomics as in proteomics, so the umbrella naming moves from
proteomics-specific to general mass spectrometry. `openmassspec-core`
(the shared foundation crate) made the same move; see its own
CHANGELOG for that side of the rename.

No API or behavioral changes from `openproteo-io`/`openproteo` 1.3.0.
Version reset to 1.0.0 to reflect that these are new package identities
on crates.io and PyPI (the old `openproteo-io`/`openproteo` names stay
published and frozen at 1.3.0, they are not superseded in place). See
[OpenProteo's CHANGELOG](https://github.com/Sigilweaver/OpenProteo/blob/main/CHANGELOG.md)
for pre-rename history.

### Changed

- Repo renamed OpenProteo -> OpenMassSpec.
- Crates renamed `openproteo-io` -> `openmassspec-io`,
  `openproteo-io-cli` -> `openmassspec-io-cli`,
  `openproteo-io-py` -> `openmassspec-io-py`.
- PyPI packages renamed `openproteo-io` -> `openmassspec-io` (bindings),
  `openproteo` -> `openmassspec` (facade).
- Dependency on the shared core crate updated to `openmassspec-core` 1.0.0.

## [1.3.0] - 2026-07-06

### Added

- `openproteo-io` (Python): `read_polars()`, a thin wrapper over the
  existing zero-copy `read_arrow()` API returning a Polars `DataFrame`
  directly. Gated behind a new optional `polars` extra
  (`pip install openproteo-io[polars]`).

### Fixed

- `CITATION.cff`: corrected the abstract, which claimed the Python
  bindings "integrate with Polars, PyArrow, and Pandas" as a blanket
  statement; now names the actual `read_arrow()`/`read_polars()` APIs
  directly.

## [1.2.1] - 2026-07-04

### Changed

- `publish.yml`: crates.io publish step now uses `continue-on-error: true`
  so re-triggered tag runs (e.g. after a pyproject.toml fix) do not fail
  the whole workflow when the crate version was already published.
- Bundle `LICENSE` into both package sdists: `openproteo-io` via a maturin
  `include`, `openproteo` via setuptools `license-files`. Enables
  source-based installs and conda-forge packaging (which build from
  source). Added `RELEASING.md`, the release SOP.

## [1.2.0] - 2026-07-02

### Added

- `Spectrum.scan_mode` (Python): `"centroid"`, `"profile"`, or `None`.
  Populated by all three vendor adapters; previously decoded in
  `SpectrumRecord` but not surfaced to Python.
- `Spectrum.analyzer` (Python): mass analyzer family as a lowercase
  string (`"itms"`, `"tqms"`, `"sqms"`, `"tofms"`, `"ftms"`,
  `"sector"`) or `None`. Populated by all three vendor adapters.
- `Spectrum.filter` (Python): Thermo-style scan filter string, or
  `None` for Bruker and Waters files.
- `Spectrum.ion_injection_time_ms` (Python): ion injection /
  accumulation time in milliseconds, or `None`.
- `Spectrum.low_mz` / `Spectrum.high_mz` (Python): observed m/z
  range endpoints, or `None`.
- `RunInfo` class and `run_info(path)` function (Python): returns
  run-level metadata (instrument name and CV accession, source file
  name, acquisition timestamp, parser software name and version)
  without iterating spectra.

### Changed

- `opentfraw` dependency bumped from `1.0.6` to `1.2.0`. Picks up
  the Exploris scan-event calibration fix (profile m/z and MS2
  precursor now decoded correctly on Exploris instruments) and the
  new per-peak FT label, scan-parameters, profile, and
  created-timestamp APIs contributed by @oskarsari.

## [1.1.0] - 2026-05-31

### Added

- `CITATION.cff`: author identity (Nathan Riley + ORCID) and a
  scaffolded `identifiers:` block ready for the Zenodo concept DOI.

### Changed

- Workspace MSRV raised from `1.87` to `1.88` (mzdata uses
  `slice_as_chunks`).
- CI: maturin-develop step now provisions a venv before invoking
  maturin, so wheel builds succeed on Ubuntu runners with PEP 668.
- Cloudflare Pages deploy moved off the `wrangler` GitHub Action;
  the Cloudflare GitHub App now handles deploys.
- Docusaurus navbar adds a Core link to docs.rs/openproteo-core
  (WP15).
- Workspace metadata hygiene (WP13): authors, repository, homepage,
  documentation, readme, keywords, categories declared once under
  `[workspace.package]` and inherited.
- README badge block unified across the Sigilweaver portfolio.

## [1.0.3] - 2026-05-22

CI / workflow correctness release. No runtime behaviour changes.

### Changed

- Workspace MSRV raised from `1.85` to `1.87` to correctly declare the
  minimum Rust version required by `mzdata 0.63.x` (`slice_as_chunks`
  was stabilised in Rust 1.87.0). The `1.85` declaration shipped in
  `1.0.2` was incorrect.
- `release.yml`: removed stale multi-repo checkout steps that were
  left over from the path-dependency era. All vendor-crate deps resolve
  from crates.io so only the umbrella repo needs to be checked out.
- `wheels.yml` deleted; the wheel build and PyPI publish are now
  handled entirely by `publish.yml`.
- CI `MSRV check` updated from `1.85` to `1.87` to match.

## [1.0.2] - 2026-05-22

Test + packaging release. No runtime behaviour changes; the umbrella
crates, the Python metapackage, and the `openproteo-io` Python wheel
are now publishable from a single tag, and the `openproteo`
metapackage drift bug is fixed.

### Added

- PyPI publish wiring in `.github/workflows/publish.yml`:
  - maturin wheel matrix (linux x86_64/aarch64, macos x86_64/aarch64,
    windows x86_64) + sdist for `openproteo-io`,
  - pure-Python sdist + wheel build for the `openproteo` metapackage,
  - PyPI uploads gated on the `pypi` GitHub environment via OIDC
    trusted publishing (no API tokens in the repo).
- Expanded `python/tests/test_metapackage.py`: dispatch tests for
  `open_run()` (thermo / bruker / waters with monkeypatched vendor
  modules), `__version__` vs installed-metadata drift check,
  immutable-`VENDORS` and full `__all__` surface check.

### Changed

- `openproteo.__version__` now derives from
  `importlib.metadata.version("openproteo")` rather than a hard-coded
  literal. This fixes the silent `0.2.0` vs `1.0.x` drift the previous
  literal could produce.
- Workspace pin: `openproteo-core = 1.0.1` (docs-only patch upstream).
- Workspace MSRV raised from `1.85` to `1.87`. `mzdata 0.63.x` uses
  `slice_as_chunks`, which was stabilised in Rust 1.87.0 (May 2025).
  The previous MSRV was chosen for `arrow-58.x`; 1.87 supersedes that.
- `STACK.md` regenerated for the new pin set.

### Removed

- `STRATEGY.md` (internal planning artifact; no longer tracked).

## [1.0.1] - 2026-05-21

Maintenance release. Pins the stack at `openproteo-core = 1.0.0`,
`opentfraw = 1.0.6`, `opentimstdf = 1.0.6`, `openwraw = 1.0.5`. Raises
the workspace MSRV to 1.85 (required by transitive `arrow` 58.x).

### Changed

- Workspace `rust-version` bumped from `1.75` to `1.85`. CI MSRV job
  updated to match.
- All sibling-repo deps now resolve from crates.io. CI no longer
  multi-checks-out the vendor repos; cargo pulls them as ordinary
  registry crates.
- Docusaurus site moved from `docs-site/` to `docs/`. The legacy
  `docs/CORPUS.md` and `docs/RELEASE.md` notes were folded into
  inline docstrings and removed from the tree (see git history for
  the originals).
- Root `README.md` rewritten as a proper project landing page.

## [1.0.0] - 2026-05-18

First stable release of the OpenProteo umbrella. Pins the stack at
`openproteo-core = 0.1.0`, `opentfraw = 1.0.5`, `opentimstdf = 1.0.5`,
`openwraw = 1.0.4`. The library crate `openproteo-io = 1.0.0` is
published to crates.io; the `vendor2mzml` CLI ships as a binary
release artifact only.

### Added

- Shared corpus schema and fetcher (STRATEGY P3 #9). New
  `docs/CORPUS.md` documents the per-repo `sources.json` /
  `manifest.json` shape. New `scripts/fetch_corpus.py` is a
  vendor-agnostic port of OpenTFRaw's fetcher, parameterized by
  `--sources`, `--corpus-dir`, `--manifest`, and `--ext-pattern`.
  OpenTFRaw now wraps it via a small shim; OpenTimsTDF and OpenWRaw
  carry stub `sources.json` files awaiting directory-bundle fetch
  support.
- README now leads with a stack callout naming the umbrella, the
  three vendor readers (`opentfraw`, `opentimstdf`, `openwraw`),
  shared core `openproteo-core`, and downstream consumer ProLance.
- ProLance truth-test gate. New `scripts/truthtest-prolance.sh` runs
  `cargo build --workspace` and `cargo test --features vendors` (and
  optionally `--with-corpus` for the full mzML -> Lance roundtrip) in
  the sibling ProLance checkout. `scripts/release-stack.sh` gains
  `--gate-prolance` and `--gate-with-corpus` flags that invoke the
  gate before tagging the umbrella; a non-zero exit aborts tag
  creation. Documented in `docs/RELEASE.md`.
- Typed `openproteo_io::Error` enum with `thiserror`-based variants
  (`UnsupportedFormat`, `FeatureDisabled`, `Io`, `Core`, feature-gated
  `Thermo`/`Bruker`/`Waters`, `Mzml`). Replaces `Box<dyn Error>` in
  `convert_to_mzml`, `convert_to_mzml_writer`, `collect`, and internal
  helpers. `openproteo-io-cli::mzml_reader` and `openproteo-io-py`'s
  internal helpers now use the same `openproteo_io::Result` alias.
- Coordinated stack release scheme. `scripts/release-stack.sh` reads
  pinned versions across the five-repo stack, aggregates per-repo
  `CHANGELOG.md` entries into combined release notes, and can create +
  push an annotated SemVer tag on the umbrella (dry-run by default;
  `--apply` gates all mutations).
- `STACK.md` pin-table snapshot of the current stack versions.
- `docs/RELEASE.md` documenting the umbrella SemVer tag scheme,
  per-repo bump rules, and the release procedure.
- `openproteo-io` 0.1.0 lib crate: vendor-feature gates (`thermo`,
  `bruker`, `waters`, `all`), `detect_format()` runtime probe, and
  `convert_to_mzml()` one-shot conversion that defers to the matching
  vendor crate.
- `openproteo-io-cli` 0.1.0 binary `vendor2mzml`: takes an input vendor
  path and an output mzML path, auto-detects the format, supports
  `--indexed`.
- End-to-end smoke test that converts Thermo / Bruker / Waters fixtures
  to mzML when present, skipping silently otherwise.
- `RENAME-TBD.md` flagging this as the working name pending the final
  decision (candidate: `OpenProteo`).

### Fixed

- Silenced unused-import warning for `openproteo_core::SpectrumSource`
  inside `openproteo_io::collect()` when no vendor features are
  enabled.

### Notes

- STRATEGY P0 #1 (route ProLance through `openproteo-io`) marked DONE.
  Shipped in ProLance `develop` commits `aece8f6` (single vendor
  ingester via `openproteo_io::collect`) and `708dbc3` (mzML writer
  delegates to `openproteo-core`).
- STRATEGY P3 #11 (ProLance integration tests as the stack truth test)
  marked DONE via the new `--gate-prolance` flag.
- STRATEGY P1 #3 rewritten and marked DONE: docs unification is
  rejected in favor of cross-linking. Each repo keeps independent
  docs to preserve legal isolation across the reverse-engineered
  parsers and to keep parser-specific docs unmuddied by umbrella
  scope. Stack callouts now live in all five stack repos plus the
  downstream ProLance consumer.
- STRATEGY P3 #9 (shared corpus + manifest) marked DONE: schema and
  fetcher shipped; actual corpus files remain per-repo and
  out-of-tree.
