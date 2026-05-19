# Mass-Spec Stack Strategy

Status: draft for review
Date: 2026-05-18
Scope: OpenProteoCore, OpenWRaw, OpenTFRaw, OpenTimsTDF (OpenTDF), OpenProteo, ProLance
Audience: maintainer (single-owner project, treat the five repos as one unit)

---

## 1. Where we are

### 1.1 The stack as it exists today

```
                      OpenProteoCore (shared trait + records + mzML writer)
                                ^
              +-----------------+-----------------+
              |                 |                 |
          OpenWRaw          OpenTFRaw         OpenTimsTDF
          (Waters .raw)     (Thermo .raw)     (Bruker .d)
              |                 |                 |
              +-----------------+-----------------+
                                v
                           OpenProteo
                  (detect_format, vendor2mzml CLI,
                   openproteo-io-py wheel, mdBook)
                                |
                                v
                            ProLance
              (Lance/Arrow store + CLI + py - in progress)
```

### 1.2 Per-repo snapshot

| Repo            | Version | Lang surface       | Status                  | Docs site   | CI / Release                  |
| --------------- | ------- | ------------------ | ----------------------- | ----------- | ----------------------------- |
| OpenProteoCore  | 0.1.0   | Rust crate         | Foundation, in use      | README only | CI shipped, no release wf     |
| OpenWRaw        | 1.0.3   | Rust + Py wheel    | Stable, narrow vendor   | Docusaurus  | CI + OIDC publish, tags pushed|
| OpenTFRaw       | 1.0.4   | Rust + Py wheel    | Stable, wide vendor     | Docusaurus  | CI + OIDC publish, tags pushed|
| OpenTimsTDF     | 1.0.4   | Rust + Py wheel    | Stable                  | Docusaurus  | CI + OIDC publish, tags pushed|
| OpenProteo      | 0.1.0   | Rust + CLI + Py    | Experimental, just bootstrapped | mdBook | CI + release + wheels (local) |
| ProLance        | 0.1.0   | Rust + CLI + py-stub| Experimental           | none        | CI on develop, no release wf  |

All repos: Apache-2.0, Rust 2021, MSRV 1.75, `unsafe_code = "forbid"`, owner = `Sigilweaver`.

### 1.3 What is actually working end-to-end

- `vendor2mzml` converts all three vendors to PSI-MS mzML 1.1.0 (validated on PXD068962
  Thermo, PXD036417 Bruker, PXD058812 Waters).
- `openproteo-io-py` exposes streaming spectra to NumPy (PyArrow optional) for all three
  vendors.
- Conformance harness validates peak-array parity / TIC / base-peak / MS2 precursor /
  RT monotonicity across 27k+ spectra.
- ProLance ingests Thermo via `opentfraw` and ingests pre-existing mzML.

### 1.4 What is not working / inconsistent

1. **ProLance bypasses OpenProteo.** Three of ProLance's four vendor adapters
   (`thermo.rs`, `bruker.rs`, `waters.rs`) talk directly to the vendor crates and
   re-implement their own mzML data model
   ([prolance-ms/src/mzml.rs](../ProLance/crates/prolance-ms/src/mzml.rs)). `waters.rs`
   is an explicit stub (returns `Unsupported`). The umbrella crate that exists for
   exactly this purpose (`openproteo-io`) is not depended on.
2. **Two docs toolchains.** OpenWRaw/OpenTFRaw/OpenTimsTDF all use Docusaurus + bun.
   OpenProteo (and the planned ProLance docs) use mdBook. Stated preference: Docusaurus.
3. **OpenProteoCore is invisible.** It is the contract every parser implements, but it
   has no docs, no README beyond a stub, no CHANGELOG, and is gitignored from the user's
   ROADMAP. Downstream contributors cannot find the canonical type definitions.
4. **Error type fragmentation.** Vendor crates use a mix of hand-rolled enums and
   `thiserror`. ProLance re-wraps everything into `MsError`.
5. **Three independent Python wheels.** `openwraw`, `opentfraw`, `opentimstdf`,
   `openproteo-io`. A Python user wanting "the stack" installs four packages and four
   top-level imports.
6. **Five separate release trains.** No coordination beyond manual checklist; no
   meta-tag spanning the stack; downstream consumers cannot pin to a stack version.
7. **Corpus is informal.** Per-repo `corpus/` directories, no shared manifest, ENV-var
   based discovery, OpenTFRaw is the only repo with a documented downloader
   (`scripts/fetch_corpus.py`, ~124 GB).

