# Releasing OpenMassSpec

Standard operating procedure for cutting an OpenMassSpec release. This repo
ships two PyPI packages and one crate from a single tag:

| Artifact | Kind | Built by | Published to |
| --- | --- | --- | --- |
| `openmassspec-io` | Rust crate | `cargo publish` | crates.io |
| `openmassspec-io` | maturin wheel + sdist | `crates/openmassspec-io-py` | PyPI |
| `openmassspec` | setuptools sdist + wheel (metapackage/facade) | `python/` | PyPI |

The facade (`openmassspec`) depends on the binding (`openmassspec-io`) plus
optional per-vendor readers (`opentfraw`, `opentimstdf`, `openwraw`).

`scripts/release-stack.sh` is the coordinated-release helper: it reads the
pinned stack versions, aggregates CHANGELOG notes, refreshes `STACK.md`,
and (with `--apply`) creates and pushes the umbrella tag. Run it dry-run
first (no `--apply`) to preview.

---

## Why this SOP exists

Every avoidable release problem in this suite has come from one of a small
set of causes. The checklist below is built to catch each one:

- A package shipped **wheels only, no sdist** - conda-forge builds from
  source, so it could not be packaged.
- An sdist shipped **without a LICENSE** - conda-forge requires the license
  file in the source; the recipe cannot be accepted without it.
- A **wheel-matrix leg failed on a transient runner flake** (Windows
  `curl`/HTTP2 while fetching a crate; macOS SDK download) and silently
  **skipped the PyPI publish**, leaving crates.io ahead of PyPI.
- **Version drift** between `versions.toml`, `STACK.md`, PyPI, and
  crates.io.
- A scary-looking `include_bytes!` / build dependency was **assumed** to
  break source builds without being **tested** (it was behind a default-off
  feature and did not).

---

## Pre-flight

1. **Pick the version.** Bump `[workspace.package] version` in `Cargo.toml`
   and the hardcoded `version` in **both** pyprojects
   (`crates/openmassspec-io-py/pyproject.toml` and `python/pyproject.toml`).
   Keep all three in lockstep.
   - The version must not already exist on crates.io or PyPI (publishes are
     irreversible; you cannot overwrite or re-upload).
2. **Reconcile versions.** Confirm the target is consistent with the intent
   in `versions.toml` (ops repo). Regenerate `STACK.md` with
   `scripts/release-stack.sh --write-stack-md --apply`.
3. **CHANGELOG.** Add a dated entry under a new version heading.

## Packaging invariants (the conda-forge gate)

For **each** published Python package, before tagging:

4. **sdist exists.** `openmassspec-io` has a `build-sdist` job; the facade is
   built with `python -m build --sdist`. Both must produce a `.tar.gz`.
5. **LICENSE is in the sdist.** The repo `LICENSE` lives at the root, but
   each package is built from a subdirectory, so a copy must be co-located:
   - `openmassspec-io` (maturin): `crates/openmassspec-io-py/LICENSE` plus
     `include = [{ path = "LICENSE", format = "sdist" }]` under
     `[tool.maturin]`. maturin forbids `..` in `include`, hence the copy.
   - `openmassspec` (setuptools): `python/LICENSE`. setuptools auto-includes
     `LICEN[CS]E*` next to `pyproject.toml`; verify it lands in the sdist.
6. **Build and install from source in a clean env.** Do not assume - test:

   ```sh
   # binding
   ( cd crates/openmassspec-io-py && uvx maturin sdist --out /tmp/op-io )
   tar tzf /tmp/op-io/*.tar.gz | grep -i licen        # LICENSE present?
   uv venv /tmp/v1 && uv pip install --python /tmp/v1/bin/python \
       --no-binary :all: /tmp/op-io/*.tar.gz
   /tmp/v1/bin/python -c "import openmassspec_io"

   # facade
   python -m build --sdist --outdir /tmp/op python/
   tar tzf /tmp/op/*.tar.gz | grep -i licen
   ```

   If a package embeds a generated data file via `include_bytes!` or a
   build script, check whether that code path is actually compiled in the
   published artifact (feature gates, `cfg`) before concluding the sdist is
   broken.

## Release

7. **Confirm release-readiness (CI/audit).** `publish.yml` triggers
   directly on `push: tags: ["v*"]` and cannot `needs:` a job defined in
   `ci.yml` or `audit.yml` (GitHub Actions has no cross-workflow
   `needs:`), so this has to be checked before the tag exists.
   `scripts/release-stack.sh --tag --apply` runs
   `scripts/check-release-ready.sh` automatically before it creates or
   pushes the tag, and refuses to tag if CI or the audit workflow hasn't
   run, is still in progress, or didn't succeed on the target commit. If
   tagging by hand instead, run the same check yourself first:

   ```sh
   scripts/check-release-ready.sh [ref]   # defaults to HEAD
   ```

   The check is bypassable with `release-stack.sh ... --skip-release-check`
   for exceptional cases; there's no equivalent bypass for a by-hand tag
   other than skipping the script entirely.
8. **Tag.** `scripts/release-stack.sh --name vX.Y.Z --tag --push --apply`
   (or tag `vX.Y.Z` by hand). `publish.yml` triggers on `v*` and runs:
   crate publish (`continue-on-error`), openmassspec-io wheels + sdist +
   publish, and the openmassspec metapackage build + publish.
9. **Watch the run.** A transient wheel-leg failure will skip
   `publish-openmassspec-io` (it `needs: [build-wheels, build-sdist]`). This
   is almost always a runner network flake, not a real error. Fix:

   ```sh
   gh run rerun <run-id> --failed
   ```

   Re-running the failed leg lets the skipped publish proceed. The crate
   publish is `continue-on-error`, so a re-tag will not fail on an
   already-published crate. (The facade publish only `needs:
   build-metapackage`, so a wheel flake does not block it.)

## Post-release

10. **Verify on PyPI.** For each package, confirm the new version has an
   sdist and that the sdist contains `LICENSE`:

   ```sh
   python - <<'PY'
   import json, urllib.request, io, tarfile
   for pkg in ("openmassspec-io", "openmassspec"):
       d = json.load(urllib.request.urlopen(f"https://pypi.org/pypi/{pkg}/json"))
       v = d["info"]["version"]; urls = d["releases"][v]
       sd = [u for u in urls if u["packagetype"] == "sdist"]
       raw = urllib.request.urlopen(sd[0]["url"]).read()
       lic = [n for n in tarfile.open(fileobj=io.BytesIO(raw)).getnames() if "licen" in n.lower()]
       print(pkg, v, "sdist" if sd else "NO SDIST", "LICENSE" if lic else "NO LICENSE")
   PY
   ```

11. **Update `versions.toml`** in the ops repo and commit
    (`versions: bump OpenMassSpec to X.Y.Z`).

## conda-forge (two-stage)

12. The facade's recipe depends on the binding, so submit in order:
    - First `openmassspec-io` to `conda-forge/staged-recipes`. Wait for the
      feedstock to be created and the package to appear on the channel.
    - Then `openmassspec` (a `noarch: python` recipe whose `run` requirement
      includes `openmassspec-io`).
    - Build each recipe locally in `condaforge/linux-anvil-cos7-x86_64`
      (install `conda-forge-pinning`, build with its
      `conda_build_config.yaml`) before opening the PR. Every recipe using a
      compiler must declare `{{ stdlib("c") }}` or the linter fails.
