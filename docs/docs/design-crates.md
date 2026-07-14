# Crate layout

OpenMassSpec is split across seven git repositories. Each repo can be
released independently, but their version constraints in
`OpenMassSpec/Cargo.toml` pin the combinations that have been validated
together.

## Repositories

| Repo            | Crate(s)                                                  |
| --------------- | --------------------------------------------------------- |
| `OpenMassSpecCore` | `openmassspec-core`                                         |
| `OpenMassSpec`     | `openmassspec-io`, `openmassspec-io-cli`, `openmassspec-io-py`  |
| `OpenTFRaw`      | `opentfraw`                                               |
| `OpenTimsTDF`        | `opentimstdf`                                             |
| `OpenWRaw`       | `openwraw`                                                |
| `OpenARaw`       | `openaraw`                                                |
| `OpenSXRaw`      | `opensxraw`                                               |

## Why seven repos and not one monorepo?

- Each vendor reader has its own conformance corpus, its own CHANGELOG,
  and its own release cadence. Bruker firmware ships new TDF columns
  often enough that `opentimstdf` cuts patch releases independently of
  the others.
- `openmassspec-core` is the only crate guaranteed to be reverse-
  dependency-stable across the 0.x line. Releasing it from its own
  repo makes it obvious when a `core` change forces a vendor-crate
  bump.
- `OpenMassSpec` is a thin umbrella. Keeping its release cadence
  decoupled from the vendor crates lets us ship CLI/Python fixes
  without re-cutting vendor releases.

## Feature flags

`openmassspec-io` exposes a feature per vendor plus an `all` convenience
feature:

```toml
[features]
default = []
all     = ["thermo", "bruker", "waters", "agilent", "sciex"]
thermo  = ["dep:opentfraw"]
bruker  = ["dep:opentimstdf"]
waters  = ["dep:openwraw"]
agilent = ["dep:openaraw"]
sciex   = ["dep:opensxraw"]
arrow   = ["openmassspec-core/arrow"]
```

Build a Thermo-only binary with:

```sh
cargo build --release -p openmassspec-io-cli --no-default-features --features thermo
```

## Workspace dependencies

The `OpenMassSpec` `Cargo.toml` declares all cross-crate paths in
`[workspace.dependencies]`. Member crates pick them up via
`{ workspace = true }`, so version bumps land in exactly one place.
