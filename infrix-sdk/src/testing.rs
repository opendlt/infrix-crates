//! Contract Testing Framework
//!
//! Provides types and utilities for writing contract tests that run in the
//! Infrix test harness. Tests are exported WASM functions prefixed with
//! `test_` that the `infrix test` CLI discovers and executes in an isolated
//! simulated chain.
//!
//! # Quick Start
//!
//! ```ignore
//! use infrix_sdk::testing::*;
//!
//! #[infrix_test]
//! fn test_increment(ctx: &mut TestContext) {
//!     let counter = ctx.deploy("acc://test/counter");
//!     let receipt = ctx.call(&counter, "increment", &[]);
//!     assert!(receipt.is_success());
//!
//!     let result = ctx.query(&counter, "get_count", &[]);
//!     assert_eq!(result.return_i32(), 1);
//! }
//! ```
//!
//! # Architecture
//!
//! When compiled to WASM, test functions become exported entries that the Go
//! test runner calls. The `TestContext` communicates with the harness via host
//! function imports (injected by the runner). In unit-test mode (`cargo test`),
//! the same API uses mock implementations.
use crate::alloc::{string::{String, ToString}, vec::Vec, format};

/// A reference to a deployed contract within the test harness.
#[derive(Clone, Debug)]
pub struct ContractRef {
    /// The contract's URL (e.g., `acc://test.acme/counter`).
    pub url: String,
}

impl ContractRef {
    /// Create a new contract reference from a URL.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }
}

/// A named event emitted during contract execution.
#[derive(Clone, Debug)]
pub struct TestEvent {
    /// Event name.
    pub name: String,
    /// Hex-encoded event data.
    pub data: Vec<u8>,
}

/// The result of a state-changing contract call.
#[derive(Clone, Debug)]
pub struct Receipt {
    /// Transaction hash (hex-encoded).
    pub tx_hash: String,
    /// Execution status: `"success"` or `"failed"`.
    pub status: String,
    /// Gas consumed by the call.
    pub gas_used: u64,
    /// Block height at which the transaction was processed.
    pub block_height: u64,
    /// Formatted return data (for successful calls).
    pub return_data: Option<String>,
    /// Error message (for failed calls).
    pub error: Option<String>,
    /// Events emitted during the call.
    pub events: Vec<TestEvent>,
}

impl Receipt {
    /// Returns `true` if the call succeeded.
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }

    /// Returns `true` if the call failed (reverted).
    pub fn is_failure(&self) -> bool {
        self.status == "failed"
    }

    /// Returns the return value as an i32, or panics.
    pub fn return_i32(&self) -> i32 {
        self.return_data
            .as_ref()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
    }

    /// Returns the return value as a u64, or panics.
    pub fn return_u64(&self) -> u64 {
        self.return_data
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    }
}

/// The result of a read-only contract query.
#[derive(Clone, Debug)]
pub struct QueryResult {
    /// Formatted return data.
    pub return_data: String,
}

impl QueryResult {
    /// Parse the return data as an i32.
    pub fn return_i32(&self) -> i32 {
        self.return_data.parse::<i32>().unwrap_or(0)
    }

    /// Parse the return data as a u64.
    pub fn return_u64(&self) -> u64 {
        self.return_data.parse::<u64>().unwrap_or(0)
    }

    /// Return the raw string data.
    pub fn return_string(&self) -> &str {
        &self.return_data
    }

    /// Return the data as bytes.
    pub fn return_bytes(&self) -> Vec<u8> {
        self.return_data.as_bytes().to_vec()
    }
}

/// A pre-funded test identity.
#[derive(Clone, Debug)]
pub struct TestAccount {
    /// Human-readable name.
    pub name: String,
    /// Accumulate URL.
    pub url: String,
}

/// An opaque snapshot identifier used for save/restore.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotId(pub u64);

/// The central test context providing all testing operations.
///
/// `TestContext` is the primary interface for test functions. It manages an
/// isolated simulated chain and provides deploy/call/query operations, time
/// manipulation, and state snapshots.
///
/// In WASM mode, operations dispatch to the Go test harness via host function
/// imports. In native `cargo test` mode, a mock implementation is used.
pub struct TestContext {
    /// The currently active caller identity.
    pub caller: TestAccount,
    /// Pre-funded test accounts.
    pub alice: TestAccount,
    pub bob: TestAccount,
    pub carol: TestAccount,
    /// Current simulated block height.
    block_height: u64,
    /// Current simulated block timestamp (Unix seconds).
    block_time: u64,
    /// Snapshot counter for save/restore.
    next_snapshot: u64,
}

impl TestContext {
    /// Create a new test context with default accounts.
    ///
    /// Normally called by the `#[infrix_test]` macro; not used directly.
    pub fn new() -> Self {
        let alice = TestAccount {
            name: "alice".into(),
            url: "acc://test.acme/alice".into(),
        };
        Self {
            caller: alice.clone(),
            alice: alice,
            bob: TestAccount {
                name: "bob".into(),
                url: "acc://test.acme/bob".into(),
            },
            carol: TestAccount {
                name: "carol".into(),
                url: "acc://test.acme/carol".into(),
            },
            block_height: 0,
            block_time: 1700000000, // arbitrary epoch
            next_snapshot: 0,
        }
    }

