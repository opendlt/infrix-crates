//! Mission Control — production observability for smart contracts.
//!
//! Contracts can read their own observability metrics through these
//! host functions to make self-adaptive decisions.
//!
//! # Example
//! ```ignore
//! use infrix_sdk::mission;
//!
//! let calls = mission::calls_total();
//! let error_rate = mission::error_rate();
//! if error_rate > 0.05 {
//!     // Error rate too high, reduce batch size
//! }
//! ```

#[cfg(target_arch = "wasm32")]
mod host_mission {
    #[link(wasm_import_module = "infrix")]
    extern "C" {
        pub fn env_mission_calls_total() -> u64;
        pub fn env_mission_error_rate() -> f64;
        pub fn env_mission_gas_avg() -> f64;
        pub fn env_mission_uptime() -> f64;
        pub fn env_mission_anomaly_score() -> f64;
        pub fn env_mission_slo_status(name_ptr: *const u8, name_len: u32) -> f64;
    }
}

/// Returns the total lifetime call count for this contract.
pub fn calls_total() -> u64 {
    #[cfg(target_arch = "wasm32")]
    { unsafe { host_mission::env_mission_calls_total() } }
    #[cfg(not(target_arch = "wasm32"))]
    { 0 }
}

/// Returns the rolling error rate (0.0-1.0).
pub fn error_rate() -> f64 {
    #[cfg(target_arch = "wasm32")]
    { unsafe { host_mission::env_mission_error_rate() } }
    #[cfg(not(target_arch = "wasm32"))]
    { 0.0 }
}

/// Returns the average gas per call.
pub fn gas_avg() -> f64 {
    #[cfg(target_arch = "wasm32")]
    { unsafe { host_mission::env_mission_gas_avg() } }
    #[cfg(not(target_arch = "wasm32"))]
    { 0.0 }
}

/// Returns the uptime percentage (0.0-1.0).
pub fn uptime() -> f64 {
    #[cfg(target_arch = "wasm32")]
    { unsafe { host_mission::env_mission_uptime() } }
    #[cfg(not(target_arch = "wasm32"))]
    { 1.0 }
}

/// Returns the current anomaly score (0.0-1.0).
pub fn anomaly_score() -> f64 {
    #[cfg(target_arch = "wasm32")]
    { unsafe { host_mission::env_mission_anomaly_score() } }
    #[cfg(not(target_arch = "wasm32"))]
    { 0.0 }
}

/// Returns the SLO compliance (0.0-1.0) for the named SLO.
/// Returns -1.0 if the SLO is not found.
pub fn slo_status(name: &str) -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        unsafe {
            host_mission::env_mission_slo_status(name.as_ptr(), name.len() as u32)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = name; 1.0 }
}