---

## 2. Strategic principles

A single owner with full control over five repos should optimize for **leverage and
clarity**, not for autonomy of each crate. Concretely:

- **One contract, owned by one crate.** `openproteo-core` is that crate. Treat its
  `SpectrumSource` trait + record types as the project's API and version them
  carefully.
- **Vendor crates stay small and stable.** They implement the contract, nothing more.
  Versioning is conservative; breaking changes are rare and announced.
- **The umbrella does umbrella work.** `openproteo-io` is the only place that knows
  about more than one vendor. CLI, Python bindings, format detection, validation, and
  high-level helpers all live there. **ProLance consumes openproteo-io, not the vendor
  crates directly.**
- **One docs site for the stack.** Docusaurus, hosted at one domain, with per-repo
  sections rather than per-repo sites.
- **Coordinated release.** A stack tag (e.g., `stack-v2026.05`) that maps to specific
  versions of each repo, with a single release notes page.
- **Pure Rust everywhere; PyO3 wheels are downstream products, not parallel codebases.**

---

## 3. Direction options

These are reasonable end-states. They are not mutually exclusive but they imply
different amounts of churn.

### Option A: "Tighten what we have" (low churn)

Keep the current 6-repo layout. Wire ProLance through `openproteo-io`. Migrate
OpenProteo docs from mdBook to Docusaurus. Add a stack-level release process and a
shared CI template. Leave Python wheels split per vendor but add an `openproteo[all]`
metapackage that pulls them.

Pros: minimal disruption to already-shipped vendor crates and their tags.
Cons: contributors still see six repos; metadata schema drift can creep back in.

### Option B: "Single Docusaurus hub + meta Python package" (medium churn)

Option A, plus:
- One Docusaurus site under `OpenProteo/docs/` covers all five repos.
  Per-vendor repos keep a stub README that links to the hub.
- One PyPI package `openproteo` re-exports the vendor wheels (or bundles them via a
  single maturin build).
- One stack-version GitHub Release that links to per-crate artifacts.
- OpenProteoCore gets its own README, CHANGELOG, and a public docs section.

Pros: outward-facing surface looks like one project; inward-facing repos stay separate.
Cons: requires keeping the hub in sync; needs a docs-build CI in OpenProteo.

### Option C: "Cargo workspace consolidation" (high churn)

Move the vendor crates into the OpenProteo workspace as `crates/opentfraw/`,
`crates/opentimstdf/`, `crates/openwraw/`. Keep them publishable as independent crates
but develop them in one repo. ProLance stays separate as a consumer.

Pros: one PR can touch the trait + a vendor parser + the umbrella + a test. CI is
trivial. No more `path = "../OpenTFRaw/..."`.
Cons: loses the per-vendor commit history isolation; bigger repo; ties release cadence
together more tightly than may be desired; throws away the already-pushed v1.0.x tags
on the vendor repos. Probably overkill given we already have OpenProteo as an umbrella.

### Recommendation

**Option B.** It captures most of the leverage (one docs site, one Python package, one
release, ProLance routed through the umbrella) without throwing away the
already-published vendor repos and their tags. Option C can be revisited later if the
five-repo overhead becomes painful in practice; nothing in B precludes it.

---

## 4. Priorities (work packages)

Ordered by impact / unblock-value. Each is sized small enough to scope as a single
work package.

### P0 - Critical correctness / unblock

1. **Route ProLance through `openproteo-io`.** Replace the per-vendor adapter modules
   with a single ingester that takes `&mut dyn SpectrumSource`. Delete
   `prolance-ms/src/{thermo,bruker,waters}.rs` and the bespoke
   `prolance-ms/src/mzml.rs`. Pull metadata + records from `openproteo_io::collect`.
   This unblocks Waters ingest immediately (vendor2mzml already handles it) and
   removes the schema-drift surface. Estimated: medium.

2. **OpenProteoCore visibility.** Write a real README + CHANGELOG. Promote the
   ROADMAP out of `.gitignore`. Publish the trait + record reference as the first
   section of the unified docs site. Estimated: small.

### P1 - Strategic alignment

3. **Unify documentation under Docusaurus.** Convert OpenProteo's mdBook to a
   Docusaurus site (matching the OpenWRaw/OpenTFRaw/OpenTimsTDF setup with bun +
   `docusaurus.config.ts`). Absorb the existing per-repo docs as subsections (Vendors
   > Waters / Thermo / Bruker). Per-vendor repos keep a thin README pointing to the
   hub; consider deprecating their standalone Docusaurus sites once the hub ships.
   Estimated: medium.

