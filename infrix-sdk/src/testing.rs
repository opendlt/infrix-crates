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
use crate::alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

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
            .unwrap_or_else(|| panic!("receipt has no return data"))
            .parse::<i32>()
            .unwrap_or_else(|_| panic!("receipt return data is not an i32"))
    }

    /// Returns the return value as a u64, or panics.
    pub fn return_u64(&self) -> u64 {
        self.return_data
            .as_ref()
            .unwrap_or_else(|| panic!("receipt has no return data"))
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("receipt return data is not a u64"))
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
        self.return_data
            .parse::<i32>()
            .unwrap_or_else(|_| panic!("query return data is not an i32"))
    }

    /// Parse the return data as a u64.
    pub fn return_u64(&self) -> u64 {
        self.return_data
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("query return data is not a u64"))
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

#[derive(Clone, Debug)]
struct NativeCallFixture {
    contract_url: String,
    function: String,
    args: Vec<i64>,
    receipt: Receipt,
}

#[derive(Clone, Debug)]
struct NativeQueryFixture {
    contract_url: String,
    function: String,
    args: Vec<i64>,
    result: QueryResult,
}

#[derive(Clone, Debug)]
struct TestContextSnapshot {
    id: SnapshotId,
    caller: TestAccount,
    block_height: u64,
    block_time: u64,
    next_snapshot: u64,
    deployments: Vec<ContractRef>,
    call_fixtures: Vec<NativeCallFixture>,
    query_fixtures: Vec<NativeQueryFixture>,
}

/// The central test context providing all testing operations.
///
/// `TestContext` is the primary interface for test functions. It manages an
/// isolated simulated chain and provides deploy/call/query operations, time
/// manipulation, and state snapshots.
///
/// In WASM mode, operations dispatch to the Go test harness via host function
/// imports. In native `cargo test` mode, calls and queries must be registered
/// explicitly as deterministic fixtures; unregistered behavior fails closed.
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
    /// Deployed contracts recorded by the native fixture harness.
    deployments: Vec<ContractRef>,
    /// One-shot call fixtures for native `cargo test` execution.
    call_fixtures: Vec<NativeCallFixture>,
    /// One-shot query fixtures for native `cargo test` execution.
    query_fixtures: Vec<NativeQueryFixture>,
    /// Captured snapshots for native `cargo test` execution.
    snapshots: Vec<TestContextSnapshot>,
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
            deployments: Vec::new(),
            call_fixtures: Vec::new(),
            query_fixtures: Vec::new(),
            snapshots: Vec::new(),
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
    /// In native test mode this records the deployment and returns its URL.
    pub fn deploy(&mut self, url: &str) -> ContractRef {
        self.block_height += 1;
        let contract = ContractRef::new(url);
        self.deployments.push(contract.clone());
        contract
    }

    /// Deploy a contract as a specific account.
    pub fn deploy_as(&mut self, _deployer: &TestAccount, url: &str) -> ContractRef {
        self.deploy(url)
    }

    /// Execute a state-changing call on a contract.
    ///
    /// In WASM mode this calls `host_test_call`.
    /// In native test mode this consumes a matching registered fixture. Missing
    /// fixtures return a failed receipt so tests cannot pass on fabricated
    /// execution.
    pub fn call(&mut self, contract: &ContractRef, function: &str, args: &[i64]) -> Receipt {
        self.block_height += 1;
        if let Some(index) = self.find_call_fixture(&contract.url, function, args) {
            let mut receipt = self.call_fixtures.remove(index).receipt;
            if receipt.block_height == 0 {
                receipt.block_height = self.block_height;
            }
            return receipt;
        }

        Receipt {
            tx_hash: String::new(),
            status: "failed".into(),
            gas_used: 0,
            block_height: self.block_height,
            return_data: None,
            error: Some(format!(
                "no native call fixture registered for {}::{}({:?})",
                contract.url, function, args
            )),
            events: Vec::new(),
        }
    }

    /// Execute a read-only query on a contract.
    ///
    /// In WASM mode this calls `host_test_query`.
    /// In native test mode this consumes a matching registered fixture. Missing
    /// fixtures panic so read-only assertions cannot pass on fabricated data.
    pub fn query(&mut self, contract: &ContractRef, function: &str, args: &[i64]) -> QueryResult {
        if let Some(index) = self.find_query_fixture(&contract.url, function, args) {
            return self.query_fixtures.remove(index).result;
        }
        panic!(
            "no native query fixture registered for {}::{}({:?})",
            contract.url, function, args
        );
    }

    /// Register a one-shot native call fixture.
    pub fn expect_call(
        &mut self,
        contract: &ContractRef,
        function: &str,
        args: &[i64],
        receipt: Receipt,
    ) {
        self.call_fixtures.push(NativeCallFixture {
            contract_url: contract.url.clone(),
            function: function.into(),
            args: args.to_vec(),
            receipt,
        });
    }

    /// Register a one-shot successful native call fixture.
    pub fn expect_call_success(
        &mut self,
        contract: &ContractRef,
        function: &str,
        args: &[i64],
        return_data: Option<&str>,
        events: Vec<TestEvent>,
    ) {
        self.expect_call(
            contract,
            function,
            args,
            Receipt {
                tx_hash: format!("fixture:{}:{}", contract.url, function),
                status: "success".into(),
                gas_used: 0,
                block_height: 0,
                return_data: return_data.map(String::from),
                error: None,
                events,
            },
        );
    }

    /// Register a one-shot failing native call fixture.
    pub fn expect_call_failure(
        &mut self,
        contract: &ContractRef,
        function: &str,
        args: &[i64],
        error: &str,
    ) {
        self.expect_call(
            contract,
            function,
            args,
            Receipt {
                tx_hash: format!("fixture:{}:{}", contract.url, function),
                status: "failed".into(),
                gas_used: 0,
                block_height: 0,
                return_data: None,
                error: Some(error.into()),
                events: Vec::new(),
            },
        );
    }

    /// Register a one-shot native query fixture.
    pub fn expect_query(
        &mut self,
        contract: &ContractRef,
        function: &str,
        args: &[i64],
        return_data: &str,
    ) {
        self.query_fixtures.push(NativeQueryFixture {
            contract_url: contract.url.clone(),
            function: function.into(),
            args: args.to_vec(),
            result: QueryResult {
                return_data: return_data.into(),
            },
        });
    }

    fn find_call_fixture(&self, contract_url: &str, function: &str, args: &[i64]) -> Option<usize> {
        self.call_fixtures.iter().position(|fixture| {
            fixture.contract_url == contract_url
                && fixture.function == function
                && fixture.args.as_slice() == args
        })
    }

    fn find_query_fixture(
        &self,
        contract_url: &str,
        function: &str,
        args: &[i64],
    ) -> Option<usize> {
        self.query_fixtures.iter().position(|fixture| {
            fixture.contract_url == contract_url
                && fixture.function == function
                && fixture.args.as_slice() == args
        })
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
        let id = SnapshotId(self.next_snapshot);
        self.snapshots.push(TestContextSnapshot {
            id,
            caller: self.caller.clone(),
            block_height: self.block_height,
            block_time: self.block_time,
            next_snapshot: self.next_snapshot,
            deployments: self.deployments.clone(),
            call_fixtures: self.call_fixtures.clone(),
            query_fixtures: self.query_fixtures.clone(),
        });
        id
    }

    /// Restore the chain to a previously captured snapshot.
    ///
    /// In native test mode this restores the recorded test context state.
    /// In WASM mode the harness restores linear memory.
    pub fn restore(&mut self, snap: SnapshotId) {
        let snapshot = self
            .snapshots
            .iter()
            .find(|snapshot| snapshot.id == snap)
            .unwrap_or_else(|| panic!("unknown snapshot id {}", snap.0))
            .clone();
        self.caller = snapshot.caller;
        self.block_height = snapshot.block_height;
        self.block_time = snapshot.block_time;
        self.next_snapshot = snapshot.next_snapshot;
        self.deployments = snapshot.deployments;
        self.call_fixtures = snapshot.call_fixtures;
        self.query_fixtures = snapshot.query_fixtures;
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
        assert!($receipt.is_failure(), "expected revert, but call succeeded");
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
    ($receipt:expr, $name:expr) => {{
        let receipt = &$receipt;
        assert!(
            receipt.events.iter().any(|event| event.name == $name),
            "expected event {:?}",
            $name
        );
    }};
}