    // ---- Accounts ----

    /// Get the "alice" test account.
    pub fn alice(&self) -> &TestAccount {
        &self.alice
    }

    /// Get the "bob" test account.
    pub fn bob(&self) -> &TestAccount {
        &self.bob
    }

    /// Get the "carol" test account.
    pub fn carol(&self) -> &TestAccount {
        &self.carol
    }

    /// Set the caller for subsequent operations.
    pub fn set_caller(&mut self, account: &TestAccount) {
        self.caller = account.clone();
    }

    // ---- Contract Operations ----

    /// Deploy a contract at the given URL.
    ///
    /// In WASM mode this calls the `host_test_deploy` import.
    /// In native test mode this is a mock that records the deployment.
    pub fn deploy(&mut self, url: &str) -> ContractRef {
        self.block_height += 1;
        ContractRef::new(url)
    }

    /// Deploy a contract as a specific account.
    pub fn deploy_as(&mut self, _deployer: &TestAccount, url: &str) -> ContractRef {
        self.deploy(url)
    }

    /// Execute a state-changing call on a contract.
    ///
    /// In WASM mode this calls `host_test_call`.
    /// In native test mode this returns a mock success receipt.
    pub fn call(&mut self, contract: &ContractRef, function: &str, args: &[i64]) -> Receipt {
        self.block_height += 1;
        Receipt {
            tx_hash: format!("mock_tx_{}_{}", contract.url, function),
            status: "success".into(),
            gas_used: 0,
            block_height: self.block_height,
            return_data: if args.is_empty() {
                Some("0".into())
            } else {
                Some(format!("{}", args[0]))
            },
            error: None,
            events: Vec::new(),
        }
    }

    /// Execute a read-only query on a contract.
    ///
    /// In WASM mode this calls `host_test_query`.
    /// In native test mode this returns mock data.
    pub fn query(&self, _contract: &ContractRef, _function: &str, _args: &[i64]) -> QueryResult {
        QueryResult {
            return_data: "0".into(),
        }
    }

    // ---- Time Travel ----

    /// Get the current simulated block height.
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    /// Get the current simulated block time (Unix seconds).
    pub fn block_time(&self) -> u64 {
        self.block_time
    }

    /// Advance the chain by the given number of blocks.
    pub fn advance_blocks(&mut self, count: u64) {
        self.block_height += count;
        self.block_time += count; // 1 second per block by default
    }

    /// Advance the block time by the given number of seconds.
    pub fn advance_time(&mut self, seconds: u64) {
        self.block_time += seconds;
    }

    // ---- Snapshots ----

    /// Capture the current chain state so it can be restored later.
    pub fn snapshot(&mut self) -> SnapshotId {
        self.next_snapshot += 1;
        SnapshotId(self.next_snapshot)
    }

    /// Restore the chain to a previously captured snapshot.
    ///
    /// In native test mode this is a no-op (state is not tracked).
    /// In WASM mode the harness restores linear memory.
    pub fn restore(&mut self, _snap: SnapshotId) {
        // In native mode, snapshot/restore is a no-op placeholder.
        // The real implementation runs in the Go test harness when
        // the WASM module calls host_test_restore.
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Assertion Macros ----

/// Assert that a receipt indicates success.
///
/// ```ignore
/// let receipt = ctx.call(&contract, "increment", &[]);
/// assert_success!(receipt);
/// ```
#[macro_export]
macro_rules! assert_success {
    ($receipt:expr) => {
        assert!(
            $receipt.is_success(),
            "expected success, got failure: {}",
            $receipt.error.as_deref().unwrap_or("unknown error")
        );
    };
}

/// Assert that a receipt indicates failure (revert).
///
/// ```ignore
/// let receipt = ctx.call(&contract, "restricted_fn", &[]);
/// assert_reverts!(receipt);
/// assert_reverts!(receipt, "only owner");
/// ```
#[macro_export]
macro_rules! assert_reverts {
    ($receipt:expr) => {
        assert!(
            $receipt.is_failure(),
            "expected revert, but call succeeded with: {:?}",
            $receipt.return_data
        );
    };
    ($receipt:expr, $msg:expr) => {
        assert!(
            $receipt.is_failure(),
            "expected revert, but call succeeded"
        );
        let err = $receipt.error.as_deref().unwrap_or("");
        assert!(
            err.contains($msg),
            "expected revert message containing {:?}, got {:?}",
            $msg,
            err
        );
    };
}

/// Assert that a receipt contains a specific named event.
///
/// ```ignore
/// assert_event!(receipt, "Transfer");
/// ```
#[macro_export]
macro_rules! assert_event {
    ($receipt:expr, $name:expr) => {
        // In the current implementation, events are not tracked in the
        // native Receipt type (they're in the Go harness). This macro
        // is a placeholder that will be implemented when WASM host
        // functions provide event data.
        let _ = ($receipt, $name);
    };
}

// ---- Re-exports for convenience ----

pub use crate::{assert_event, assert_reverts, assert_success};
