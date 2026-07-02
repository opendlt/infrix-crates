# infrix-sdk

The Rust SDK for writing **Infrix smart contracts** that compile to WebAssembly
and run on Infrix's governed execution spine over Accumulate.

```toml
[dependencies]
infrix-sdk = "0.1"
```

## A contract in 20 lines

```rust
use infrix_sdk::prelude::*;

#[contract]
pub struct Counter {
    value: U256,
}

#[contract_impl]
impl Counter {
    #[init]
    pub fn new() -> Self {
        Self { value: U256::ZERO }
    }

    #[call]
    pub fn increment(&mut self) -> Result<(), Error> {
        self.value = self.value.checked_add(&U256::ONE).ok_or(Error::Overflow)?;
        Ok(())
    }

    #[view]
    pub fn get(&self) -> U256 {
        self.value
    }
}
```

Build it (see [`examples/counter`](../examples/counter)):

```sh
cargo build --release --target wasm32-unknown-unknown
```

Then deploy the `.wasm` as a governed intent: `infrix intent submit --goal
CONTRACT_DEPLOY --wasm target/wasm32-unknown-unknown/release/counter.wasm ...`.

## What's in the box

- **Contract macros** (re-exported from `infrix-macros`): `#[contract]`,
  `#[contract_impl]`, `#[init]`, `#[call]`, `#[view]`, `#[event]`,
  `#[storage_map]`.
- **Governance macros**: `#[governed]`, `#[require_approval]`,
  `#[require_capability]`, `#[require_role]`, `#[evidenced]`.
- **Host APIs**: `storage` (state), `env` (block/caller context), `l0`
  (Accumulate L0 reads/transfers), `events`, `crypto`.
- **Token standards**: `tokens::acu20`, `tokens::acu721`, `tokens::acu1155`.
- **Adaptive modules**: `temporal`, `shadow`, `sentient`, `shapes`, `swarm`.
- **Types**: re-exports `infrix-types` (`U256`, `Error`, ABI, `IntentGoalType`).

`#![no_std]` + `alloc` by default so contracts compile small for
`wasm32-unknown-unknown`.
