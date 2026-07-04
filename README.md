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

## Reusable components (the OpenZeppelin analogs)

Don't hand-roll standard logic — **compose** these importable components (DX P6-2):

| Component | Import | What it gives you |
|-----------|--------|-------------------|
| **ACU-20 token** | `infrix_sdk::tokens::acu20::Acu20` | A full storage-backed fungible token (balances, allowances, `mint`, checked math). The ERC-20 analog. |
| **ACU-721 NFT** | `infrix_sdk::tokens::acu721::Acu721` | A full storage-backed non-fungible token — ownership, balances, approvals, operator approvals, `mint`/`mint_next` (auto-increment), and EIP-2981 royalties. The ERC-721 analog. |
| **ACU-1155 multi-token** | `infrix_sdk::tokens::acu1155::Acu1155` | A storage-backed multi-token — any mix of fungible + non-fungible ids in one contract, per-(owner,id) balances, operator approvals, `mint`. The ERC-1155 analog. |
| **Ownable** | `infrix_sdk::access::Ownable` | Single-owner access control — `init`, `only_owner()`, `transfer_ownership`. |
| **AccessControl** | `infrix_sdk::access::AccessControl` | Role-based access — `grant_role`/`revoke_role`/`has_role`/`only_role`. |
| **Pausable** | `infrix_sdk::security::Pausable` | An emergency pause switch — `pause`/`unpause`/`when_not_paused()`. |
| **ReentrancyGuard** | `infrix_sdk::storage::ReentrancyGuard` | Re-entrancy protection. |

All are `use`-and-compose (no inheritance): hold them in your contract and delegate.
Access components **fail closed** (no owner set ⇒ `only_owner` denies).

## Examples

- [`examples/counter`](./examples/counter) — the canonical governed counter
  contract, verified to compile to `wasm32-unknown-unknown`. Copy it as your
  starting point.
- [`examples/token`](./examples/token) — a governed **ACU-20 token that composes
  `Acu20` + `Ownable` + `Pausable`** with almost no domain code of its own
  (verified to build to `wasm32`). The "import and inherit" model.

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
