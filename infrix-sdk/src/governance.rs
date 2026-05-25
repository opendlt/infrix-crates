//! Governance module for Infrix smart contracts.
//!
//! Provides safe wrappers around governance host functions, enabling
//! contracts to interact with the intent pipeline, object registry,
//! approval system, trust profiles, capabilities, roles, and evidence
//! chains from within WASM execution.
//!
//! # Example
//!
//! ```ignore
//! use infrix_sdk::governance;
//!
//! #[call]
//! pub fn transfer_governed(&self, to: Address, amount: U256) -> Result<(), Error> {
//!     // Submit a governed transfer intent from within the contract
//!     let goal = IntentGoal {
//!         goal_type: IntentGoalType::Transfer,
//!         source_assets: vec![AssetAmount { asset: "ACME".into(), amount: amount.as_u64() }],
//!         ..Default::default()
//!     };
//!     let result = governance::submit_intent(&goal)?;
//!     Ok(())
//! }
//! ```

#[cfg(feature = "alloc")]
use alloc::{string::String, vec};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::host;
use infrix_types::Error;

const MAX_OUTPUT_SIZE: usize = 65536;

// =============================================================================
// Intent Operations
// =============================================================================

/// Serialize a contract-call goal into the canonical JSON envelope
/// the runtime accepts via host_governance_submit_intent. Used by the
/// `#[governed]` proc-macro expansion to wrap a contract method call
/// in an intent submission.
pub fn serialize_call_goal(contract_address: &str, method_name: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64 + contract_address.len() + method_name.len());
    buf.extend_from_slice(b"{\"type\":\"CONTRACT_CALL\",\"contract\":\"");
    buf.extend_from_slice(contract_address.as_bytes());
    buf.extend_from_slice(b"\",\"method\":\"");
    buf.extend_from_slice(method_name.as_bytes());
    buf.extend_from_slice(b"\"}");
    buf
}

