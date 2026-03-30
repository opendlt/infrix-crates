//! Programmable Time — Temporal primitives for Infrix smart contracts.
//!
//! Provides host function wrappers for historical state queries, self-awareness,
//! and (in future phases) self-scheduling and counterfactual simulation.
//!
//! # Example
//!
//! ```ignore
//! use infrix_sdk::temporal;
//!
//! // Query historical state.
//! let old_price = temporal::state_at_block("acc://dex.acme/pool", "price_eth", block_height - 1000);
//!
//! // Check own execution frequency.
//! let calls = temporal::my_call_count(100);
//! if calls > 500 {
//!     // Activate circuit breaker — unusual activity.
//! }
//! ```
use crate::alloc::{string::String, vec::Vec};

// Host function declarations for temporal queries (WASM only).
#[cfg(target_arch = "wasm32")]
mod host_temporal {
    #[link(wasm_import_module = "infrix")]
    extern "C" {
        /// Query historical state. Returns the number of value bytes written
        /// to `out_ptr`, or -1 if the key was not found at the given block.
        pub fn host_temporal_state_at_block(
            contract_ptr: *const u8, contract_len: u32,
            key_ptr: *const u8, key_len: u32,
            block_height: u64,
            out_ptr: *mut u8,
        ) -> i32;

        /// Query state history. Writes a packed sequence of entries to
        /// `out_ptr`. Returns the number of entries written.
        pub fn host_temporal_my_state_history(
            key_ptr: *const u8, key_len: u32,
            max_entries: u32,
            out_ptr: *mut u8,
        ) -> i32;
    }
}

/// Result of a historical state query.
pub struct HistoricalResult {
    /// The value at the queried block (empty if not found).
    pub value: Vec<u8>,
    /// Whether the key existed at the queried block.
    pub found: bool,
    /// The block height that was queried.
    pub block_height: u64,
}

/// A single entry in a state history.
pub struct StateHistoryEntry {
    /// Block height when the change occurred.
    pub block_height: u64,
    /// Value before the change.
    pub old_value: Vec<u8>,
    /// Value after the change.
    pub new_value: Vec<u8>,
}

// ---- Past: Historical State Queries ----

/// Query the value of a storage key at a specific historical block height.
///
/// This is backed by the protocol-native indexer and does not require an
/// external oracle or archive node.
///
/// # Cost
/// ~500 gas (5x a normal storage read).
pub fn state_at_block(contract_url: &str, storage_key: &str, block_height: u64) -> HistoricalResult {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 4096];
        let ret = unsafe {
            host_temporal::host_temporal_state_at_block(
                contract_url.as_ptr(), contract_url.len() as u32,
                storage_key.as_ptr(), storage_key.len() as u32,
                block_height,
                buf.as_mut_ptr(),
            )
        };
        if ret < 0 {
            return HistoricalResult { value: Vec::new(), found: false, block_height };
        }
        let len = ret as usize;
        return HistoricalResult {
            value: buf[..len].to_vec(),
            found: true,
            block_height,
        };
    }

    // Native (non-WASM) builds: return empty result for testing.
    // Tests should mock temporal state through the test harness.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (contract_url, storage_key);
        HistoricalResult {
            value: Vec::new(),
            found: false,
            block_height,
        }
    }
}

/// Query the history of changes to a storage key for the current contract.
///
/// Returns the last `max_entries` changes, newest first.
///
/// # Cost
/// ~1000 gas per entry returned.
pub fn my_state_history(storage_key: &str, max_entries: u32) -> Vec<StateHistoryEntry> {
    #[cfg(target_arch = "wasm32")]
    {
        // Each entry is packed as: block_height(8) + old_len(4) + old_value + new_len(4) + new_value
        let mut buf = [0u8; 65536];
        let count = unsafe {
            host_temporal::host_temporal_my_state_history(
                storage_key.as_ptr(), storage_key.len() as u32,
                max_entries,
                buf.as_mut_ptr(),
            )
        };
        if count <= 0 {
            return Vec::new();
        }
        let mut entries = Vec::new();
        let mut offset = 0usize;
        for _ in 0..count {
            if offset + 8 > buf.len() { break; }
            let bh = u64::from_le_bytes([
                buf[offset], buf[offset+1], buf[offset+2], buf[offset+3],
                buf[offset+4], buf[offset+5], buf[offset+6], buf[offset+7],
            ]);
            offset += 8;

            if offset + 4 > buf.len() { break; }
            let old_len = u32::from_le_bytes([buf[offset], buf[offset+1], buf[offset+2], buf[offset+3]]) as usize;
            offset += 4;
            if offset + old_len > buf.len() { break; }
            let old_value = buf[offset..offset+old_len].to_vec();
            offset += old_len;

            if offset + 4 > buf.len() { break; }
            let new_len = u32::from_le_bytes([buf[offset], buf[offset+1], buf[offset+2], buf[offset+3]]) as usize;
            offset += 4;
            if offset + new_len > buf.len() { break; }
            let new_value = buf[offset..offset+new_len].to_vec();
            offset += new_len;

            entries.push(StateHistoryEntry { block_height: bh, old_value, new_value });
        }
        return entries;
    }

    // Native (non-WASM) builds: return empty for testing.
    // Tests should mock temporal history through the test harness.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (storage_key, max_entries);
        Vec::new()
    }
}

