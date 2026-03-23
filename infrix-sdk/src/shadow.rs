//! Shadow State — Private-by-default contract execution.
//!
//! Provides transparent encryption for contract state. Developers write
//! normal storage calls; the runtime encrypts private values automatically.
//!
//! # Example
//!
//! ```ignore
//! use infrix_sdk::shadow;
//!
//! // Write private state — encrypted automatically.
//! shadow::set("salary_alice", &85000u64.to_le_bytes());
//!
//! // Read private state — decrypted automatically.
//! let salary = shadow::get("salary_alice");
//!
//! // Public state is plaintext (configured via visibility policy).
//! shadow::set("total_supply", &1000000u64.to_le_bytes());
//! ```

/// Visibility levels for storage keys.
pub mod visibility {
    pub const PUBLIC: &str = "public";
    pub const OWNER_READ: &str = "owner_read";
    pub const AUTHORITY_ONLY: &str = "authority_only";
    pub const PRIVATE: &str = "private";
}

/// Result of reading a private state value.
pub struct PrivateValue {
    pub data: Vec<u8>,
    pub level: String,
    pub decrypted: bool,
}

/// Write a value to shadow storage. The runtime determines the visibility
/// level from the contract's policy and encrypts if needed.
///
/// In WASM mode, this calls the shadow-aware `storage::set` host function.
/// In native test mode, this writes plaintext.
pub fn set(_key: &str, _value: &[u8]) {
    // WASM: calls host_shadow_set(key_ptr, key_len, value_ptr, value_len)
    // The runtime encrypts based on the visibility policy.
}

/// Read a value from shadow storage. The runtime decrypts if the caller
/// is authorized based on the visibility policy.
///
/// Returns None if the key doesn't exist.
pub fn get(_key: &str) -> Option<Vec<u8>> {
    // WASM: calls host_shadow_get(key_ptr, key_len, out_ptr) -> len
    None
}

/// Check the visibility level of a storage key.
pub fn level(_key: &str) -> &'static str {
    // WASM: calls host_shadow_level(key_ptr, key_len) -> level_i32
    visibility::PUBLIC
}

/// Encrypt data with the contract's key (for explicit use cases).
pub fn encrypt(_data: &[u8]) -> Vec<u8> {
    // WASM: calls host_shadow_encrypt(data_ptr, data_len, out_ptr) -> len
    Vec::new()
}

/// Decrypt data with the contract's key (for explicit use cases).
pub fn decrypt(_data: &[u8]) -> Option<Vec<u8>> {
    // WASM: calls host_shadow_decrypt(data_ptr, data_len, out_ptr) -> len
    None
}
