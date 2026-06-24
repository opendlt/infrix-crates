# infrix-crates

**The Rust SDK for authoring [Infrix](https://github.com/opendlt/infrix-accumen) smart contracts.**

This repo is the Cargo workspace extracted from the Infrix monorepo. It holds the
crates a contract author depends on to write WASM contracts that run on the
Infrix governed execution layer, published to [crates.io](https://crates.io)
under the `infrix-*` names.

## Crates

| Crate | crates.io | What it is |
|-------|-----------|------------|
| [`infrix-types`](https://crates.io/crates/infrix-types) | v0.1.0 | Core types for Infrix smart contracts (governance, ABI, the canonical `IntentGoalType` enum) |
| [`infrix-macros`](https://crates.io/crates/infrix-macros) | v0.1.0 | Procedural macros (`#[contract]`, ABI schema-section generation, selector calculation) |
| [`infrix-sdk`](https://crates.io/crates/infrix-sdk) | v0.1.0 | The contract-authoring SDK: host-function bindings, storage, the `prelude` |

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
