//! Infrix Smart Contract SDK
//!
//! Policy-governed execution and evidence layer for Accumulate.
//!
//! This SDK provides all the tools needed to develop smart contracts for the Infrix platform.
//! Contracts execute as subordinate plugins within the governance pipeline:
//!   Intent -> Plan -> Approval -> **Execution** -> Outcome -> Evidence -> Anchor
//!
//! # Features
//!
//! - **Storage API**: Read/write contract state
//! - **L0 API**: Interact with Accumulate L0 (accounts, tokens, data)
//! - **Environment API**: Access execution context
//! - **Events API**: Emit indexed events
//! - **Crypto API**: Cryptographic operations
//! - **Contract Macros**: Procedural macros for contract development
//!
//! # Example
//!
//! ```ignore
//! use infrix_sdk::prelude::*;
//!
//! #[contract]
//! pub struct Counter {
//!     value: U256,
//! }
//!
//! #[contract_impl]
//! impl Counter {
//!     #[init]
//!     pub fn new() -> Self {
//!         Self { value: U256::ZERO }
//!     }
//!
//!     #[call]
//!     pub fn increment(&mut self) -> Result<(), Error> {
//!         self.value = self.value.checked_add(&U256::ONE)
//!             .ok_or(Error::Overflow)?;
//!         Ok(())
//!     }
//!
//!     #[view]
//!     pub fn get(&self) -> U256 {
//!         self.value
//!     }
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

// When compiling for no_std + alloc (e.g. wasm32-unknown-unknown),
// bring core alloc types into scope so all modules can use Vec, String, format! etc.
#[cfg(feature = "alloc")]
#[allow(unused_imports)]
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

// Re-export infrix_types at crate root so proc macro generated code
// (which references `infrix_types::X`) can resolve through `use infrix_sdk::prelude::*`.
pub use infrix_types;

// Re-export types and macros
pub use infrix_types::{
    self as types, Address, CallResult, Context, Decode, Encode, Error, Event, EventTrait,
    FunctionAbi, Hash, IntoResult, Mutability, Signature, SignatureType, Topic, U256,
};

pub use infrix_macros::{call, contract, contract_impl, event, init, storage_map, view};
pub use infrix_macros::{evidenced, governed, require_approval, require_capability, require_role};
pub use infrix_macros::{infrix_fuzz, infrix_test};

/// Programmable Time — temporal primitives for historical queries,
/// self-awareness, and (in future phases) scheduling and simulation.
pub mod temporal;

/// Shadow State — private-by-default contract execution with transparent
/// encryption based on visibility policies.
pub mod shadow;

/// Sentient Contracts — native data feeds, timestamps, and randomness.
pub mod sentient;

/// Shape-Shifting Contracts — adaptive parameter access and shape queries.
///
/// Provides type-safe functions to read the currently active shape's
/// parameters, query shape history, and inspect transition state.
pub mod shapes;

/// Swarm Contracts — shared channels, member state, and coordinated actions.
///
/// Provides functions for reading/writing the shared communication channel,
/// querying other members' state, triggering coordinated actions, and
/// checking the collective immune state.
pub mod swarm;

/// Governance module — intent submission, object management, approvals,
/// trust evaluation, capability/role management, and evidence access
/// from within smart contracts.
pub mod governance;

/// Mission Control — production observability for smart contracts.
///
/// Provides host functions for contracts to read their own metrics:
/// call counts, error rate, gas averages, uptime, anomaly scores, and SLO compliance.
pub mod mission;

/// Contract testing framework.
///
/// Provides `TestContext`, `Receipt`, `QueryResult`, assertion macros, and
/// the `#[infrix_test]` / `#[infrix_fuzz]` attributes for writing contract
/// tests that work both in WASM (via `infrix test`) and natively (via
/// `cargo test`).
pub mod testing;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::types::{
        Address, CallResult, Context, Decode, Encode, Error, Event, EventTrait, FunctionAbi, Hash,
        IntoResult, Mutability, Signature, SignatureType, Topic, U256,
    };

    pub use crate::{call, contract, contract_impl, event, init, storage_map, view};
    pub use crate::{evidenced, governed, require_approval, require_capability, require_role};

    pub use crate::crypto;
    pub use crate::env;
    pub use crate::events;
    pub use crate::governance;
    pub use crate::l0;
    pub use crate::storage;
}

// =============================================================================
// Host Function Declarations (extern "C")
// =============================================================================

#[cfg(target_arch = "wasm32")]
mod host {
    //! Host function declarations for the WASM runtime
    //!
    //! These functions are provided by the Infrix runtime and called by contracts.

