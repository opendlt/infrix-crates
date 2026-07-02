// The canonical Infrix contract example (DX P2-1): a governed on-chain counter.
//
// Build to WebAssembly:
//   cargo build --release --target wasm32-unknown-unknown
// Deploy the resulting counter.wasm as a governed intent:
//   infrix intent submit --goal CONTRACT_DEPLOY \
//     --wasm target/wasm32-unknown-unknown/release/counter.wasm ...

#![no_std]
extern crate alloc;

use infrix_sdk::prelude::*;

#[contract]
pub struct Counter {
    value: U256,
}

#[contract_impl]
impl Counter {
    /// Deploy-time constructor.
    #[init]
    pub fn new() -> Self {
        Self { value: U256::ZERO }
    }

    /// State-mutating entry point — routed as a governed CONTRACT_CALL.
    #[call]
    pub fn increment(&mut self) -> Result<(), Error> {
        self.value = self.value.checked_add(&U256::ONE).ok_or(Error::Overflow)?;
        Ok(())
    }

    /// Read-only entry point — no state change.
    #[view]
    pub fn get(&self) -> U256 {
        self.value
    }
}
