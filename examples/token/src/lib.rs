// A governed ACU-20 token (DX P6-2) — the OpenZeppelin-style "compose reusable
// components" experience. It holds no domain logic of its own: it delegates to the
// importable `Acu20` token component, gates admin with `Ownable`, and gates
// transfers with `Pausable`. This is the "import and inherit" model the audit
// found was missing.
//
// Build to WebAssembly:
//   cargo build --release --target wasm32-unknown-unknown
// Deploy as a governed intent:
//   infrix intent submit --goal CONTRACT_DEPLOY \
//     --wasm target/wasm32-unknown-unknown/release/token.wasm ...

#![no_std]
extern crate alloc;

use infrix_sdk::access::Ownable;
use infrix_sdk::prelude::*;
use infrix_sdk::security::Pausable;
use infrix_sdk::tokens::acu20::{Acu20, Token};

const NAME: &str = "DemoToken";
const SYMBOL: &str = "DEMO";
const DECIMALS: u8 = 18;

fn acu20() -> Acu20 {
    Acu20::new(NAME, SYMBOL, DECIMALS)
}

#[contract]
pub struct DemoToken {}

#[contract_impl]
impl DemoToken {
    /// Deploy-time constructor: the deployer becomes owner and receives the
    /// initial supply.
    #[init]
    pub fn new() -> Self {
        let owner = env::caller();
        Ownable::standard().init(&owner).unwrap();
        // Initial supply = 1,000,000 base units (0x0F4240), big-endian.
        let mut bytes = [0u8; 32];
        bytes[29] = 0x0F;
        bytes[30] = 0x42;
        bytes[31] = 0x40;
        let supply = U256::from_be_bytes(&bytes);
        acu20().mint(&owner, &supply).unwrap();
        Self {}
    }

    /// Transfer tokens — refused while the contract is paused.
    ///
    /// `to` is declared as a Recipient rather than a plain Address. That is what
    /// makes this token ROUTABLE: the schema section records the parameter role,
    /// and a router moving value along a path knows which argument the
    /// destination goes in. `approve` below keeps a plain Address on purpose —
    /// a spender is granted an allowance, it does not receive the asset.
    #[call]
    pub fn transfer(&mut self, to: Recipient, amount: U256) -> Result<bool, Error> {
        Pausable::standard().when_not_paused()?;
        acu20().transfer(to.address(), &amount)
    }

    /// Approve a spender.
    #[call]
    pub fn approve(&mut self, spender: Address, amount: U256) -> Result<bool, Error> {
        acu20().approve(&spender, &amount)
    }

    /// Owner-gated emergency pause.
    #[call]
    pub fn pause(&mut self) -> Result<(), Error> {
        Ownable::standard().only_owner()?;
        Pausable::standard().pause()
    }

    /// Owner-gated unpause.
    #[call]
    pub fn unpause(&mut self) -> Result<(), Error> {
        Ownable::standard().only_owner()?;
        Pausable::standard().unpause()
    }

    /// Read a balance.
    #[view]
    pub fn balance_of(&self, who: Address) -> U256 {
        acu20().balance_of(&who)
    }

    /// Read the total supply.
    #[view]
    pub fn total_supply(&self) -> U256 {
        acu20().total_supply()
    }
}

#[cfg(test)]
mod routing_declaration_tests {
    use super::*;

    /// Matrix row A.DEFI.TRANSFERHOP.RECIPIENT — the end-to-end check on the
    /// crates side: a REAL contract, compiled by the real macro, must publish a
    /// recipient role in the `infrix:schema` section the platform reads.
    ///
    /// Every layer between the signature and the platform is exercised here:
    /// the Recipient type, rust_type_to_role, the schema JSON builder, and the
    /// section the macro embeds.
    #[test]
    fn test_the_token_publishes_a_routable_recipient() {
        let schema = DemoToken::__schema_json();
        assert!(
            schema.contains(r#""name":"to","type":"address","role":"recipient""#),
            "transfer's destination is not declared as a recipient, so a router cannot fill \
             it and this token is unroutable. Schema: {}",
            schema
        );
        // A spender is granted an allowance; it does not receive the asset, and
        // a router must not deliver value to it.
        assert!(
            schema.contains(r#""name":"spender","type":"address"}"#),
            "approve's spender was declared as a recipient; a router would deliver value to \
             an account that was only being granted an allowance. Schema: {}",
            schema
        );
    }
}