    #[link(wasm_import_module = "infrix")]
    extern "C" {
        // Storage operations
        pub fn host_storage_get(key_ptr: *const u8, key_len: u32, value_ptr: *mut u8) -> i32;
        pub fn host_storage_set(
            key_ptr: *const u8,
            key_len: u32,
            value_ptr: *const u8,
            value_len: u32,
        );
        pub fn host_storage_delete(key_ptr: *const u8, key_len: u32);
        pub fn host_storage_has(key_ptr: *const u8, key_len: u32) -> i32;

        // Environment
        pub fn host_env_caller(output_ptr: *mut u8) -> i32;
        pub fn host_env_self_address(output_ptr: *mut u8) -> i32;
        pub fn host_env_owner(output_ptr: *mut u8) -> i32;
        pub fn host_env_block_height() -> u64;
        pub fn host_env_block_time() -> u64;
        pub fn host_env_value(output_ptr: *mut u8);
        pub fn host_env_gas_remaining() -> u64;
        pub fn host_env_tx_hash(output_ptr: *mut u8);

        // L0 Account Operations
        pub fn host_l0_get_account(url_ptr: *const u8, url_len: u32, output_ptr: *mut u8) -> i32;
        pub fn host_l0_get_balance(url_ptr: *const u8, url_len: u32, output_ptr: *mut u8) -> i32;
        pub fn host_l0_get_data(
            url_ptr: *const u8,
            url_len: u32,
            entry_hash_ptr: *const u8,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_l0_create_account(url_ptr: *const u8, url_len: u32, account_type: u8) -> i32;
        pub fn host_l0_write_data(
            url_ptr: *const u8,
            url_len: u32,
            data_ptr: *const u8,
            data_len: u32,
        ) -> i32;
        pub fn host_l0_transfer(
            from_ptr: *const u8,
            from_len: u32,
            to_ptr: *const u8,
            to_len: u32,
            amount_ptr: *const u8,
        ) -> i32;
        pub fn host_l0_burn_credits(url_ptr: *const u8, url_len: u32, amount: u64) -> i32;

        // L0 Authority Operations
        pub fn host_l0_get_authority(url_ptr: *const u8, url_len: u32, output_ptr: *mut u8) -> i32;
        pub fn host_l0_check_authority(
            url_ptr: *const u8,
            url_len: u32,
            signer_ptr: *const u8,
            signer_len: u32,
        ) -> i32;

        // Events
        pub fn host_event_emit(
            topics_ptr: *const u8,
            topics_len: u32,
            data_ptr: *const u8,
            data_len: u32,
        );

        // Cryptography
        pub fn host_crypto_sha256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8);
        pub fn host_crypto_sha3_256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8);
        pub fn host_crypto_keccak256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8);
        pub fn host_crypto_blake2b_256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8);
        pub fn host_crypto_ripemd160(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8);
        pub fn host_crypto_ed25519_verify(
            msg_ptr: *const u8,
            msg_len: u32,
            sig_ptr: *const u8,
            pubkey_ptr: *const u8,
        ) -> i32;
        pub fn host_crypto_secp256k1_verify(
            msg_ptr: *const u8,
            msg_len: u32,
            sig_ptr: *const u8,
            pubkey_ptr: *const u8,
        ) -> i32;
        pub fn host_crypto_secp256k1_recover(
            msg_ptr: *const u8,
            msg_len: u32,
            sig_ptr: *const u8,
            recovery_id: u8,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_crypto_bls12_381_verify(
            msg_ptr: *const u8,
            msg_len: u32,
            sig_ptr: *const u8,
            pubkey_ptr: *const u8,
        ) -> i32;

        // Cross-contract calls
        pub fn host_call_contract(
            address_ptr: *const u8,
            address_len: u32,
            input_ptr: *const u8,
            input_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_delegate_call(
            address_ptr: *const u8,
            address_len: u32,
            input_ptr: *const u8,
            input_len: u32,
            output_ptr: *mut u8,
        ) -> i32;

        // Utility
        pub fn host_log(msg_ptr: *const u8, msg_len: u32);
        pub fn host_revert(msg_ptr: *const u8, msg_len: u32) -> !;
        pub fn host_assert(condition: i32, msg_ptr: *const u8, msg_len: u32);

        // === Governance Host Functions ===

        // Intent operations
        pub fn host_governance_submit_intent(
            goal_ptr: *const u8,
            goal_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_governance_get_intent_status(
            intent_id_ptr: *const u8,
            intent_id_len: u32,
            output_ptr: *mut u8,
        ) -> i32;

        // Object operations
        pub fn host_governance_create_object(
            obj_type_ptr: *const u8,
            obj_type_len: u32,
            fields_ptr: *const u8,
            fields_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_governance_get_object(
            obj_type_ptr: *const u8,
            obj_type_len: u32,
            id_ptr: *const u8,
            id_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_governance_transition_object(
            obj_type_ptr: *const u8,
            obj_type_len: u32,
            id_ptr: *const u8,
            id_len: u32,
            state_ptr: *const u8,
            state_len: u32,
        ) -> i32;

        // Approval operations
        pub fn host_governance_require_approval(
            role_ptr: *const u8,
            role_len: u32,
            threshold: u32,
        ) -> i32;
        pub fn host_governance_check_approval(plan_id_ptr: *const u8, plan_id_len: u32) -> i32;

        // Trust operations
        pub fn host_governance_get_trust_profile(
            profile_id_ptr: *const u8,
            profile_id_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_governance_evaluate_trust(
            profile_id_ptr: *const u8,
            profile_id_len: u32,
            output_ptr: *mut u8,
        ) -> i32;

        // Capability operations
        pub fn host_governance_has_capability(
            identity_ptr: *const u8,
            identity_len: u32,
            cap_ptr: *const u8,
            cap_len: u32,
        ) -> i32;
        pub fn host_governance_grant_capability(
            grantee_ptr: *const u8,
            grantee_len: u32,
            caps_ptr: *const u8,
            caps_len: u32,
            scope_ptr: *const u8,
            scope_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
        pub fn host_governance_revoke_capability(grant_id_ptr: *const u8, grant_id_len: u32)
            -> i32;

        // Role operations
        pub fn host_governance_has_role(
            identity_ptr: *const u8,
            identity_len: u32,
            role_ptr: *const u8,
            role_len: u32,
        ) -> i32;
        pub fn host_governance_assign_role(
            identity_ptr: *const u8,
            identity_len: u32,
            role_ptr: *const u8,
            role_len: u32,
            scope_ptr: *const u8,
            scope_len: u32,
            output_ptr: *mut u8,
        ) -> i32;

        // Evidence operations
        pub fn host_governance_get_evidence(
            intent_id_ptr: *const u8,
            intent_id_len: u32,
            output_ptr: *mut u8,
        ) -> i32;

        // Policy operations
        pub fn host_governance_evaluate_policy(
            scope_ptr: *const u8,
            scope_len: u32,
            op_type_ptr: *const u8,
            op_type_len: u32,
            operands_ptr: *const u8,
            operands_len: u32,
            output_ptr: *mut u8,
        ) -> i32;
    }
}

// Mock host functions for non-WASM builds (testing)
#[cfg(not(target_arch = "wasm32"))]
mod host {
    use core::cell::RefCell;

    #[cfg(feature = "std")]
    use std::collections::HashMap;

    #[cfg(feature = "std")]
    thread_local! {
        static STORAGE: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
        static CALLER: RefCell<Vec<u8>> = RefCell::new(vec![0u8; 256]);
        static BLOCK_HEIGHT: RefCell<u64> = RefCell::new(1);
    }

    pub unsafe fn host_storage_get(key_ptr: *const u8, key_len: u32, value_ptr: *mut u8) -> i32 {
        #[cfg(feature = "std")]
        {
            let key = core::slice::from_raw_parts(key_ptr, key_len as usize);
            STORAGE.with(|s| {
                if let Some(value) = s.borrow().get(key) {
                    core::ptr::copy_nonoverlapping(value.as_ptr(), value_ptr, value.len());
                    value.len() as i32
                } else {
                    -1
                }
            })
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (key_ptr, key_len, value_ptr);
            -1
        }
    }

    pub unsafe fn host_storage_set(
        key_ptr: *const u8,
        key_len: u32,
        value_ptr: *const u8,
        value_len: u32,
    ) {
        #[cfg(feature = "std")]
        {
            let key = core::slice::from_raw_parts(key_ptr, key_len as usize).to_vec();
            let value = core::slice::from_raw_parts(value_ptr, value_len as usize).to_vec();
            STORAGE.with(|s| s.borrow_mut().insert(key, value));
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (key_ptr, key_len, value_ptr, value_len);
        }
    }

    pub unsafe fn host_storage_delete(key_ptr: *const u8, key_len: u32) {
        #[cfg(feature = "std")]
        {
            let key = core::slice::from_raw_parts(key_ptr, key_len as usize);
            STORAGE.with(|s| s.borrow_mut().remove(key));
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (key_ptr, key_len);
        }
    }

    pub unsafe fn host_storage_has(key_ptr: *const u8, key_len: u32) -> i32 {
        #[cfg(feature = "std")]
        {
            let key = core::slice::from_raw_parts(key_ptr, key_len as usize);
            STORAGE.with(|s| if s.borrow().contains_key(key) { 1 } else { 0 })
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (key_ptr, key_len);
            0
        }
    }

    pub unsafe fn host_env_caller(output_ptr: *mut u8) -> i32 {
        #[cfg(feature = "std")]
        {
            CALLER.with(|c| {
                let caller = c.borrow();
                core::ptr::copy_nonoverlapping(caller.as_ptr(), output_ptr, caller.len());
                caller.len() as i32
            })
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = output_ptr;
            0
        }
    }

    pub unsafe fn host_env_self_address(output_ptr: *mut u8) -> i32 {
        let _ = output_ptr;
        0
    }

    pub unsafe fn host_env_owner(output_ptr: *mut u8) -> i32 {
        let _ = output_ptr;
        0
    }

    pub unsafe fn host_env_block_height() -> u64 {
        #[cfg(feature = "std")]
        {
            BLOCK_HEIGHT.with(|h| *h.borrow())
        }
        #[cfg(not(feature = "std"))]
        {
            1
        }
    }

    pub unsafe fn host_env_block_time() -> u64 {
        1704067200 // 2024-01-01 00:00:00 UTC
    }

    pub unsafe fn host_env_value(output_ptr: *mut u8) {
        // Zero value
        for i in 0..32 {
            *output_ptr.add(i) = 0;
        }
    }

    pub unsafe fn host_env_gas_remaining() -> u64 {
        1_000_000
    }

    pub unsafe fn host_env_tx_hash(output_ptr: *mut u8) {
        for i in 0..32 {
            *output_ptr.add(i) = 0;
        }
    }

    pub unsafe fn host_l0_get_account(
        _url_ptr: *const u8,
        _url_len: u32,
        _output_ptr: *mut u8,
    ) -> i32 {
        -1 // Not found in mock
    }

    pub unsafe fn host_l0_get_balance(
        _url_ptr: *const u8,
        _url_len: u32,
        _output_ptr: *mut u8,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_l0_get_data(
        _url_ptr: *const u8,
        _url_len: u32,
        _entry_hash_ptr: *const u8,
        _output_ptr: *mut u8,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_l0_create_account(
        _url_ptr: *const u8,
        _url_len: u32,
        _account_type: u8,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_l0_write_data(
        _url_ptr: *const u8,
        _url_len: u32,
        _data_ptr: *const u8,
        _data_len: u32,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_l0_transfer(
        _from_ptr: *const u8,
        _from_len: u32,
        _to_ptr: *const u8,
        _to_len: u32,
        _amount_ptr: *const u8,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_l0_burn_credits(_url_ptr: *const u8, _url_len: u32, _amount: u64) -> i32 {
        -1
    }

    pub unsafe fn host_l0_get_authority(
        _url_ptr: *const u8,
        _url_len: u32,
        _output_ptr: *mut u8,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_l0_check_authority(
        _url_ptr: *const u8,
        _url_len: u32,
        _signer_ptr: *const u8,
        _signer_len: u32,
    ) -> i32 {
        0
    }

    pub unsafe fn host_event_emit(
        _topics_ptr: *const u8,
        _topics_len: u32,
        _data_ptr: *const u8,
        _data_len: u32,
    ) {
        // No-op in mock
    }

    // P0-003 (2026-05-02): the std/test host crypto mocks now compute
    // real digests + verify real signatures rather than zeroing
    // outputs and unconditionally returning 1. Pre-closure, every
    // mock returned all-zero hashes and "valid" for every signature
    // — a failure mode that trained contract authors into unsafe
    // assumptions (the test-mode path could never catch a bad
    // signature or a wrong digest). The mocks below now match the
    // wasm32 host's contract on the non-wasm32 (test) target.
    //
    // wasm32 builds keep the extern "C" host imports defined above;
    // operators wire real implementations on the host side.

    use blake2::{digest::consts::U32, Blake2b};
    use ed25519_dalek::{
        Signature as Ed25519Signature, Verifier as _, VerifyingKey as Ed25519VerifyingKey,
    };
    use k256::ecdsa::{
        signature::hazmat::PrehashVerifier as _, RecoveryId, Signature as K256Signature,
        VerifyingKey as K256VerifyingKey,
    };
    use ripemd::Ripemd160;
    use sha2::{Digest as _, Sha256};
    use sha3::{Keccak256, Sha3_256};

    pub unsafe fn host_crypto_sha256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8) {
        let data = core::slice::from_raw_parts(data_ptr, data_len as usize);
        let digest = Sha256::digest(data);
        core::ptr::copy_nonoverlapping(digest.as_ptr(), output_ptr, 32);
    }

    pub unsafe fn host_crypto_sha3_256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8) {
        let data = core::slice::from_raw_parts(data_ptr, data_len as usize);
        let digest = Sha3_256::digest(data);
        core::ptr::copy_nonoverlapping(digest.as_ptr(), output_ptr, 32);
    }

    pub unsafe fn host_crypto_keccak256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8) {
        let data = core::slice::from_raw_parts(data_ptr, data_len as usize);
        let digest = Keccak256::digest(data);
        core::ptr::copy_nonoverlapping(digest.as_ptr(), output_ptr, 32);
    }

    pub unsafe fn host_crypto_blake2b_256(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8) {
        let data = core::slice::from_raw_parts(data_ptr, data_len as usize);
        let mut hasher = Blake2b::<U32>::new();
        hasher.update(data);
        let digest = hasher.finalize();
        core::ptr::copy_nonoverlapping(digest.as_ptr(), output_ptr, 32);
    }

    pub unsafe fn host_crypto_ripemd160(data_ptr: *const u8, data_len: u32, output_ptr: *mut u8) {
        let data = core::slice::from_raw_parts(data_ptr, data_len as usize);
        let digest = Ripemd160::digest(data);
        core::ptr::copy_nonoverlapping(digest.as_ptr(), output_ptr, 20);
    }

    pub unsafe fn host_crypto_ed25519_verify(
        msg_ptr: *const u8,
        msg_len: u32,
        sig_ptr: *const u8,
        pubkey_ptr: *const u8,
    ) -> i32 {
        let msg = core::slice::from_raw_parts(msg_ptr, msg_len as usize);
        let sig_bytes = core::slice::from_raw_parts(sig_ptr, 64);
        let pubkey_bytes = core::slice::from_raw_parts(pubkey_ptr, 32);
        let mut pubkey_arr = [0u8; 32];
        pubkey_arr.copy_from_slice(pubkey_bytes);
        let verifying_key = match Ed25519VerifyingKey::from_bytes(&pubkey_arr) {
            Ok(k) => k,
            Err(_) => return 0,
        };
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(sig_bytes);
        let signature = Ed25519Signature::from_bytes(&sig_arr);
        match verifying_key.verify(msg, &signature) {
            Ok(()) => 1,
            Err(_) => 0,
        }
    }

    pub unsafe fn host_crypto_secp256k1_verify(
        msg_ptr: *const u8,
        msg_len: u32,
        sig_ptr: *const u8,
        pubkey_ptr: *const u8,
    ) -> i32 {
        // The secp256k1 verify contract takes the canonical 32-byte
        // message digest the caller has already produced (matches the
        // wasm host contract — host_crypto_secp256k1_verify hashes are
        // the caller's responsibility). When msg_len != 32 we fail
        // closed rather than silently rehashing.
        if msg_len != 32 {
            return 0;
        }
        let msg = core::slice::from_raw_parts(msg_ptr, 32);
        let sig_bytes = core::slice::from_raw_parts(sig_ptr, 64);
        let pubkey_bytes = core::slice::from_raw_parts(pubkey_ptr, 33);

        let signature = match K256Signature::from_slice(sig_bytes) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        let verifying_key = match K256VerifyingKey::from_sec1_bytes(pubkey_bytes) {
            Ok(k) => k,
            Err(_) => return 0,
        };
        match verifying_key.verify_prehash(msg, &signature) {
            Ok(()) => 1,
            Err(_) => 0,
        }
    }

    pub unsafe fn host_crypto_secp256k1_recover(
        msg_ptr: *const u8,
        msg_len: u32,
        sig_ptr: *const u8,
        recovery_id: u8,
        output_ptr: *mut u8,
    ) -> i32 {
        // Like verify, the caller supplies the pre-hashed 32-byte
        // digest. Output is the canonical 65-byte uncompressed SEC1
        // public key (0x04 || X || Y).
        if msg_len != 32 {
            return -1;
        }
        let msg = core::slice::from_raw_parts(msg_ptr, 32);
        let sig_bytes = core::slice::from_raw_parts(sig_ptr, 64);

        let signature = match K256Signature::from_slice(sig_bytes) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let recid = match RecoveryId::try_from(recovery_id) {
            Ok(r) => r,
            Err(_) => return -1,
        };
        let recovered = match K256VerifyingKey::recover_from_prehash(msg, &signature, recid) {
            Ok(k) => k,
            Err(_) => return -1,
        };
        let encoded = recovered.to_encoded_point(false);
        let bytes = encoded.as_bytes();
        if bytes.len() != 65 {
            return -1;
        }
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), output_ptr, 65);
        65
    }

    pub unsafe fn host_crypto_bls12_381_verify(
        _msg_ptr: *const u8,
        _msg_len: u32,
        _sig_ptr: *const u8,
        _pubkey_ptr: *const u8,
    ) -> i32 {
        // P0-003 documented error path: the std/test mock host does
        // NOT verify BLS12-381 signatures. Returning 0 (invalid) is
        // the fail-closed posture every other host crypto mock now
        // follows on bad input — the test surface refuses to produce
        // an unconditional "valid" verdict. wasm32 builds reach a
        // real BLS12-381 verifier through the host import.
        0
    }

    pub unsafe fn host_call_contract(
        _address_ptr: *const u8,
        _address_len: u32,
        _input_ptr: *const u8,
        _input_len: u32,
        _output_ptr: *mut u8,
    ) -> i32 {
        -1 // Not implemented in mock
    }

    pub unsafe fn host_delegate_call(
        _address_ptr: *const u8,
        _address_len: u32,
        _input_ptr: *const u8,
        _input_len: u32,
        _output_ptr: *mut u8,
    ) -> i32 {
        -1
    }

    pub unsafe fn host_log(msg_ptr: *const u8, msg_len: u32) {
        #[cfg(feature = "std")]
        {
            let msg = core::slice::from_raw_parts(msg_ptr, msg_len as usize);
            if let Ok(s) = core::str::from_utf8(msg) {
                println!("[CONTRACT LOG] {}", s);
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (msg_ptr, msg_len);
        }
    }

    pub unsafe fn host_revert(msg_ptr: *const u8, msg_len: u32) -> ! {
        #[cfg(feature = "std")]
        {
            let msg = core::slice::from_raw_parts(msg_ptr, msg_len as usize);
            if let Ok(s) = core::str::from_utf8(msg) {
                panic!("Contract reverted: {}", s);
            } else {
                panic!("Contract reverted");
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (msg_ptr, msg_len);
            loop {}
        }
    }

    pub unsafe fn host_assert(condition: i32, msg_ptr: *const u8, msg_len: u32) {
        if condition == 0 {
            host_revert(msg_ptr, msg_len);
        }
    }

    // Governance host function mocks
    pub unsafe fn host_governance_submit_intent(_: *const u8, _: u32, _: *mut u8) -> i32 {
        -1
    }
    pub unsafe fn host_governance_get_intent_status(_: *const u8, _: u32, _: *mut u8) -> i32 {
        -1
    }
    pub unsafe fn host_governance_create_object(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *mut u8,
    ) -> i32 {
        -1
    }
    pub unsafe fn host_governance_get_object(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *mut u8,
    ) -> i32 {
        -1
    }
    pub unsafe fn host_governance_transition_object(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
    ) -> i32 {
        -1
    }
    pub unsafe fn host_governance_require_approval(_: *const u8, _: u32, _: u32) -> i32 {
        -1
    }
    pub unsafe fn host_governance_check_approval(_: *const u8, _: u32) -> i32 {
        -1
    }
    pub unsafe fn host_governance_get_trust_profile(_: *const u8, _: u32, _: *mut u8) -> i32 {
        -1
    }
    pub unsafe fn host_governance_evaluate_trust(_: *const u8, _: u32, _: *mut u8) -> i32 {
        -1
    }
    pub unsafe fn host_governance_has_capability(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
    ) -> i32 {
        0
    }
    pub unsafe fn host_governance_grant_capability(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *mut u8,
    ) -> i32 {
        -1
    }
    pub unsafe fn host_governance_revoke_capability(_: *const u8, _: u32) -> i32 {
        -1
    }
    pub unsafe fn host_governance_has_role(_: *const u8, _: u32, _: *const u8, _: u32) -> i32 {
        0
    }
    pub unsafe fn host_governance_assign_role(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *mut u8,
    ) -> i32 {
        -1
    }
    pub unsafe fn host_governance_get_evidence(_: *const u8, _: u32, _: *mut u8) -> i32 {
        -1
    }
    pub unsafe fn host_governance_evaluate_policy(
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *const u8,
        _: u32,
        _: *mut u8,
    ) -> i32 {
        -1
    }
}

// =============================================================================
// Storage Module
// =============================================================================

/// Storage operations for contract state
pub mod storage {
    use super::*;

    /// Maximum storage value size
    pub const MAX_VALUE_SIZE: usize = 65536;

    /// Get a value from storage
    ///
    /// Returns `None` if the key doesn't exist.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let balance: Option<U256> = storage::get(b"balance");
    /// ```
    #[cfg(feature = "alloc")]
    pub fn get(key: &[u8]) -> Option<alloc::vec::Vec<u8>> {
        let mut buffer = alloc::vec![0u8; MAX_VALUE_SIZE];
        let result =
            unsafe { host::host_storage_get(key.as_ptr(), key.len() as u32, buffer.as_mut_ptr()) };

        if result < 0 {
            None
        } else {
            buffer.truncate(result as usize);
            Some(buffer)
        }
    }

    /// Get a value from storage into a fixed-size buffer
    ///
    /// Returns the number of bytes read, or `None` if the key doesn't exist.
    pub fn get_into(key: &[u8], buffer: &mut [u8]) -> Option<usize> {
        let result =
            unsafe { host::host_storage_get(key.as_ptr(), key.len() as u32, buffer.as_mut_ptr()) };

        if result < 0 {
            None
        } else {
            Some(result as usize)
        }
    }

    /// Get and decode a value from storage
    pub fn get_decoded<T: Decode>(key: &[u8]) -> Option<T> {
        let mut buffer = [0u8; MAX_VALUE_SIZE];
        let result =
            unsafe { host::host_storage_get(key.as_ptr(), key.len() as u32, buffer.as_mut_ptr()) };

        if result < 0 {
            None
        } else {
            T::decode(&buffer[..result as usize]).ok()
        }
    }

    /// Set a value in storage
    ///
    /// # Example
    ///
    /// ```ignore
    /// storage::set(b"balance", &U256::from(100).to_bytes());
    /// ```
    pub fn set(key: &[u8], value: &[u8]) {
        unsafe {
            host::host_storage_set(
                key.as_ptr(),
                key.len() as u32,
                value.as_ptr(),
                value.len() as u32,
            );
        }
    }

    /// Encode and set a value in storage
    pub fn set_encoded<T: Encode>(key: &[u8], value: &T) -> Result<(), Error> {
        let mut buffer = [0u8; MAX_VALUE_SIZE];
        let len = value.encode(&mut buffer)?;
        set(key, &buffer[..len]);
        Ok(())
    }

    /// Delete a value from storage
    pub fn delete(key: &[u8]) {
        unsafe {
            host::host_storage_delete(key.as_ptr(), key.len() as u32);
        }
    }

    /// Check if a key exists in storage
    pub fn has(key: &[u8]) -> bool {
        unsafe { host::host_storage_has(key.as_ptr(), key.len() as u32) != 0 }
    }

    /// Storage map helper for key-value mappings
    pub struct StorageMap<'a> {
        prefix: &'a [u8],
    }

    impl<'a> StorageMap<'a> {
        /// Create a new storage map with a prefix
        pub const fn new(prefix: &'a [u8]) -> Self {
            Self { prefix }
        }

        /// Build a storage key from the prefix and a key
        fn build_key(&self, key: &[u8], buffer: &mut [u8]) -> usize {
            let prefix_len = self.prefix.len();
            buffer[..prefix_len].copy_from_slice(self.prefix);
            buffer[prefix_len..prefix_len + key.len()].copy_from_slice(key);
            prefix_len + key.len()
        }

        /// Get a value from the map
        pub fn get(&self, key: &[u8]) -> Option<usize> {
            let mut key_buffer = [0u8; 512];
            let key_len = self.build_key(key, &mut key_buffer);

            let mut value_buffer = [0u8; MAX_VALUE_SIZE];
            get_into(&key_buffer[..key_len], &mut value_buffer)
        }

        /// Get and decode a value from the map
        pub fn get_decoded<T: Decode>(&self, key: &[u8]) -> Option<T> {
            let mut key_buffer = [0u8; 512];
            let key_len = self.build_key(key, &mut key_buffer);
            get_decoded(&key_buffer[..key_len])
        }

        /// Set a value in the map
        pub fn set(&self, key: &[u8], value: &[u8]) {
            let mut key_buffer = [0u8; 512];
            let key_len = self.build_key(key, &mut key_buffer);
            set(&key_buffer[..key_len], value);
        }

        /// Encode and set a value in the map
        pub fn set_encoded<T: Encode>(&self, key: &[u8], value: &T) -> Result<(), Error> {
            let mut key_buffer = [0u8; 512];
            let key_len = self.build_key(key, &mut key_buffer);
            set_encoded(&key_buffer[..key_len], value)
        }

        /// Delete a value from the map
        pub fn delete(&self, key: &[u8]) {
            let mut key_buffer = [0u8; 512];
            let key_len = self.build_key(key, &mut key_buffer);
            delete(&key_buffer[..key_len]);
        }

        /// Check if a key exists in the map
        pub fn has(&self, key: &[u8]) -> bool {
            let mut key_buffer = [0u8; 512];
            let key_len = self.build_key(key, &mut key_buffer);
            has(&key_buffer[..key_len])
        }
    }
}

// =============================================================================
// Environment Module
// =============================================================================

/// Environment and execution context
pub mod env {
    use super::*;

    /// Get the caller's address
    ///
    /// Returns the address of the account that initiated the current call.
    /// For external calls, this is the transaction signer.
    /// For internal calls, this is the calling contract.
    pub fn caller() -> Address {
        let mut buffer = [0u8; 256];
        let len = unsafe { host::host_env_caller(buffer.as_mut_ptr()) };
        Address::from_bytes(&buffer[..len as usize]).unwrap_or_default()
    }

    /// Get this contract's address
    pub fn self_address() -> Address {
        let mut buffer = [0u8; 256];
        let len = unsafe { host::host_env_self_address(buffer.as_mut_ptr()) };
        Address::from_bytes(&buffer[..len as usize]).unwrap_or_default()
    }

    /// Get the contract owner's address
    pub fn owner() -> Address {
        let mut buffer = [0u8; 256];
        let len = unsafe { host::host_env_owner(buffer.as_mut_ptr()) };
        Address::from_bytes(&buffer[..len as usize]).unwrap_or_default()
    }

    /// Get the current block height
    pub fn block_height() -> u64 {
        unsafe { host::host_env_block_height() }
    }

    /// Get the current block timestamp (Unix seconds)
    pub fn block_time() -> u64 {
        unsafe { host::host_env_block_time() }
    }

    /// Get the value (tokens) sent with the call
    pub fn value() -> U256 {
        let mut buffer = [0u8; 32];
        unsafe { host::host_env_value(buffer.as_mut_ptr()) };
        U256::from_be_bytes(&buffer)
    }

    /// Get the remaining gas
    pub fn gas_remaining() -> u64 {
        unsafe { host::host_env_gas_remaining() }
    }

    /// Get the transaction hash
    pub fn tx_hash() -> Hash {
        let mut buffer = [0u8; 32];
        unsafe { host::host_env_tx_hash(buffer.as_mut_ptr()) };
        Hash(buffer)
    }

    /// Get the full execution context
    pub fn context() -> Context {
        Context {
            caller: caller(),
            block_height: block_height(),
            block_time: block_time(),
            tx_hash: tx_hash(),
            value: value(),
            gas_limit: gas_remaining(),
        }
    }

    /// Log a message (for debugging)
    pub fn log(msg: &str) {
        unsafe {
            host::host_log(msg.as_ptr(), msg.len() as u32);
        }
    }

    /// Revert the transaction with a message
    pub fn revert(msg: &str) -> ! {
        unsafe {
            host::host_revert(msg.as_ptr(), msg.len() as u32);
        }
    }

    /// Assert a condition, reverting if false
    pub fn assert(condition: bool, msg: &str) {
        unsafe {
            host::host_assert(condition as i32, msg.as_ptr(), msg.len() as u32);
        }
    }

    /// Require a condition, returning an error if false
    pub fn require(condition: bool, error: Error) -> Result<(), Error> {
        if condition {
            Ok(())
        } else {
            Err(error)
        }
    }
}

// =============================================================================
// L0 Module (Accumulate Layer 0 Operations)
// =============================================================================

/// Accumulate L0 (Layer 0) operations
pub mod l0 {
    use super::*;

    /// Account types for L0 accounts
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AccountType {
        /// ADI (Accumulate Digital Identifier)
        ADI = 0,
        /// Token Account
        TokenAccount = 1,
        /// Lite Token Account
        LiteTokenAccount = 2,
        /// Data Account
        DataAccount = 3,
        /// Key Book
        KeyBook = 4,
        /// Key Page
        KeyPage = 5,
    }

    /// L0 Account information
    #[derive(Clone, Debug)]
    pub struct L0Account {
        pub url: Address,
        pub account_type: AccountType,
        pub balance: U256,
        pub credit_balance: u64,
        pub data_entry_count: u64,
    }

    /// Get information about an L0 account
    pub fn get_account(url: &Address) -> Option<L0Account> {
        let url_bytes = url.as_bytes();
        let mut buffer = [0u8; 1024];

        let result = unsafe {
            host::host_l0_get_account(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                buffer.as_mut_ptr(),
            )
        };

        if result < 0 {
            return None;
        }

        // Decode account info
        let account_type = match buffer[0] {
            0 => AccountType::ADI,
            1 => AccountType::TokenAccount,
            2 => AccountType::LiteTokenAccount,
            3 => AccountType::DataAccount,
            4 => AccountType::KeyBook,
            5 => AccountType::KeyPage,
            _ => return None,
        };

        let balance = U256::from_be_bytes(buffer[1..33].try_into().unwrap_or(&[0u8; 32]));
        let credit_balance = u64::from_be_bytes([
            buffer[33], buffer[34], buffer[35], buffer[36], buffer[37], buffer[38], buffer[39],
            buffer[40],
        ]);
        let data_entry_count = u64::from_be_bytes([
            buffer[41], buffer[42], buffer[43], buffer[44], buffer[45], buffer[46], buffer[47],
            buffer[48],
        ]);

        Some(L0Account {
            url: url.clone(),
            account_type,
            balance,
            credit_balance,
            data_entry_count,
        })
    }

    /// Get the token balance of an L0 account
    pub fn get_balance(url: &Address) -> U256 {
        let url_bytes = url.as_bytes();
        let mut buffer = [0u8; 32];

        let result = unsafe {
            host::host_l0_get_balance(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                buffer.as_mut_ptr(),
            )
        };

        if result < 0 {
            U256::ZERO
        } else {
            U256::from_be_bytes(&buffer)
        }
    }

    /// Get data from an L0 Data Account
    #[cfg(feature = "alloc")]
    pub fn get_data(url: &Address, entry_hash: &Hash) -> Option<alloc::vec::Vec<u8>> {
        let url_bytes = url.as_bytes();
        let mut buffer = alloc::vec![0u8; 65536];

        let result = unsafe {
            host::host_l0_get_data(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                entry_hash.0.as_ptr(),
                buffer.as_mut_ptr(),
            )
        };

        if result < 0 {
            None
        } else {
            buffer.truncate(result as usize);
            Some(buffer)
        }
    }

    /// Get data from an L0 Data Account into a buffer
    pub fn get_data_into(url: &Address, entry_hash: &Hash, buffer: &mut [u8]) -> Option<usize> {
        let url_bytes = url.as_bytes();

        let result = unsafe {
            host::host_l0_get_data(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                entry_hash.0.as_ptr(),
                buffer.as_mut_ptr(),
            )
        };

        if result < 0 {
            None
        } else {
            Some(result as usize)
        }
    }

    /// Create an L0 account
    ///
    /// The contract must have authority over the parent ADI to create sub-accounts.
    pub fn create_account(url: &Address, account_type: AccountType) -> Result<(), Error> {
        let url_bytes = url.as_bytes();

        let result = unsafe {
            host::host_l0_create_account(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                account_type as u8,
            )
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            Ok(())
        }
    }

    /// Write data to an L0 Data Account
    ///
    /// Returns the entry hash on success.
    pub fn write_data(url: &Address, data: &[u8]) -> Result<Hash, Error> {
        let url_bytes = url.as_bytes();

        let result = unsafe {
            host::host_l0_write_data(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                data.as_ptr(),
                data.len() as u32,
            )
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            // Compute entry hash (SHA-256 of data)
            Ok(super::crypto::sha256(data))
        }
    }

    /// Transfer tokens between L0 accounts
    ///
    /// The contract must have authority over the source account.
    pub fn transfer(from: &Address, to: &Address, amount: &U256) -> Result<(), Error> {
        let from_bytes = from.as_bytes();
        let to_bytes = to.as_bytes();
        let amount_bytes = amount.to_be_bytes();

        let result = unsafe {
            host::host_l0_transfer(
                from_bytes.as_ptr(),
                from_bytes.len() as u32,
                to_bytes.as_ptr(),
                to_bytes.len() as u32,
                amount_bytes.as_ptr(),
            )
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            Ok(())
        }
    }

    /// Burn credits from an L0 account
    ///
    /// Credits are used to pay for L0 operations.
    pub fn burn_credits(url: &Address, amount: u64) -> Result<(), Error> {
        let url_bytes = url.as_bytes();

        let result = unsafe {
            host::host_l0_burn_credits(url_bytes.as_ptr(), url_bytes.len() as u32, amount)
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            Ok(())
        }
    }

    /// Authority information for an L0 account
    #[derive(Clone, Debug)]
    pub struct Authority {
        pub key_book: Address,
        pub threshold: u64,
        pub signers: u64,
    }

    /// Get authority information for an L0 account
    pub fn get_authority(url: &Address) -> Option<Authority> {
        let url_bytes = url.as_bytes();
        let mut buffer = [0u8; 512];

        let result = unsafe {
            host::host_l0_get_authority(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                buffer.as_mut_ptr(),
            )
        };

        if result < 0 {
            return None;
        }

        let key_book_len = buffer[0] as usize;
        let key_book = Address::from_bytes(&buffer[1..1 + key_book_len])?;

        let offset = 1 + key_book_len;
        let threshold = u64::from_be_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
            buffer[offset + 4],
            buffer[offset + 5],
            buffer[offset + 6],
            buffer[offset + 7],
        ]);
        let signers = u64::from_be_bytes([
            buffer[offset + 8],
            buffer[offset + 9],
            buffer[offset + 10],
            buffer[offset + 11],
            buffer[offset + 12],
            buffer[offset + 13],
            buffer[offset + 14],
            buffer[offset + 15],
        ]);

        Some(Authority {
            key_book,
            threshold,
            signers,
        })
    }

    /// Check if a signer has authority over an L0 account
    pub fn check_authority(url: &Address, signer: &Address) -> bool {
        let url_bytes = url.as_bytes();
        let signer_bytes = signer.as_bytes();

        let result = unsafe {
            host::host_l0_check_authority(
                url_bytes.as_ptr(),
                url_bytes.len() as u32,
                signer_bytes.as_ptr(),
                signer_bytes.len() as u32,
            )
        };

        result != 0
    }
}

// =============================================================================
// Events Module
// =============================================================================

/// Event emission for contract logging
pub mod events {
    use super::*;

    /// Emit an event with topics and data
    ///
    /// Topics are indexed for efficient filtering.
    /// Data is the unindexed event payload.
    pub fn emit(topics: &[Topic], data: &[u8]) {
        // Serialize topics
        let mut topic_buffer = [0u8; 128]; // 4 topics * 32 bytes
        let topic_len = topics.len().min(4) * 32;

        for (i, topic) in topics.iter().take(4).enumerate() {
            topic_buffer[i * 32..(i + 1) * 32].copy_from_slice(&topic.0);
        }

        unsafe {
            host::host_event_emit(
                topic_buffer.as_ptr(),
                topic_len as u32,
                data.as_ptr(),
                data.len() as u32,
            );
        }
    }

    /// Emit a simple event with just a topic signature
    pub fn emit_simple(signature: u32) {
        let topic = Topic::from_u32(signature);
        emit(&[topic], &[]);
    }

    /// Emit an event with one indexed value
    pub fn emit_indexed1(signature: u32, indexed: &[u8; 32], data: &[u8]) {
        let topics = [Topic::from_u32(signature), Topic(*indexed)];
        emit(&topics, data);
    }

    /// Emit an event with two indexed values
    pub fn emit_indexed2(signature: u32, indexed1: &[u8; 32], indexed2: &[u8; 32], data: &[u8]) {
        let topics = [
            Topic::from_u32(signature),
            Topic(*indexed1),
            Topic(*indexed2),
        ];
        emit(&topics, data);
    }

    /// Emit an event with three indexed values (maximum)
    pub fn emit_indexed3(
        signature: u32,
        indexed1: &[u8; 32],
        indexed2: &[u8; 32],
        indexed3: &[u8; 32],
        data: &[u8],
    ) {
        let topics = [
            Topic::from_u32(signature),
            Topic(*indexed1),
            Topic(*indexed2),
            Topic(*indexed3),
        ];
        emit(&topics, data);
    }
}

// =============================================================================
// Crypto Module
// =============================================================================

/// Cryptographic operations
pub mod crypto {
    use super::*;

    /// Compute SHA-256 hash
    pub fn sha256(data: &[u8]) -> Hash {
        let mut output = [0u8; 32];
        unsafe {
            host::host_crypto_sha256(data.as_ptr(), data.len() as u32, output.as_mut_ptr());
        }
        Hash(output)
    }

    /// Compute SHA3-256 (Keccak) hash
    pub fn sha3_256(data: &[u8]) -> Hash {
        let mut output = [0u8; 32];
        unsafe {
            host::host_crypto_sha3_256(data.as_ptr(), data.len() as u32, output.as_mut_ptr());
        }
        Hash(output)
    }

    /// Compute Keccak-256 hash (Ethereum compatible)
    pub fn keccak256(data: &[u8]) -> Hash {
        let mut output = [0u8; 32];
        unsafe {
            host::host_crypto_keccak256(data.as_ptr(), data.len() as u32, output.as_mut_ptr());
        }
        Hash(output)
    }

    /// Compute Blake2b-256 hash
    pub fn blake2b_256(data: &[u8]) -> Hash {
        let mut output = [0u8; 32];
        unsafe {
            host::host_crypto_blake2b_256(data.as_ptr(), data.len() as u32, output.as_mut_ptr());
        }
        Hash(output)
    }

    /// Compute RIPEMD-160 hash (returns 20 bytes)
    pub fn ripemd160(data: &[u8]) -> [u8; 20] {
        let mut output = [0u8; 20];
        unsafe {
            host::host_crypto_ripemd160(data.as_ptr(), data.len() as u32, output.as_mut_ptr());
        }
        output
    }

    /// Verify an Ed25519 signature
    pub fn ed25519_verify(message: &[u8], signature: &[u8; 64], public_key: &[u8; 32]) -> bool {
        let result = unsafe {
            host::host_crypto_ed25519_verify(
                message.as_ptr(),
                message.len() as u32,
                signature.as_ptr(),
                public_key.as_ptr(),
            )
        };
        result != 0
    }

    /// Verify a secp256k1 signature
    pub fn secp256k1_verify(message: &[u8], signature: &[u8; 64], public_key: &[u8; 33]) -> bool {
        let result = unsafe {
            host::host_crypto_secp256k1_verify(
                message.as_ptr(),
                message.len() as u32,
                signature.as_ptr(),
                public_key.as_ptr(),
            )
        };
        result != 0
    }

    /// Recover a secp256k1 public key from a signature
    pub fn secp256k1_recover(
        message: &[u8],
        signature: &[u8; 64],
        recovery_id: u8,
    ) -> Option<[u8; 65]> {
        let mut output = [0u8; 65];
        let result = unsafe {
            host::host_crypto_secp256k1_recover(
                message.as_ptr(),
                message.len() as u32,
                signature.as_ptr(),
                recovery_id,
                output.as_mut_ptr(),
            )
        };

        if result < 0 {
            None
        } else {
            Some(output)
        }
    }

    /// Verify a BLS12-381 signature
    pub fn bls12_381_verify(message: &[u8], signature: &[u8; 96], public_key: &[u8; 48]) -> bool {
        let result = unsafe {
            host::host_crypto_bls12_381_verify(
                message.as_ptr(),
                message.len() as u32,
                signature.as_ptr(),
                public_key.as_ptr(),
            )
        };
        result != 0
    }

    /// Hash an address to a 32-byte value (for event indexing)
    pub fn hash_address(address: &Address) -> [u8; 32] {
        sha256(address.as_bytes()).0
    }

    /// Hash a U256 to a 32-byte value (for event indexing)
    pub fn hash_u256(value: &U256) -> [u8; 32] {
        value.to_be_bytes()
    }
}

// =============================================================================
// Cross-Contract Calls Module
// =============================================================================

/// Cross-contract call functionality
pub mod calls {
    use super::*;

    /// Call another contract
    ///
    /// Executes a function on another contract and returns the result.
    #[cfg(feature = "alloc")]
    pub fn call(address: &Address, input: &[u8]) -> Result<alloc::vec::Vec<u8>, Error> {
        let address_bytes = address.as_bytes();
        let mut output = alloc::vec![0u8; 65536];

        let result = unsafe {
            host::host_call_contract(
                address_bytes.as_ptr(),
                address_bytes.len() as u32,
                input.as_ptr(),
                input.len() as u32,
                output.as_mut_ptr(),
            )
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            output.truncate(result as usize);
            Ok(output)
        }
    }

    /// Call another contract into a buffer
    pub fn call_into(address: &Address, input: &[u8], output: &mut [u8]) -> Result<usize, Error> {
        let address_bytes = address.as_bytes();

        let result = unsafe {
            host::host_call_contract(
                address_bytes.as_ptr(),
                address_bytes.len() as u32,
                input.as_ptr(),
                input.len() as u32,
                output.as_mut_ptr(),
            )
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            Ok(result as usize)
        }
    }

    /// Delegate call to another contract
    ///
    /// Executes code from another contract in the context of this contract.
    /// Storage and caller remain the same as the calling contract.
    #[cfg(feature = "alloc")]
    pub fn delegate_call(address: &Address, input: &[u8]) -> Result<alloc::vec::Vec<u8>, Error> {
        let address_bytes = address.as_bytes();
        let mut output = alloc::vec![0u8; 65536];

        let result = unsafe {
            host::host_delegate_call(
                address_bytes.as_ptr(),
                address_bytes.len() as u32,
                input.as_ptr(),
                input.len() as u32,
                output.as_mut_ptr(),
            )
        };

        if result < 0 {
            Err(Error::from_code((-result) as u32))
        } else {
            output.truncate(result as usize);
            Ok(output)
        }
    }

    /// Build a function call with selector and encoded arguments
    pub fn build_call(selector: u32, args: &[u8], buffer: &mut [u8]) -> usize {
        let selector_bytes = selector.to_be_bytes();
        buffer[..4].copy_from_slice(&selector_bytes);
        buffer[4..4 + args.len()].copy_from_slice(args);
        4 + args.len()
    }
}

// =============================================================================
// Token Standards Module
// =============================================================================

/// Standard token interfaces
pub mod tokens {
    use super::*;

    /// ACU-20 (Fungible Token) function selectors
    pub mod acu20 {
        use super::*;

        /// Function selector for `name()`
        pub const NAME_SELECTOR: u32 = 0x06fdde03;
        /// Function selector for `symbol()`
        pub const SYMBOL_SELECTOR: u32 = 0x95d89b41;
        /// Function selector for `decimals()`
        pub const DECIMALS_SELECTOR: u32 = 0x313ce567;
        /// Function selector for `totalSupply()`
        pub const TOTAL_SUPPLY_SELECTOR: u32 = 0x18160ddd;
        /// Function selector for `balanceOf(address)`
        pub const BALANCE_OF_SELECTOR: u32 = 0x70a08231;
        /// Function selector for `transfer(address,uint256)`
        pub const TRANSFER_SELECTOR: u32 = 0xa9059cbb;
        /// Function selector for `approve(address,uint256)`
        pub const APPROVE_SELECTOR: u32 = 0x095ea7b3;
        /// Function selector for `allowance(address,address)`
        pub const ALLOWANCE_SELECTOR: u32 = 0xdd62ed3e;
        /// Function selector for `transferFrom(address,address,uint256)`
        pub const TRANSFER_FROM_SELECTOR: u32 = 0x23b872dd;

        /// Transfer event signature
        pub const TRANSFER_EVENT: u32 = 0xddf252ad;
        /// Approval event signature
        pub const APPROVAL_EVENT: u32 = 0x8c5be1e5;

        /// ACU-20 Token Interface
        pub trait Token {
            /// Returns the name of the token
            fn name(&self) -> &str;

            /// Returns the symbol of the token
            fn symbol(&self) -> &str;

            /// Returns the number of decimals
            fn decimals(&self) -> u8;

            /// Returns the total supply
            fn total_supply(&self) -> U256;

            /// Returns the balance of an account
            fn balance_of(&self, owner: &Address) -> U256;

            /// Transfers tokens to a recipient
            fn transfer(&mut self, to: &Address, amount: &U256) -> Result<bool, Error>;

            /// Approves a spender to transfer tokens
            fn approve(&mut self, spender: &Address, amount: &U256) -> Result<bool, Error>;

            /// Returns the allowance for a spender
            fn allowance(&self, owner: &Address, spender: &Address) -> U256;

            /// Transfers tokens from one account to another (requires approval)
            fn transfer_from(
                &mut self,
                from: &Address,
                to: &Address,
                amount: &U256,
            ) -> Result<bool, Error>;
        }
    }

    /// ACU-721 (NFT) function selectors
    pub mod acu721 {
        use super::*;

        /// Function selector for `ownerOf(uint256)`
        pub const OWNER_OF_SELECTOR: u32 = 0x6352211e;
        /// Function selector for `balanceOf(address)`
        pub const BALANCE_OF_SELECTOR: u32 = 0x70a08231;
        /// Function selector for `approve(address,uint256)`
        pub const APPROVE_SELECTOR: u32 = 0x095ea7b3;
        /// Function selector for `getApproved(uint256)`
        pub const GET_APPROVED_SELECTOR: u32 = 0x081812fc;
        /// Function selector for `setApprovalForAll(address,bool)`
        pub const SET_APPROVAL_FOR_ALL_SELECTOR: u32 = 0xa22cb465;
        /// Function selector for `isApprovedForAll(address,address)`
        pub const IS_APPROVED_FOR_ALL_SELECTOR: u32 = 0xe985e9c5;
        /// Function selector for `transferFrom(address,address,uint256)`
        pub const TRANSFER_FROM_SELECTOR: u32 = 0x23b872dd;
        /// Function selector for `safeTransferFrom(address,address,uint256)`
        pub const SAFE_TRANSFER_FROM_SELECTOR: u32 = 0x42842e0e;
        /// Function selector for `tokenURI(uint256)`
        pub const TOKEN_URI_SELECTOR: u32 = 0xc87b56dd;

        /// Transfer event signature
        pub const TRANSFER_EVENT: u32 = 0xddf252ad;
        /// Approval event signature
        pub const APPROVAL_EVENT: u32 = 0x8c5be1e5;
        /// ApprovalForAll event signature
        pub const APPROVAL_FOR_ALL_EVENT: u32 = 0x17307eab;

        /// ACU-721 NFT Interface
        pub trait NFT {
            /// Returns the owner of a token
            fn owner_of(&self, token_id: &U256) -> Option<Address>;

            /// Returns the number of tokens owned by an address
            fn balance_of(&self, owner: &Address) -> U256;

            /// Approves an address to transfer a specific token
            fn approve(&mut self, to: &Address, token_id: &U256) -> Result<(), Error>;

            /// Returns the approved address for a token
            fn get_approved(&self, token_id: &U256) -> Option<Address>;

            /// Sets approval for all tokens owned by the caller
            fn set_approval_for_all(
                &mut self,
                operator: &Address,
                approved: bool,
            ) -> Result<(), Error>;

            /// Checks if an operator is approved for all tokens of an owner
            fn is_approved_for_all(&self, owner: &Address, operator: &Address) -> bool;

            /// Transfers a token from one address to another
            fn transfer_from(
                &mut self,
                from: &Address,
                to: &Address,
                token_id: &U256,
            ) -> Result<(), Error>;

            /// Returns the token URI for metadata
            fn token_uri(&self, token_id: &U256) -> Option<&str>;
        }
    }

    /// ACU-1155 (Multi-Token) function selectors
    pub mod acu1155 {
        use super::*;

        /// Function selector for `balanceOf(address,uint256)`
        pub const BALANCE_OF_SELECTOR: u32 = 0x00fdd58e;
        /// Function selector for `balanceOfBatch(address[],uint256[])`
        pub const BALANCE_OF_BATCH_SELECTOR: u32 = 0x4e1273f4;
        /// Function selector for `setApprovalForAll(address,bool)`
        pub const SET_APPROVAL_FOR_ALL_SELECTOR: u32 = 0xa22cb465;
        /// Function selector for `isApprovedForAll(address,address)`
        pub const IS_APPROVED_FOR_ALL_SELECTOR: u32 = 0xe985e9c5;
        /// Function selector for `safeTransferFrom(address,address,uint256,uint256,bytes)`
        pub const SAFE_TRANSFER_FROM_SELECTOR: u32 = 0xf242432a;
        /// Function selector for `safeBatchTransferFrom(address,address,uint256[],uint256[],bytes)`
        pub const SAFE_BATCH_TRANSFER_FROM_SELECTOR: u32 = 0x2eb2c2d6;
        /// Function selector for `uri(uint256)`
        pub const URI_SELECTOR: u32 = 0x0e89341c;

        /// TransferSingle event signature
        pub const TRANSFER_SINGLE_EVENT: u32 = 0xc3d58168;
        /// TransferBatch event signature
        pub const TRANSFER_BATCH_EVENT: u32 = 0x4a39dc06;
        /// ApprovalForAll event signature
        pub const APPROVAL_FOR_ALL_EVENT: u32 = 0x17307eab;
        /// URI event signature
        pub const URI_EVENT: u32 = 0x6bb7ff70;

        /// ACU-1155 Multi-Token Interface
        pub trait MultiToken {
            /// Returns the balance of a specific token for an owner
            fn balance_of(&self, owner: &Address, token_id: &U256) -> U256;

            /// Sets approval for all tokens
            fn set_approval_for_all(
                &mut self,
                operator: &Address,
                approved: bool,
            ) -> Result<(), Error>;

            /// Checks if an operator is approved for all tokens
            fn is_approved_for_all(&self, owner: &Address, operator: &Address) -> bool;

            /// Transfers a single token type
            fn safe_transfer_from(
                &mut self,
                from: &Address,
                to: &Address,
                token_id: &U256,
                amount: &U256,
                data: &[u8],
            ) -> Result<(), Error>;

            /// Returns the URI for a token type
            fn uri(&self, token_id: &U256) -> Option<&str>;
        }
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Utility functions for contract development
pub mod utils {
    use super::*;

    /// Calculate a storage slot for a mapping
    ///
    /// Uses keccak256(key || slot) similar to Solidity storage layout.
    pub fn mapping_slot(slot: &[u8; 32], key: &[u8]) -> Hash {
        let mut data = [0u8; 64];
        data[..key.len()].copy_from_slice(key);
        data[32..64].copy_from_slice(slot);
        crypto::keccak256(&data[..32 + key.len()])
    }

    /// Calculate a storage slot for a nested mapping
    ///
    /// Uses keccak256(key2 || keccak256(key1 || slot))
    pub fn nested_mapping_slot(slot: &[u8; 32], key1: &[u8], key2: &[u8]) -> Hash {
        let inner = mapping_slot(slot, key1);
        mapping_slot(&inner.0, key2)
    }

    /// Safe math operations that revert on overflow
    pub mod safe_math {
        use super::*;

        /// Add two U256 values, reverting on overflow
        pub fn add(a: &U256, b: &U256) -> Result<U256, Error> {
            a.checked_add(b).ok_or(Error::Overflow)
        }

        /// Subtract two U256 values, reverting on underflow
        pub fn sub(a: &U256, b: &U256) -> Result<U256, Error> {
            a.checked_sub(b).ok_or(Error::Underflow)
        }

        /// Multiply two U256 values, reverting on overflow
        pub fn mul(a: &U256, b: &U256) -> Result<U256, Error> {
            a.checked_mul(b).ok_or(Error::Overflow)
        }

        /// Divide two U256 values, reverting on division by zero
        pub fn div(a: &U256, b: &U256) -> Result<U256, Error> {
            if *b == U256::ZERO {
                return Err(Error::DivisionByZero);
            }
            a.checked_div(b).ok_or(Error::Overflow)
        }
    }

    /// Address validation utilities
    pub mod address {
        use super::*;

        /// Check if an address is the zero address
        pub fn is_zero(addr: &Address) -> bool {
            addr.as_bytes().iter().all(|&b| b == 0)
        }

        /// Require an address is not zero
        pub fn require_non_zero(addr: &Address) -> Result<(), Error> {
            if is_zero(addr) {
                Err(Error::ZeroAddress)
            } else {
                Ok(())
            }
        }
    }

    /// Reentrancy guard
    pub struct ReentrancyGuard {
        key: &'static [u8],
    }

    impl ReentrancyGuard {
        /// Create a new reentrancy guard
        pub const fn new(key: &'static [u8]) -> Self {
            Self { key }
        }

        /// Enter the guarded section
        pub fn enter(&self) -> Result<ReentrancyGuardLock<'_>, Error> {
            if storage::has(self.key) {
                return Err(Error::ReentrancyDetected);
            }
            storage::set(self.key, &[1]);
            Ok(ReentrancyGuardLock { guard: self })
        }
    }

    /// Lock that releases the reentrancy guard when dropped
    pub struct ReentrancyGuardLock<'a> {
        guard: &'a ReentrancyGuard,
    }

    impl<'a> Drop for ReentrancyGuardLock<'a> {
        fn drop(&mut self) {
            storage::delete(self.guard.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_operations() {
        let key = b"test_key";
        let value = b"test_value";

        storage::set(key, value);
        assert!(storage::has(key));

        let retrieved = storage::get(key).unwrap();
        assert_eq!(retrieved.as_slice(), value);

        storage::delete(key);
        assert!(!storage::has(key));
    }

    #[test]
    fn test_storage_map() {
        let map = storage::StorageMap::new(b"balances_");
        let key = b"user1";
        let value = U256::from(1000u64);

        map.set_encoded(key, &value).unwrap();
        assert!(map.has(key));

        let retrieved: U256 = map.get_decoded(key).unwrap();
        assert_eq!(retrieved, value);

        map.delete(key);
        assert!(!map.has(key));
    }

    #[test]
    fn test_env_functions() {
        // Test that env functions don't panic
        let _ = env::block_height();
        let _ = env::block_time();
        let _ = env::gas_remaining();
        let _ = env::value();
    }

    // P0-003 (2026-05-02): the std/test host crypto mock now computes
    // real digests. The known-answer vectors below are taken from the
    // canonical test suites for each algorithm:
    //
    //   - SHA-256 / SHA3-256: NIST CSRC examples for "abc" / "" / NIST
    //     short-message vectors.
    //   - Keccak-256: the original Keccak team's reference vectors
    //     (matches Ethereum's keccak256 — distinct from SHA3-256 by
    //     padding rule).
    //   - Blake2b-256: RFC 7693 short-message vectors.
    //   - RIPEMD-160: ISO/IEC 10118-3 reference vectors.
    //
    // A green run of these tests is the canonical "no host crypto
    // mock returns unconditional success or zero output" gate the
    // milestone's done-when condition demands.
    #[test]
    fn test_crypto_hash_sha256_known_answer() {
        // RFC 6234 §8.5 single-block "abc" → ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let want = hex::decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
            .unwrap();
        let got = crypto::sha256(b"abc");
        assert_eq!(
            got.0.to_vec(),
            want,
            "SHA-256(\"abc\") known-answer mismatch"
        );

        // FIPS 180-2 empty-string vector → e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let want_empty =
            hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .unwrap();
        let got_empty = crypto::sha256(b"");
        assert_eq!(
            got_empty.0.to_vec(),
            want_empty,
            "SHA-256(\"\") known-answer mismatch"
        );
    }

    #[test]
    fn test_crypto_hash_sha3_256_known_answer() {
        // NIST FIPS 202 SHA3-256("") → a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a
        let want_empty =
            hex::decode("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a")
                .unwrap();
        let got_empty = crypto::sha3_256(b"");
        assert_eq!(
            got_empty.0.to_vec(),
            want_empty,
            "SHA3-256(\"\") known-answer mismatch"
        );

        // SHA3-256("abc") → 3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532
        let want_abc =
            hex::decode("3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532")
                .unwrap();
        let got_abc = crypto::sha3_256(b"abc");
        assert_eq!(
            got_abc.0.to_vec(),
            want_abc,
            "SHA3-256(\"abc\") known-answer mismatch"
        );
    }

    #[test]
    fn test_crypto_hash_keccak256_known_answer() {
        // Ethereum keccak256("") → c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let want_empty =
            hex::decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
                .unwrap();
        let got_empty = crypto::keccak256(b"");
        assert_eq!(
            got_empty.0.to_vec(),
            want_empty,
            "Keccak-256(\"\") known-answer mismatch"
        );

        // Ethereum keccak256("abc") → 4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45
        let want_abc =
            hex::decode("4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45")
                .unwrap();
        let got_abc = crypto::keccak256(b"abc");
        assert_eq!(
            got_abc.0.to_vec(),
            want_abc,
            "Keccak-256(\"abc\") known-answer mismatch"
        );

        // Sanity: SHA3-256("abc") and Keccak-256("abc") differ (padding rule)
        assert_ne!(crypto::sha3_256(b"abc").0, crypto::keccak256(b"abc").0,
            "SHA3-256 and Keccak-256 must produce different digests for the same input (different padding)");
    }

    #[test]
    fn test_crypto_hash_blake2b_256_known_answer() {
        // Blake2b with 32-byte output (digest_length=32 is part of the
        // BLAKE2 personalization, so this is NOT Blake2b-512 truncated
        // to 32 bytes). The vectors below match the canonical
        // RustCrypto blake2 crate (blake2::Blake2b<U32>) which is the
        // industry-standard reference Rust implementation.
        //
        //   Blake2b-256("abc") =
        //     bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319
        //   Blake2b-256("")    =
        //     0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8
        let want_abc =
            hex::decode("bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319")
                .unwrap();
        let got_abc = crypto::blake2b_256(b"abc");
        assert_eq!(
            got_abc.0.to_vec(),
            want_abc,
            "Blake2b-256(\"abc\") known-answer mismatch"
        );

        let want_empty =
            hex::decode("0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8")
                .unwrap();
        let got_empty = crypto::blake2b_256(b"");
        assert_eq!(
            got_empty.0.to_vec(),
            want_empty,
            "Blake2b-256(\"\") known-answer mismatch"
        );
    }

    #[test]
    fn test_crypto_hash_ripemd160_known_answer() {
        // ISO/IEC 10118-3 — RIPEMD-160("abc") =
        // 8eb208f7e05d987a9b044a8e98c6b087f15a0bfc
        let want_abc = hex::decode("8eb208f7e05d987a9b044a8e98c6b087f15a0bfc").unwrap();
        let got_abc = crypto::ripemd160(b"abc");
        assert_eq!(
            got_abc.to_vec(),
            want_abc,
            "RIPEMD-160(\"abc\") known-answer mismatch"
        );

        // RIPEMD-160("") = 9c1185a5c5e9fc54612808977ee8f548b2258d31
        let want_empty = hex::decode("9c1185a5c5e9fc54612808977ee8f548b2258d31").unwrap();
        let got_empty = crypto::ripemd160(b"");
        assert_eq!(
            got_empty.to_vec(),
            want_empty,
            "RIPEMD-160(\"\") known-answer mismatch"
        );
    }

    #[test]
    fn test_crypto_ed25519_verify_positive_and_negative() {
        // RFC 8032 §7.1 test vector 1:
        //   secret: 9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60
        //   public: d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a
        //   message: ""
        //   signature:
        //     e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b
        use ed25519_dalek::{SecretKey, SigningKey};
        let secret_bytes: [u8; 32] =
            hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .unwrap()
                .try_into()
                .unwrap();
        let signing_key = SigningKey::from_bytes(&SecretKey::from(secret_bytes));
        let verifying_key = signing_key.verifying_key();

        let pub_bytes = verifying_key.to_bytes();
        assert_eq!(
            hex::encode(pub_bytes),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            "ed25519 RFC 8032 vector 1 public key mismatch"
        );

        let sig_hex = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
        let sig_bytes: [u8; 64] = hex::decode(sig_hex).unwrap().try_into().unwrap();

        // Positive — RFC 8032 vector verifies.
        assert!(
            crypto::ed25519_verify(b"", &sig_bytes, &pub_bytes),
            "ed25519_verify must accept the canonical RFC 8032 vector 1 signature"
        );

        // Negative — flipped first signature byte must fail.
        let mut tampered = sig_bytes;
        tampered[0] ^= 0x01;
        assert!(
            !crypto::ed25519_verify(b"", &tampered, &pub_bytes),
            "ed25519_verify must reject a tampered signature (P0-003 — no unconditional success)"
        );

        // Negative — wrong message must fail.
        assert!(
            !crypto::ed25519_verify(b"different message", &sig_bytes, &pub_bytes),
            "ed25519_verify must reject a different message under the same signature"
        );

        // Negative — flipped public-key byte must fail.
        let mut wrong_pub = pub_bytes;
        wrong_pub[31] ^= 0x01;
        assert!(
            !crypto::ed25519_verify(b"", &sig_bytes, &wrong_pub),
            "ed25519_verify must reject a tampered public key"
        );
    }

    #[test]
    fn test_crypto_secp256k1_verify_positive_and_negative_round_trip() {
        // Generate a fresh ECDSA keypair, sign a message digest, then
        // verify positive + tampered + wrong-message + wrong-pubkey
        // and exercise the recover round-trip.
        use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey, VerifyingKey};

        let mut rng = rand_core::OsRng;
        let signing_key = SigningKey::random(&mut rng);
        let verifying_key = signing_key.verifying_key();

        let msg = b"infrix-sdk p0-003 secp256k1 round trip";
        let digest = crypto::sha256(msg);

        let signature: Signature = signing_key.sign_prehash(&digest.0).unwrap();
        let sig_bytes: [u8; 64] = signature.to_bytes().into();

        let pub_bytes = verifying_key.to_sec1_bytes();
        assert_eq!(
            pub_bytes.len(),
            33,
            "secp256k1 verifying key must be 33-byte SEC1 compressed form"
        );
        let mut pub_arr = [0u8; 33];
        pub_arr.copy_from_slice(&pub_bytes);

        // Positive.
        assert!(
            crypto::secp256k1_verify(&digest.0, &sig_bytes, &pub_arr),
            "secp256k1_verify must accept a freshly-produced signature"
        );

        // Negative — flipped digest byte.
        let mut wrong_digest = digest.0;
        wrong_digest[0] ^= 0x01;
        assert!(
            !crypto::secp256k1_verify(&wrong_digest, &sig_bytes, &pub_arr),
            "secp256k1_verify must reject a different digest under the same signature"
        );

        // Negative — flipped signature byte.
        let mut tampered_sig = sig_bytes;
        tampered_sig[10] ^= 0x01;
        assert!(
            !crypto::secp256k1_verify(&digest.0, &tampered_sig, &pub_arr),
            "secp256k1_verify must reject a tampered signature"
        );

        // Negative — different keypair's pubkey.
        let other = SigningKey::random(&mut rng);
        let other_pub = other.verifying_key().to_sec1_bytes();
        let mut other_arr = [0u8; 33];
        other_arr.copy_from_slice(&other_pub);
        assert!(
            !crypto::secp256k1_verify(&digest.0, &sig_bytes, &other_arr),
            "secp256k1_verify must reject a signature against a different verifying key"
        );

        // Negative — non-32-byte digest.
        let bogus_digest = [0u8; 31];
        assert!(
            !crypto::secp256k1_verify(&bogus_digest, &sig_bytes, &pub_arr),
            "secp256k1_verify must reject a digest of incorrect length (fail-closed)"
        );

        // Recover round-trip — produce a signature with explicit
        // RecoveryId, recover the public key, compare against the
        // canonical 65-byte uncompressed SEC1 encoding of the
        // original verifying key.
        let (recoverable_sig, recid) = signing_key.sign_prehash_recoverable(&digest.0).unwrap();
        let recoverable_bytes: [u8; 64] = recoverable_sig.to_bytes().into();
        let recovered = crypto::secp256k1_recover(&digest.0, &recoverable_bytes, recid.to_byte());
        let recovered = recovered.expect("secp256k1_recover must succeed for a valid signature");
        let want = VerifyingKey::recover_from_prehash(&digest.0, &recoverable_sig, recid)
            .unwrap()
            .to_encoded_point(false);
        assert_eq!(
            &recovered[..],
            want.as_bytes(),
            "secp256k1_recover must round-trip the canonical 65-byte uncompressed pubkey"
        );

        // Negative — flipped digest byte must produce a different recovered key.
        let mut wrong_recover_digest = digest.0;
        wrong_recover_digest[5] ^= 0x01;
        if let Some(other_recovered) =
            crypto::secp256k1_recover(&wrong_recover_digest, &recoverable_bytes, recid.to_byte())
        {
            assert_ne!(&other_recovered[..], want.as_bytes(),
                "secp256k1_recover must produce a different pubkey for a tampered digest (no unconditional success)");
        }

        // Negative — bad recovery_id must fail.
        let bad_recover = crypto::secp256k1_recover(&digest.0, &recoverable_bytes, 99);
        assert!(
            bad_recover.is_none(),
            "secp256k1_recover must reject an out-of-range recovery_id"
        );

        // Negative — non-32-byte digest must fail.
        let bad_msg_recover =
            crypto::secp256k1_recover(&[0u8; 31], &recoverable_bytes, recid.to_byte());
        assert!(
            bad_msg_recover.is_none(),
            "secp256k1_recover must reject a digest of incorrect length"
        );
    }

    #[test]
    fn test_crypto_bls12_381_verify_fails_closed_in_test_mode() {
        // P0-003 documented error path: the std/test mock host
        // intentionally fails BLS12-381 verification rather than
        // returning an unconditional success. wasm32 builds reach a
        // real BLS12-381 verifier through the host import.
        let msg = b"any message";
        let sig = [0u8; 96];
        let pubkey = [0u8; 48];
        assert!(!crypto::bls12_381_verify(msg, &sig, &pubkey),
            "bls12_381_verify must NOT return unconditional success in std/test mode (P0-003 — fail-closed)");
    }

    #[test]
    fn test_safe_math() {
        let a = U256::from(100u64);
        let b = U256::from(50u64);

        assert_eq!(utils::safe_math::add(&a, &b).unwrap(), U256::from(150u64));
        assert_eq!(utils::safe_math::sub(&a, &b).unwrap(), U256::from(50u64));
        assert_eq!(utils::safe_math::mul(&a, &b).unwrap(), U256::from(5000u64));
        assert_eq!(utils::safe_math::div(&a, &b).unwrap(), U256::from(2u64));

        // Test division by zero
        assert!(utils::safe_math::div(&a, &U256::ZERO).is_err());

        // Test underflow
        assert!(utils::safe_math::sub(&b, &a).is_err());
    }

    #[test]
    fn test_reentrancy_guard() {
        let guard = utils::ReentrancyGuard::new(b"__reentrancy_lock");

        {
            let _lock = guard.enter().unwrap();
            // Should fail to enter again
            assert!(guard.enter().is_err());
        }

        // Should succeed after lock is dropped
        assert!(guard.enter().is_ok());
    }
}
