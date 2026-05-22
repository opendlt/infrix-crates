//! Swarm Contracts — shared channels, member state, and coordinated actions.
//!
//! This module provides the Rust SDK for swarm contract operations. Contracts
//! that are members of a swarm can use these functions to interact with the
//! shared communication channel, read other members' state, and trigger
//! coordinated actions.
//!
//! # Example
//!
//! ```ignore
//! use infrix_sdk::swarm;
//!
//! // Read from the shared channel
//! let price: u64 = swarm::channel::get_u64("eth_price")?;
//!
//! // Write to the shared channel
//! swarm::channel::set_u64("total_borrows", 1000)?;
//!
//! // Check swarm immune state
//! let state = swarm::immune_state()?;
//!
//! // List swarm members
//! let members = swarm::members()?;
//! ```
use crate::alloc::{string::String, vec::Vec};

/// Swarm error types.
#[derive(Debug)]
pub enum SwarmError {
    NotInSwarm,
    AccessDenied,
    KeyNotFound,
    SchemaViolation,
    SizeExceeded,
    ActionNotFound,
    ActionAlreadyActive,
    ActionFailed(String),
}

impl core::fmt::Display for SwarmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotInSwarm => write!(f, "contract is not a swarm member"),
            Self::AccessDenied => write!(f, "channel access denied"),
            Self::KeyNotFound => write!(f, "channel key not found"),
            Self::SchemaViolation => write!(f, "channel schema violation"),
            Self::SizeExceeded => write!(f, "channel size limit exceeded"),
            Self::ActionNotFound => write!(f, "coordinated action not found"),
            Self::ActionAlreadyActive => write!(f, "coordinated action already executing"),
            Self::ActionFailed(msg) => write!(f, "coordinated action failed: {msg}"),
        }
    }
}

/// Swarm immune states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmuneState {
    Normal,
    Throttled,
    Paused,
    Frozen,
}

// ---- Host function declarations ----

#[cfg(target_arch = "wasm32")]
mod host_swarm {
    #[link(wasm_import_module = "infrix_swarm")]
    extern "C" {
        pub fn swarm_id(out_ptr: *mut u8) -> i32;
        pub fn swarm_channel_get(key_ptr: *const u8, key_len: u32, out_ptr: *mut u8) -> i32;
        pub fn swarm_channel_set(
            key_ptr: *const u8,
            key_len: u32,
            val_ptr: *const u8,
            val_len: u32,
        ) -> i32;
        pub fn swarm_channel_has(key_ptr: *const u8, key_len: u32) -> i32;
        pub fn swarm_channel_delete(key_ptr: *const u8, key_len: u32) -> i32;
        pub fn swarm_members(out_ptr: *mut u8, out_len: u32) -> i32;
        pub fn swarm_member_state(
            addr_ptr: *const u8,
            addr_len: u32,
            key_ptr: *const u8,
            key_len: u32,
            out_ptr: *mut u8,
        ) -> i32;
        pub fn swarm_coordinate(
            action_ptr: *const u8,
            action_len: u32,
            args_ptr: *const u8,
            args_len: u32,
            out_ptr: *mut u8,
        ) -> i32;
        pub fn swarm_immune_state() -> i32;
    }
}

// ---- Public API ----

/// Returns the swarm ID of the calling contract, or None if not in a swarm.
pub fn swarm_id() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 64];
        let len = unsafe { host_swarm::swarm_id(buf.as_mut_ptr()) };
        if len <= 0 {
            return None;
        }
        Some(String::from_utf8_lossy(&buf[..len as usize]).into_owned())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Returns the list of member addresses in the swarm.
pub fn members() -> Result<Vec<String>, SwarmError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 4096];
        let len = unsafe { host_swarm::swarm_members(buf.as_mut_ptr(), buf.len() as u32) };
        if len < 0 {
            return Err(SwarmError::NotInSwarm);
        }
        if len == 0 {
            return Ok(Vec::new());
        }
        let data = &buf[..len as usize];
        Ok(data
            .split(|&b| b == 0)
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Vec::new())
    }
}

/// Reads a storage value from another swarm member.
pub fn member_state(member_address: &str, key: &str) -> Result<Vec<u8>, SwarmError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 4096];
        let len = unsafe {
            host_swarm::swarm_member_state(
                member_address.as_ptr(),
                member_address.len() as u32,
                key.as_ptr(),
                key.len() as u32,
                buf.as_mut_ptr(),
            )
        };
        if len < 0 {
            return Err(SwarmError::NotInSwarm);
        }
        Ok(buf[..len as usize].to_vec())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (member_address, key);
        Ok(Vec::new())
    }
}