/// Submit an intent from within a contract.
///
/// The goal is serialized as JSON bytes and passed to the runtime.
/// Returns the serialized IntentResult on success.
pub fn submit_intent(goal_json: &[u8]) -> Result<Vec<u8>, Error> {
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_submit_intent(
            goal_json.as_ptr(),
            goal_json.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

/// Get the status of an intent by ID.
pub fn get_intent_status(intent_id: &str) -> Result<Vec<u8>, Error> {
    let id_bytes = intent_id.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_get_intent_status(
            id_bytes.as_ptr(),
            id_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

// =============================================================================
// Object Operations
// =============================================================================

/// Create a governed object from within a contract.
///
/// Returns the serialized object ID on success.
pub fn create_object(obj_type: &str, fields_json: &[u8]) -> Result<String, Error> {
    let type_bytes = obj_type.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_create_object(
            type_bytes.as_ptr(),
            type_bytes.len() as u32,
            fields_json.as_ptr(),
            fields_json.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    String::from_utf8(output).map_err(|_| Error::DecodingError)
}

/// Get a governed object by type and ID.
///
/// Returns the serialized object as JSON bytes.
pub fn get_object(obj_type: &str, id: &str) -> Result<Vec<u8>, Error> {
    let type_bytes = obj_type.as_bytes();
    let id_bytes = id.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_get_object(
            type_bytes.as_ptr(),
            type_bytes.len() as u32,
            id_bytes.as_ptr(),
            id_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

/// Transition an object to a new state.
pub fn transition_object(obj_type: &str, id: &str, new_state: &str) -> Result<(), Error> {
    let type_bytes = obj_type.as_bytes();
    let id_bytes = id.as_bytes();
    let state_bytes = new_state.as_bytes();
    let result = unsafe {
        host::host_governance_transition_object(
            type_bytes.as_ptr(),
            type_bytes.len() as u32,
            id_bytes.as_ptr(),
            id_bytes.len() as u32,
            state_bytes.as_ptr(),
            state_bytes.len() as u32,
        )
    };
    if result < 0 {
        return Err(Error::GovernanceError);
    }
    Ok(())
}

// =============================================================================
// Approval Operations
// =============================================================================

/// Declare that the current operation requires approval from a specific role.
///
/// If the operation has not received sufficient approvals, execution
/// is suspended until the approval threshold is met.
pub fn require_approval(role: &str, threshold: u32) -> Result<(), Error> {
    let role_bytes = role.as_bytes();
    let result = unsafe {
        host::host_governance_require_approval(
            role_bytes.as_ptr(),
            role_bytes.len() as u32,
            threshold,
        )
    };
    if result < 0 {
        return Err(Error::ApprovalRequired);
    }
    Ok(())
}

/// Check if a plan has received sufficient approvals.
pub fn check_approval(plan_id: &str) -> Result<bool, Error> {
    let id_bytes = plan_id.as_bytes();
    let result =
        unsafe { host::host_governance_check_approval(id_bytes.as_ptr(), id_bytes.len() as u32) };
    if result < 0 {
        return Err(Error::GovernanceError);
    }
    Ok(result > 0)
}

// =============================================================================
// Trust Operations
// =============================================================================

/// Get a trust profile by ID.
///
/// Returns the serialized TrustProfile as JSON bytes.
pub fn get_trust_profile(profile_id: &str) -> Result<Vec<u8>, Error> {
    let id_bytes = profile_id.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_get_trust_profile(
            id_bytes.as_ptr(),
            id_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

/// Evaluate a trust profile, returning the evaluation result as JSON bytes.
pub fn evaluate_trust(profile_id: &str) -> Result<Vec<u8>, Error> {
    let id_bytes = profile_id.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_evaluate_trust(
            id_bytes.as_ptr(),
            id_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

// =============================================================================
// Capability Operations
// =============================================================================

/// Check if the caller (or a specific identity) has a capability.
pub fn has_capability(identity: &str, capability: &str) -> bool {
    let identity_bytes = identity.as_bytes();
    let cap_bytes = capability.as_bytes();
    let result = unsafe {
        host::host_governance_has_capability(
            identity_bytes.as_ptr(),
            identity_bytes.len() as u32,
            cap_bytes.as_ptr(),
            cap_bytes.len() as u32,
        )
    };
    result > 0
}

/// Grant a capability to a grantee. Returns the grant ID.
pub fn grant_capability(
    grantee: &str,
    capabilities: &[&str],
    scope: &str,
) -> Result<String, Error> {
    let grantee_bytes = grantee.as_bytes();
    let caps_joined = capabilities.join(",");
    let caps_bytes = caps_joined.as_bytes();
    let scope_bytes = scope.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_grant_capability(
            grantee_bytes.as_ptr(),
            grantee_bytes.len() as u32,
            caps_bytes.as_ptr(),
            caps_bytes.len() as u32,
            scope_bytes.as_ptr(),
            scope_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    String::from_utf8(output).map_err(|_| Error::DecodingError)
}

/// Revoke a capability grant.
pub fn revoke_capability(grant_id: &str) -> Result<(), Error> {
    let id_bytes = grant_id.as_bytes();
    let result = unsafe {
        host::host_governance_revoke_capability(id_bytes.as_ptr(), id_bytes.len() as u32)
    };
    if result < 0 {
        return Err(Error::GovernanceError);
    }
    Ok(())
}

// =============================================================================
// Role Operations
// =============================================================================

/// Check if an identity has a specific role.
pub fn has_role(identity: &str, role: &str) -> bool {
    let identity_bytes = identity.as_bytes();
    let role_bytes = role.as_bytes();
    let result = unsafe {
        host::host_governance_has_role(
            identity_bytes.as_ptr(),
            identity_bytes.len() as u32,
            role_bytes.as_ptr(),
            role_bytes.len() as u32,
        )
    };
    result > 0
}

/// Assign a role to an identity. Returns the binding ID.
pub fn assign_role(identity: &str, role: &str, scope: &str) -> Result<String, Error> {
    let identity_bytes = identity.as_bytes();
    let role_bytes = role.as_bytes();
    let scope_bytes = scope.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_assign_role(
            identity_bytes.as_ptr(),
            identity_bytes.len() as u32,
            role_bytes.as_ptr(),
            role_bytes.len() as u32,
            scope_bytes.as_ptr(),
            scope_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    String::from_utf8(output).map_err(|_| Error::DecodingError)
}

// =============================================================================
// Evidence Operations
// =============================================================================

/// Get the evidence bundle for an intent, returned as JSON bytes.
pub fn get_evidence(intent_id: &str) -> Result<Vec<u8>, Error> {
    let id_bytes = intent_id.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_get_evidence(
            id_bytes.as_ptr(),
            id_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

// =============================================================================
// Policy Operations
// =============================================================================

/// Evaluate a policy from within a contract.
///
/// Returns the evaluation result as JSON bytes (PolicyEvaluationResult).
pub fn evaluate_policy(scope: &str, op_type: &str, operands_json: &[u8]) -> Result<Vec<u8>, Error> {
    let scope_bytes = scope.as_bytes();
    let type_bytes = op_type.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_governance_evaluate_policy(
            scope_bytes.as_ptr(),
            scope_bytes.len() as u32,
            type_bytes.as_ptr(),
            type_bytes.len() as u32,
            operands_json.as_ptr(),
            operands_json.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}