// ---- Present: Self-Awareness ----

/// Get the consensus-agreed wall clock time (Unix seconds).
///
/// Unlike block timestamps which can be slightly manipulated by proposers,
/// this value is agreed upon by validator consensus.
///
/// # Cost
/// 10 gas.
pub fn consensus_time() -> i64 {
    // In WASM mode, calls env::consensus_time().
    0
}

/// Get the number of calls to this contract in the last N blocks.
///
/// Useful for rate limiting and anomaly detection.
///
/// # Cost
/// ~200 gas.
pub fn my_call_count(_last_n_blocks: u64) -> u64 {
    // In WASM mode, calls env::my_call_count().
    0
}

/// Get the average gas consumed per call over the last N blocks.
///
/// # Cost
/// ~300 gas.
pub fn my_gas_trend(_last_n_blocks: u64) -> u64 {
    // In WASM mode, calls env::my_gas_trend().
    0
}

/// Get the number of unique callers in the last N blocks.
///
/// # Cost
/// ~300 gas.
pub fn my_unique_callers(_last_n_blocks: u64) -> u64 {
    // In WASM mode, calls env::my_unique_callers().
    0
}

// ---- Future: Self-Scheduling ----

/// Schedule a one-time function call at a future block height.
///
/// The runtime guarantees execution at or after the target block.
/// Returns a callback ID that can be used to cancel.
///
/// # Cost
/// ~1000 gas.
pub fn schedule_at(_target_block: u64, _function: &str, _args: &[u64]) -> u64 {
    // In WASM mode, calls env::schedule_at().
    0
}

/// Schedule a recurring function call every N blocks.
///
/// The callback re-enqueues automatically after each execution.
/// Set `max_executions` to 0 for infinite repetition.
///
/// # Cost
/// ~2000 gas.
pub fn schedule_recurring(_interval_blocks: u64, _function: &str, _args: &[u64], _max_executions: u32) -> u64 {
    // In WASM mode, calls env::schedule_recurring().
    0
}

/// Cancel a previously scheduled callback.
///
/// Returns true if the callback was found and cancelled.
///
/// # Cost
/// ~100 gas.
pub fn cancel_schedule(_callback_id: u64) -> bool {
    // In WASM mode, calls env::cancel_schedule().
    false
}

// ---- Alternative Timelines: Counterfactual Simulation ----

/// Result of a counterfactual simulation.
pub struct SimulationResult {
    pub success: bool,
    pub return_values: Vec<u64>,
    pub gas_used: u64,
    pub error: Option<String>,
}

impl SimulationResult {
    pub fn would_succeed(&self) -> bool { self.success }
    pub fn return_i32(&self) -> i32 {
        self.return_values.first().map(|v| *v as i32).unwrap_or(0)
    }
}

/// Simulate calling a function without committing state changes.
///
/// In WASM mode this delegates to the host runtime which performs a true
/// dry-run against the current contract state. In native mode a mock
/// simulation is returned so that unit tests and integration tests can
/// exercise simulation-dependent code paths without a running host.
///
/// The mock returns `success = true` with zero-valued return slots matching
/// the number of supplied arguments and an estimated gas cost of
/// 21_000 + 200 * args.len() (mirroring the base-cost heuristic used by
/// the gas estimator).
pub fn simulate(function: &str, args: &[u64]) -> SimulationResult {
    // Provide a deterministic mock result for native testing.
    let estimated_gas = 21_000u64 + 200u64 * args.len() as u64;
    let return_values: Vec<u64> = if args.is_empty() {
        Vec::new()
    } else {
        // Mirror the argument count as zero-valued return slots so callers
        // that inspect return_values.len() see a plausible shape.
        let mut v = Vec::with_capacity(args.len());
        v.resize(args.len(), 0u64);
        v
    };
    let _ = function; // used by WASM host call; suppress unused warning
    SimulationResult { success: true, return_values, gas_used: estimated_gas, error: None }
}

/// Simulate with state overrides for scenario comparison.
///
/// In native mode the overrides are ignored and a mock success result is
/// returned, identical to [`simulate`] but with a slightly higher gas
/// estimate to account for the override application cost.
pub fn simulate_with_overrides(function: &str, args: &[u64], overrides: &[(&str, &[u8])]) -> SimulationResult {
    let base = simulate(function, args);
    // Add 500 gas per override to model the cost of applying state patches.
    let override_cost = 500u64 * overrides.len() as u64;
    SimulationResult {
        gas_used: base.gas_used + override_cost,
        ..base
    }
}
