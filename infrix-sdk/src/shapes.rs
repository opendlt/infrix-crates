//! Shape-Shifting Contracts — parameter access and shape query API.
//!
//! Shape-shifting contracts declare multiple named parameter configurations
//! (shapes) at deploy time. The runtime selects which shape is active based
//! on evolution rules evaluated each block. Contract code accesses the
//! current shape's parameters through the functions in this module.
//!
//! # Example
//!
//! ```ignore
//! use infrix_sdk::shapes;
//!
//! let ltv: u64 = shapes::param_u64("ltv_ratio");
//! let enabled: bool = shapes::param_bool("borrow_enabled");
//! let model: String = shapes::param_string("interest_model");
//! let current = shapes::current_shape();
//! ```
use crate::alloc::{string::String, vec::Vec};

#[cfg(target_arch = "wasm32")]
mod host_shapes {
    #[link(wasm_import_module = "infrix")]
    extern "C" {
        pub fn env_current_shape(result_ptr: *mut u8) -> i32;
        pub fn env_shape_param_u64(name_ptr: *const u8, name_len: u32) -> i64;
        pub fn env_shape_param_i64(name_ptr: *const u8, name_len: u32) -> i64;
        pub fn env_shape_param_bool(name_ptr: *const u8, name_len: u32) -> i32;
        pub fn env_shape_param_string(
            name_ptr: *const u8,
            name_len: u32,
            result_ptr: *mut u8,
        ) -> i32;
        pub fn env_shape_param_bytes(
            name_ptr: *const u8,
            name_len: u32,
            result_ptr: *mut u8,
        ) -> i32;
        pub fn env_shape_param_u128(name_ptr: *const u8, name_len: u32, result_ptr: *mut u8)
            -> i32;
        pub fn env_shape_active_since() -> i64;
        pub fn env_shape_time_in(name_ptr: *const u8, name_len: u32) -> i64;
        pub fn env_shape_history(count: i32, result_ptr: *mut u8) -> i32;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn panic_native_shape_host(function: &str, name: Option<&str>) -> ! {
    match name {
        Some(name) => panic!(
            "{function}({name}): shape host functions are only available in the Infrix WASM runtime"
        ),
        None => {
            panic!("{function}: shape host functions are only available in the Infrix WASM runtime")
        }
    }
}

/// Returns the name of the currently active shape.
///
/// # Panics
///
/// Panics if the contract is not shape-shifting enabled.
pub fn current_shape() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 64];
        let len = unsafe { host_shapes::env_current_shape(buf.as_mut_ptr()) };
        if len < 0 {
            panic!("current_shape: shape-shifting not enabled");
        }
        String::from_utf8_lossy(&buf[..len as usize]).into_owned()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("current_shape", None)
    }
}

/// Returns a `u64` shape parameter by name.
///
/// # Panics
///
/// Panics if the parameter does not exist or is not a u64.
pub fn param_u64(name: &str) -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        let result = unsafe { host_shapes::env_shape_param_u64(name.as_ptr(), name.len() as u32) };
        if result < 0 {
            panic!("shape_param_u64({name}): error {result}");
        }
        result as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("param_u64", Some(name))
    }
}

/// Returns an `i64` shape parameter by name.
pub fn param_i64(name: &str) -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        let result = unsafe { host_shapes::env_shape_param_i64(name.as_ptr(), name.len() as u32) };
        if result == i64::MIN {
            panic!("shape_param_i64({name}): error");
        }
        result
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("param_i64", Some(name))
    }
}

/// Returns a `bool` shape parameter by name.
pub fn param_bool(name: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let result = unsafe { host_shapes::env_shape_param_bool(name.as_ptr(), name.len() as u32) };
        if result < 0 {
            panic!("shape_param_bool({name}): error {result}");
        }
        result != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("param_bool", Some(name))
    }
}

/// Returns a `String` shape parameter by name.
pub fn param_string(name: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 1024];
        let len = unsafe {
            host_shapes::env_shape_param_string(name.as_ptr(), name.len() as u32, buf.as_mut_ptr())
        };
        if len < 0 {
            panic!("shape_param_string({name}): error {len}");
        }
        String::from_utf8_lossy(&buf[..len as usize]).into_owned()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("param_string", Some(name))
    }
}

/// Returns a `bytes` shape parameter by name as a `Vec<u8>`.
pub fn param_bytes(name: &str) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 4096];
        let len = unsafe {
            host_shapes::env_shape_param_bytes(name.as_ptr(), name.len() as u32, buf.as_mut_ptr())
        };
        if len < 0 {
            panic!("shape_param_bytes({name}): error {len}");
        }
        buf[..len as usize].to_vec()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("param_bytes", Some(name))
    }
}

/// Returns a `u128` shape parameter by name.
pub fn param_u128(name: &str) -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 16];
        let result = unsafe {
            host_shapes::env_shape_param_u128(name.as_ptr(), name.len() as u32, buf.as_mut_ptr())
        };
        if result < 0 {
            panic!("shape_param_u128({name}): error {result}");
        }
        u128::from_be_bytes(buf)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("param_u128", Some(name))
    }
}

/// Returns the block height at which the current shape was activated.
pub fn active_since() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        let result = unsafe { host_shapes::env_shape_active_since() };
        if result < 0 {
            panic!("shape_active_since: error {result}");
        }
        result as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("active_since", None)
    }
}

/// Returns total blocks the contract has spent in the named shape.
pub fn time_in(shape_name: &str) -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        let result =
            unsafe { host_shapes::env_shape_time_in(shape_name.as_ptr(), shape_name.len() as u32) };
        if result < 0 {
            panic!("shape_time_in({shape_name}): error {result}");
        }
        result as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic_native_shape_host("time_in", Some(shape_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_shape_api_fails_closed_instead_of_returning_mock_values() {
        assert!(std::panic::catch_unwind(current_shape).is_err());
        assert!(std::panic::catch_unwind(|| param_u64("ltv_ratio")).is_err());
        assert!(std::panic::catch_unwind(|| param_i64("delta")).is_err());
        assert!(std::panic::catch_unwind(|| param_bool("enabled")).is_err());
        assert!(std::panic::catch_unwind(|| param_string("model")).is_err());
        assert!(std::panic::catch_unwind(|| param_bytes("blob")).is_err());
        assert!(std::panic::catch_unwind(|| param_u128("limit")).is_err());
        assert!(std::panic::catch_unwind(active_since).is_err());
        assert!(std::panic::catch_unwind(|| time_in("growth")).is_err());
    }
}
