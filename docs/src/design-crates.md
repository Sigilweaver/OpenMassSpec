# Crate layout

OpenProteo is split across five git repositories. Each repo can be
released independently, but their version constraints in
`OpenProteo/Cargo.toml` pin the combinations that have been validated
together.

## Repositories

| Repo            | Crate(s)                                                  |
| --------------- | --------------------------------------------------------- |
| `OpenProteoCore` | `openproteo-core`                                         |
| `OpenProteo`     | `openproteo-io`, `openproteo-io-cli`, `openproteo-io-py`  |
| `OpenTFRaw`      | `opentfraw`                                               |
| `OpenTDF`        | `opentimstdf`                                             |
| `OpenWRaw`       | `openwraw`                                                |

## Why five repos and not one monorepo?

- Each vendor reader has its own conformance corpus, its own CHANGELOG,
  and its own release cadence. Bruker firmware ships new TDF columns
  often enough that `opentimstdf` cuts patch releases independently of
  the others.
- `openproteo-core` is the only crate guaranteed to be reverse-
  dependency-stable across the 0.x line. Releasing it from its own
  repo makes it obvious when a `core` change forces a vendor-crate
  bump.
- `OpenProteo` is a thin umbrella. Keeping its release cadence
  decoupled from the vendor crates lets us ship CLI/Python fixes
  without re-cutting vendor releases.

## Feature flags

`openproteo-io` exposes a feature per vendor plus an `all` convenience
feature:

```toml
[features]
default = ["all"]
all     = ["thermo", "bruker", "waters", "arrow"]
thermo  = ["dep:opentfraw"]
bruker  = ["dep:opentimstdf"]
waters  = ["dep:openwraw"]
arrow   = ["openproteo-core/arrow"]
```

Build a Thermo-only binary with:

```sh
cargo build --release -p openproteo-io-cli --no-default-features --features thermo
```

## Workspace dependencies

The `OpenProteo` `Cargo.toml` declares all cross-crate paths in
`[workspace.dependencies]`. Member crates pick them up via
`{ workspace = true }`, so version bumps land in exactly one place.