4. **Coordinated stack release.** Define a `stack-vYYYY.MM` tag scheme owned by
   OpenProteo. Add a `scripts/release-stack.sh` that validates versions, builds, tags,
   and writes combined release notes. First stack release pins the current versions
   (Core 0.1, vendor crates 1.0.x, Umbrella 0.1, ProLance once it lands). Estimated:
   small.

5. **Unified Python distribution.** [DONE - variant (a)] `pip install openproteo`
   ships the metapackage at `python/openproteo` (v0.2.0). Base install
   depends on `openproteo-io`; extras `[thermo]` / `[bruker]` / `[waters]`
   / `[all]` layer on `opentfraw` / `opentimstdf` / `openwraw`. Re-exports
   `detect_format`, `to_mzml`, `iter_spectra`, `Spectrum` from
   `openproteo_io`; adds structural `detect()` + `open_run()` dispatcher
   for the native vendor bindings. CI builds and tests it; the release
   workflow attaches the metapackage sdist. Variant (b) (single maturin
   build with feature-gated vendors) remains as a future option.

### P2 - Quality / consistency

6. **Standardize error handling.** Migrate OpenWRaw to `thiserror` to match the other
   vendor crates. Optionally publish an `openproteo-error` aggregate enum used by the
   umbrella and ProLance so downstream `?`-propagation is uniform. Estimated: small.

7. **Conformance suite as a binary.** [DONE] `vendor2mzml validate <input>`
   accepts any vendor input plus `.mzML` / `.mzML.gz` (via `mzdata`) and runs
   `openproteo_core::conformance::assert_iter_invariants`. Exit codes:
   `0` pass, `2` unrecognised input, `3` conformance failure. JSON output via
   `--json`. CI runs it best-effort against the shared corpus secrets.

8. **Cross-vendor benchmark suite.** Criterion benches for parse-throughput and
   mzML-write latency, one set per vendor crate, results published to the docs site.
   Catches regressions early; informs Phase 4 perf work. Estimated: medium.

### P3 - Investment / future

9. **Shared corpus + manifest.** YAML manifest (vendor, format version, acquisition
   mode, file sizes, expected spectrum counts, fetch URL) used by all five repos.
   Single `fetch-corpus` script. ProLance becomes the natural home for a reference
   Lance store derived from the corpus. Estimated: medium.

10. **Async / object-store readers.** S3 / GCS-backed `SpectrumSource` so the same
    parsers work on cloud-resident vendor files without a download step. Phase 4 on
    the OpenProteoCore roadmap. Estimated: large; do after P0-P2 stabilize.

11. **ProLance integration tests as the stack's truth test.** Once ProLance routes
    through the umbrella (P0 #1), promote ProLance's end-to-end ingest + Lance write +
    mzML roundtrip into the stack-release gate. Estimated: small (after P0 #1).

---

## 5. Suggested sequencing

A reasonable 3-package execution order, each independently shippable:

**Package S1 - "Wire ProLance to the umbrella + surface the core"**
- P0 #1, P0 #2.
- Outcome: one trait, one mzML writer, one set of vendor adapters. Waters ingest works
  for the first time.

**Package S2 - "One docs site, one release train"**
- P1 #3, P1 #4, P2 #6.
- Outcome: a Docusaurus hub at one URL; a stack-vYYYY.MM release that names exact
  versions; uniform error types.

**Package S3 - "One Python install, one validation tool, one benchmark page"**
- P1 #5 (variant a), P2 #7, P2 #8.
- Outcome: `pip install openproteo` brings the whole stack; CI runs the conformance
  binary and benchmarks against a shared corpus.

Anything beyond S3 (shared corpus formalization, async readers, ProLance gate) is
P3 / Phase-4 territory and should be scoped after S1-S3 land and we see real usage.

---

## 6. Decisions needed before execution

1. Confirm Option B (single hub, keep five repos) vs Option C (consolidate workspaces).
2. Confirm Docusaurus as the docs toolchain for the stack (preference stated; this
   doc assumes yes).
3. Confirm "ProLance consumes `openproteo-io`, not vendor crates directly" as a
   binding architectural rule.
4. Pick one of P1 #5 (a) or (b) for the Python distribution shape.
5. Choose whether OpenProteoCore should be treated as a publishable crate
   (recommend yes once a README + CHANGELOG land) or stay path-only.

Once those five questions are answered, S1 is ready to be turned into a concrete work
package with file-level tasks.