/// Returns the collective immune state of the swarm.
pub fn immune_state() -> Result<ImmuneState, SwarmError> {
    #[cfg(target_arch = "wasm32")]
    {
        let state = unsafe { host_swarm::swarm_immune_state() };
        match state {
            0 => Ok(ImmuneState::Normal),
            1 => Ok(ImmuneState::Throttled),
            2 => Ok(ImmuneState::Paused),
            3 => Ok(ImmuneState::Frozen),
            _ => Err(SwarmError::NotInSwarm),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(ImmuneState::Normal)
    }
}

/// Triggers a coordinated action programmatically.
pub fn coordinate(action: &str, args_json: &[u8]) -> Result<Vec<u8>, SwarmError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 8192];
        let len = unsafe {
            host_swarm::swarm_coordinate(
                action.as_ptr(),
                action.len() as u32,
                args_json.as_ptr(),
                args_json.len() as u32,
                buf.as_mut_ptr(),
            )
        };
        if len < 0 {
            return match len {
                -1 => Err(SwarmError::ActionNotFound),
                -2 => Err(SwarmError::AccessDenied),
                -3 => Err(SwarmError::ActionAlreadyActive),
                _ => Err(SwarmError::ActionFailed("unknown error".into())),
            };
        }
        Ok(buf[..len as usize].to_vec())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (action, args_json);
        Ok(Vec::new())
    }
}

/// Shared channel operations.
pub mod channel {
    use super::SwarmError;
    use crate::alloc::vec::Vec;

    /// Read raw bytes from the shared channel.
    pub fn get(key: &str) -> Result<Vec<u8>, SwarmError> {
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = [0u8; 4096];
            let len = unsafe {
                super::host_swarm::swarm_channel_get(
                    key.as_ptr(),
                    key.len() as u32,
                    buf.as_mut_ptr(),
                )
            };
            match len {
                n if n >= 0 => Ok(buf[..n as usize].to_vec()),
                -1 => Err(SwarmError::KeyNotFound),
                -2 => Err(SwarmError::AccessDenied),
                _ => Err(SwarmError::NotInSwarm),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            Ok(Vec::new())
        }
    }

    /// Read a u64 value from the shared channel.
    pub fn get_u64(key: &str) -> Result<u64, SwarmError> {
        let data = get(key)?;
        if data.len() < 8 {
            return Ok(0);
        }
        Ok(u64::from_be_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    /// Write raw bytes to the shared channel.
    pub fn set(key: &str, value: &[u8]) -> Result<(), SwarmError> {
        #[cfg(target_arch = "wasm32")]
        {
            let status = unsafe {
                super::host_swarm::swarm_channel_set(
                    key.as_ptr(),
                    key.len() as u32,
                    value.as_ptr(),
                    value.len() as u32,
                )
            };
            match status {
                0 => Ok(()),
                1 => Err(SwarmError::SchemaViolation),
                2 => Err(SwarmError::AccessDenied),
                3 => Err(SwarmError::NotInSwarm),
                4 => Err(SwarmError::SizeExceeded),
                _ => Err(SwarmError::SchemaViolation),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
            Ok(())
        }
    }

    /// Write a u64 value to the shared channel.
    pub fn set_u64(key: &str, value: u64) -> Result<(), SwarmError> {
        set(key, &value.to_be_bytes())
    }

    /// Check if a key exists in the shared channel.
    pub fn has(key: &str) -> Result<bool, SwarmError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result =
                unsafe { super::host_swarm::swarm_channel_has(key.as_ptr(), key.len() as u32) };
            match result {
                1 => Ok(true),
                0 => Ok(false),
                -1 => Err(SwarmError::NotInSwarm),
                -2 => Err(SwarmError::AccessDenied),
                _ => Ok(false),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            Ok(false)
        }
    }

    /// Delete a key from the shared channel.
    pub fn delete(key: &str) -> Result<(), SwarmError> {
        #[cfg(target_arch = "wasm32")]
        {
            let status =
                unsafe { super::host_swarm::swarm_channel_delete(key.as_ptr(), key.len() as u32) };
            match status {
                0 => Ok(()),
                1 => Err(SwarmError::KeyNotFound),
                2 => Err(SwarmError::AccessDenied),
                _ => Err(SwarmError::NotInSwarm),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            Ok(())
        }
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::channel;
    pub use super::{coordinate, immune_state, member_state, members, swarm_id};
    pub use super::{ImmuneState, SwarmError};
}
