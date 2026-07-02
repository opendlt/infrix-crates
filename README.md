# infrix-crates

**The Rust SDK for authoring [Infrix](https://github.com/opendlt) smart contracts.**

> **Infrix — governed, verifiable execution on Accumulate.** New here? Start at
> [infrix.opendlt.org](https://infrix.opendlt.org) · try it live at
> [play.infrix.opendlt.org](https://play.infrix.opendlt.org).

This repo is the Cargo workspace extracted from the Infrix monorepo. It holds the
crates a contract author depends on to write WASM contracts that run on the
Infrix governed execution layer, published to [crates.io](https://crates.io)
under the `infrix-*` names.

## Crates

| Crate | Docs | What it is |
|-------|------|------------|
| [`infrix-types`](./infrix-types/README.md) | [crates.io](https://crates.io/crates/infrix-types) | Core types for Infrix smart contracts (governance, ABI, the canonical `IntentGoalType` enum) |
| [`infrix-macros`](./infrix-macros/README.md) | [crates.io](https://crates.io/crates/infrix-macros) | Procedural macros (`#[contract]`, `#[call]`/`#[view]`/`#[init]`, governance + ABI generation) |
| [`infrix-sdk`](./infrix-sdk/README.md) | [crates.io](https://crates.io/crates/infrix-sdk) | The contract-authoring SDK: host bindings, storage, token standards, the `prelude` |

## Examples

- [`examples/counter`](./examples/counter) — the canonical governed counter
  contract, verified to compile to `wasm32-unknown-unknown`. Copy it as your
  starting point.

## Use it

```toml
[dependencies]
infrix-sdk = { version = "0.1.0", default-features = false, features = ["alloc"] }
```

```rust
#![no_std]
use infrix_sdk::prelude::*;

#[contract]
pub struct Counter { /* ... */ }
```

`infrix init` (in the Infrix CLI) scaffolds a new contract project wired to the
published `infrix-sdk` crate.

## Build & test

```sh
cargo build --workspace
cargo test --workspace
```

Contracts compile to WebAssembly:

```sh
cargo build --target wasm32-unknown-unknown --release
```

## License

MIT — see [LICENSE](LICENSE).