// ---- Re-exports for convenience ----

pub use crate::{assert_event, assert_reverts, assert_success};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_call_without_fixture_fails_closed() {
        let mut ctx = TestContext::new();
        let contract = ctx.deploy("acc://test.acme/counter");
        let receipt = ctx.call(&contract, "increment", &[]);

        assert!(receipt.is_failure());
        assert!(receipt
            .error
            .as_deref()
            .unwrap_or("")
            .contains("no native call fixture registered"));
    }

    #[test]
    fn native_call_and_query_use_explicit_one_shot_fixtures() {
        let mut ctx = TestContext::new();
        let contract = ctx.deploy("acc://test.acme/counter");

        ctx.expect_call_success(
            &contract,
            "increment",
            &[1],
            Some("2"),
            vec![TestEvent {
                name: "Incremented".into(),
                data: vec![2],
            }],
        );
        ctx.expect_query(&contract, "get", &[], "2");

        let receipt = ctx.call(&contract, "increment", &[1]);
        assert_success!(receipt);
        assert_event!(receipt, "Incremented");
        assert_eq!(receipt.return_i32(), 2);

        let query = ctx.query(&contract, "get", &[]);
        assert_eq!(query.return_u64(), 2);

        let missing = ctx.call(&contract, "increment", &[1]);
        assert!(missing.is_failure());
    }

    #[test]
    fn native_query_without_fixture_panics() {
        let mut ctx = TestContext::new();
        let contract = ctx.deploy("acc://test.acme/counter");
        assert!(std::panic::catch_unwind(move || {
            let _ = ctx.query(&contract, "get", &[]);
        })
        .is_err());
    }

    #[test]
    fn snapshot_restore_restores_native_fixture_state() {
        let mut ctx = TestContext::new();
        let contract = ctx.deploy("acc://test.acme/counter");
        ctx.expect_query(&contract, "get", &[], "7");
        let snap = ctx.snapshot();

        let first = ctx.query(&contract, "get", &[]);
        assert_eq!(first.return_i32(), 7);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = ctx.query(&contract, "get", &[]);
        }))
        .is_err());

        ctx.restore(snap);
        let restored = ctx.query(&contract, "get", &[]);
        assert_eq!(restored.return_i32(), 7);
    }

    #[test]
    fn return_parsers_panic_on_missing_or_invalid_data() {
        let missing = Receipt {
            tx_hash: String::new(),
            status: "success".into(),
            gas_used: 0,
            block_height: 0,
            return_data: None,
            error: None,
            events: Vec::new(),
        };
        assert!(std::panic::catch_unwind(|| missing.return_i32()).is_err());

        let invalid = QueryResult {
            return_data: "not-a-number".into(),
        };
        assert!(std::panic::catch_unwind(|| invalid.return_u64()).is_err());
    }
}
