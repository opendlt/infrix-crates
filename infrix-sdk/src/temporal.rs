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
use crate::alloc::{string::{String, ToString}, vec::Vec, format};

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
pub fn state_at_block(_contract_url: &str, _storage_key: &str, _block_height: u64) -> HistoricalResult {
    // In WASM mode, this calls:
    //   env::state_at_block(contract_ptr, contract_len, key_ptr, key_len, block_height)
    // For native testing, return a placeholder.
    HistoricalResult {
        value: Vec::new(),
        found: false,
        block_height: _block_height,
    }
}

/// Query the history of changes to a storage key for the current contract.
///
/// Returns the last `max_entries` changes, newest first.
///
/// # Cost
/// ~1000 gas per entry returned.
pub fn my_state_history(_storage_key: &str, _max_entries: u32) -> Vec<StateHistoryEntry> {
    // In WASM mode, calls env::my_state_history().
    Vec::new()
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
pub fn simulate(_function: &str, _args: &[u64]) -> SimulationResult {
    SimulationResult { success: false, return_values: Vec::new(), gas_used: 0, error: Some("not available in native mode".into()) }
}

/// Simulate with state overrides for scenario comparison.
pub fn simulate_with_overrides(_function: &str, _args: &[u64], _overrides: &[(&str, &[u8])]) -> SimulationResult {
    SimulationResult { success: false, return_values: Vec::new(), gas_used: 0, error: Some("not available in native mode".into()) }
}
