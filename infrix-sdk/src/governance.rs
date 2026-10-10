//! Governance access for Infrix smart contracts.
//!
//! # What a contract may and may not do
//!
//! The platform decided this, and states it in code
//! (`pkg/executor/wazero_infrix_module.go`, "Object / capability write family"):
//!
//! > A contract may NOT mutate objects / capabilities directly — the spine
//! > requires those mutations to flow through a governed intent with its own
//! > plan, approval, and evidence. The production runtime refuses them so a
//! > contract cannot bypass the governance spine.
//!
//! So the rule is: **a contract may READ governance state, and may not mutate it
//! DIRECTLY.** Reads are served for real and disclosure-gated — the contract is
//! the actor, and fields it holds no grant for come back redacted. Direct writes
//! are refused, deliberately, because a mutation made inside an already-governed
//! call would have no plan, no approvals and no evidence of its own.
//!
//! It does **not** follow that a contract may not initiate governance at all,
//! and reading it that way was a mistake this module made for one day. The
//! platform mandates a channel for exactly this — [`submit_intent`], which
//! submits a CHILD INTENT that runs the full spine and therefore carries its own
//! plan, approvals and evidence. It is capability-gated on `"intent:submit"`.
//! The objection to a direct write is precisely what a child intent answers.
//!
//! # What changed, and why it was worse before
//!
//! Until 2026-09-05 this module offered sixteen functions over host imports the
//! platform **has never served**. They compiled, they were documented as
//! ordinary contract API, and every one produced a module that would not LINK —
//! so a contract author following this file's own example got a build that ran
//! nowhere, with an error naming a wasm import rather than the thing they did
//! wrong. Documented, callable, unlinkable is the worst of the three states, and
//! that is what this module was.
//!
//! Now every function here is in exactly one of three classes, and which class
//! it is in is visible at the call site:
//!
//! * **Served** — [`get_object`] and [`has_capability`] call host functions the
//!   platform really exports, and work.
//! * **Refused by doctrine** — the six DIRECT write operations. Calling one is a
//!   COMPILE ERROR that says why a direct mutation is refused and names what to
//!   do instead: [`submit_intent`].
//! * **Permitted but not served** — four reads doctrine allows and the platform
//!   does not yet export.
//! * **Refused as unboundable** — [`evaluate_trust`], whose substrate exists but
//!   whose only available signature takes a profile ref the host cannot bound to
//!   the calling contract. "Not in this shape" is a different answer from "not
//!   yet", and an implementer owed the difference. Calling one is a compile error too, but a DIFFERENT
//!   one, because "you may not" and "not yet" are different promises and an
//!   author is owed the difference.
//!
//! `pkg/executor` TestHostImportParity holds this file's host imports against
//! the module the platform actually serves, so a fourth class cannot appear.

#[cfg(feature = "alloc")]
use alloc::{string::String, vec};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::host;
use infrix_types::Error;

const MAX_OUTPUT_SIZE: usize = 65536;

// =============================================================================
// The two refusals, as compile-time diagnostics
// =============================================================================

mod refusal_seal {
    // Keep the marker traits below unimplementable outside this crate. A public
    // empty marker trait is only an advisory: a contract can implement it for a
    // local type and walk straight through the diagnostic-only API. The private
    // supertrait makes the boundary a compiler-enforced refusal instead.
    pub trait Sealed {}
}

/// Refuses a DIRECT governance mutation at compile time.
///
/// Deliberately has no implementations. A call site naming it gets the message
/// below rather than a link failure at deploy — which is what these functions
/// produced before 2026-09-05.
#[diagnostic::on_unimplemented(
    message = "a contract may not mutate governed state DIRECTLY",
    label = "refused: use governance::submit_intent, which runs the full spine",
    note = "A DIRECT mutation would be a governance act with no plan, no approvals and no evidence of its own, so the platform refuses it (pkg/executor/wazero_infrix_module.go).",
    note = "Use governance::submit_intent instead: a CHILD INTENT runs the full Intent -> Plan -> Approval -> Execution -> Outcome -> Evidence -> Anchor spine, so it has all three. Declare the \"intent:submit\" capability in your contract manifest.",
    note = "Reads are allowed too: governance::get_object, governance::query_objects, governance::has_capability."
)]
pub trait AContractMayNotInitiateGovernance: refusal_seal::Sealed {}

/// Refuses a governance READ the platform does not yet export.
///
/// Separate from [`AContractMayNotInitiateGovernance`] on purpose: doctrine
/// ALLOWS these, and the platform has simply never served them. Telling an
/// author they may not do something they may is a different lie from telling
/// them it is unavailable.
#[diagnostic::on_unimplemented(
    message = "this governance read is permitted but the platform does not serve it yet",
    label = "not implemented — this is a gap, not a prohibition",
    note = "A contract MAY read governance state; the platform's \"infrix\" host module exports no import for this one.",
    note = "Served reads today: get_object, query_objects, capability_grants, has_capability, evaluate_policy, approvals_satisfied.",
    note = "Tracked as matrix row A.ABI.GOVERNANCE.CONTRACTREADS."
)]
pub trait ThePlatformDoesNotServeThisRead: refusal_seal::Sealed {}

/// Refuses a read whose SUBSTRATE EXISTS but whose only available shape cannot
/// be bounded to the calling contract.
///
/// A third class, and it earns its place: "you may not", "not yet" and "not in
/// this shape" send an implementer to three different places. The last one is
/// the dangerous one, because the substrate is right there and adding the
/// obvious signature would open a disclosure hole.
#[diagnostic::on_unimplemented(
    message = "this governance read cannot be served in a shape that is safe",
    label = "the substrate exists; the signature does not",
    note = "Measured 2026-09-05: the platform's trust check takes an ARBITRARY profile ref and looks it up in the whole trust registry, so a contract naming one could probe any profile's existence and state.",
    note = "There is no bounded no-argument form either: ExecutionContext.TrustProfiles is an append-only AUDIT LOG of what plugins recorded relying on (empty on a plain contract call), and the step's own TrustProfileRef is a local in the plan executor that never reaches the execution context.",
    note = "Serving this needs a bounded question first — the step's own trust ref carried on the execution context — not a parameter the host cannot check. Tracked as matrix row A.ABI.GOVERNANCE.TRUSTREAD."
)]
pub trait ThisReadCannotBeBoundedToTheCallingContract: refusal_seal::Sealed {}

// =============================================================================
// Served: reads the platform really answers
// =============================================================================

/// Get a governed object by type and ID.
///
/// Returns the object as JSON bytes. **Disclosure-gated**: the contract is the
/// actor and the purpose is "operational", so fields the contract holds no
/// disclosure grant for come back redacted. That is the honest answer, not a
/// failure — a contract sees what it is entitled to see.
///
/// Backed by `host_object_read`, which the platform serves.
pub fn get_object(obj_type: &str, id: &str) -> Result<Vec<u8>, Error> {
    let type_bytes = obj_type.as_bytes();
    let id_bytes = id.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_object_read(
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

/// Query governed objects of a type, with an optional JSON filter.
///
/// Each object is per-actor disclosure-redacted, as [`get_object`] is. Backed by
/// `host_object_query`, which the platform serves — it had no wrapper here at
/// all, so a capability the platform offered was unreachable from Rust.
pub fn query_objects(obj_type: &str, filter_json: &[u8]) -> Result<Vec<u8>, Error> {
    let type_bytes = obj_type.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_object_query(
            type_bytes.as_ptr(),
            type_bytes.len() as u32,
            filter_json.as_ptr(),
            filter_json.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::GovernanceError);
    }
    output.truncate(len as usize);
    Ok(output)
}

/// The capability grants an identity holds, as JSON.
///
/// Backed by `host_capability_query`, which the platform serves.
pub fn capability_grants(identity: &str) -> Result<Vec<u8>, Error> {
    let id_bytes = identity.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_capability_query(
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

/// Whether an identity holds a capability.
///
/// Built on [`capability_grants`] rather than on a host function of its own: the
/// platform serves the grant list, and asking it a yes/no question is the
/// caller's work, not a second host import to keep in step with the first.
///
/// A read that FAILS answers `false`. That is the fail-closed reading and the
/// only safe one — a contract must never treat "I could not find out" as "yes".
pub fn has_capability(identity: &str, capability: &str) -> bool {
    let grants = match capability_grants(identity) {
        Ok(g) => g,
        Err(_) => return false,
    };
    // The grant list is JSON. A capability is present when its exact quoted
    // form appears; quoting is what stops "storage:read" matching
    // "storage:read_extended".
    let needle = {
        let mut q = String::with_capacity(capability.len() + 2);
        q.push('"');
        q.push_str(capability);
        q.push('"');
        q
    };
    contains(&grants, needle.as_bytes())
}

/// Naive substring search, so this module needs no allocator-heavy helpers and
/// no dependency beyond core.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

// =============================================================================
// Refused by doctrine: a contract may not initiate a governance write
// =============================================================================
//
// Each of these keeps the name and the shape an author would reach for, so the
// error lands on the line they wrote. The bound is what refuses it.

/// The goal type of the child intent `serialize_call_goal` describes, and the
/// one the `#[governed]` expansion submits. A named constant so the expansion and
/// the tests that check it name the same goal.
pub const CALL_GOAL_TYPE: &str = "CONTRACT_CALL";

/// Serialize the parameter map for a `CONTRACT_CALL` child intent.
///
/// The node's CONTRACT_CALL declaration requires exactly two keys: `contract`
/// (the callee) and `function` (the exported function to call). This wrote the
/// second as `method`, a key nothing reads, so every call goal built here was
/// refused at admission with `missing required param "function"` — and the
/// crate's own unit test asserted `method`, which pinned the defect instead of
/// catching it. The vendored `goal-vocabulary.json` is now the reference both the
/// tests and infrix-core's fence hold this output to.
///
/// Both values are JSON-escaped: a name or address carrying a quote or a
/// backslash stays data and cannot end the string or add a key.
///
/// Pure: builds bytes and calls nothing. Kept because the `#[governed]` macro
/// expansion uses it to describe a goal an INTENT then carries — which is the
/// supported path, and the one the refusals below point at.
pub fn serialize_call_goal(contract_address: &str, function_name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(contract_address.len() + function_name.len() + 48);
    out.extend_from_slice(b"{\"contract\":");
    push_json_string(&mut out, contract_address);
    out.extend_from_slice(b",\"function\":");
    push_json_string(&mut out, function_name);
    out.push(b'}');
    out
}

/// Appends `s` as a JSON string literal (RFC 8259 escaping).
fn push_json_string(out: &mut Vec<u8>, s: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(b"\\u00");
                out.push(HEX[((c as usize) >> 4) & 0xf]);
                out.push(HEX[(c as usize) & 0xf]);
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// Submit a CHILD INTENT — the one way a contract may initiate governance.
///
/// The child runs the full Intent -> Plan -> Approval -> Execution -> Outcome ->
/// Evidence -> Anchor spine, so it carries its own plan, its own approvals and
/// its own evidence. That is exactly what a direct mutation lacks, and exactly
/// why the direct mutations below are refused while this is not.
///
/// `goal_type` is the canonical wire string (for example `"OBJECT_CREATE"`,
/// `"ROLE_ASSIGN"`, `"SETTLEMENT"`); `params_json` is the goal's parameter map.
/// Returns the child intent's id.
///
/// The contract's manifest must declare the `"intent:submit"` capability. Without
/// it the call is refused — declared, not assumed.
///
/// Backed by `host_intent_submit`. Until 2026-09-05 the platform exported this
/// only in the legacy "intent" module, which a contract built with this SDK does
/// not import, so the one write path the doctrine mandates was unreachable.
pub fn submit_intent(goal_type: &str, params_json: &[u8]) -> Result<String, Error> {
    let goal_bytes = goal_type.as_bytes();
    let mut out = vec![0u8; MAX_INTENT_ID_SIZE];
    let len = unsafe {
        host::host_intent_submit(
            goal_bytes.as_ptr(),
            goal_bytes.len() as u32,
            params_json.as_ptr(),
            params_json.len() as u32,
            out.as_mut_ptr(),
            MAX_INTENT_ID_SIZE as u32,
        )
    };
    if len < 0 {
        return Err(Error::from_result_code(len).unwrap_or(Error::GovernanceError));
    }
    out.truncate(len as usize);
    match String::from_utf8(out) {
        Ok(id) => Ok(id),
        Err(_) => Err(Error::DecodingError),
    }
}

/// An intent id is a bounded identifier; this is the buffer the host writes it
/// into. A larger id is refused with `BufferTooSmall` rather than truncated,
/// because a truncated intent id names a different intent, or none.
const MAX_INTENT_ID_SIZE: usize = 512;

/// Create a governed object. **Refused**: see [`AContractMayNotInitiateGovernance`].
pub fn create_object(
    _obj_type: &str,
    _fields_json: impl AContractMayNotInitiateGovernance,
) -> Result<String, Error> {
    Err(Error::GovernanceError)
}

/// Transition an object's state. **Refused**: see [`AContractMayNotInitiateGovernance`].
pub fn transition_object(
    _obj_type: &str,
    _id: &str,
    _new_state: impl AContractMayNotInitiateGovernance,
) -> Result<(), Error> {
    Err(Error::GovernanceError)
}

/// Require an approval. **Refused**: see [`AContractMayNotInitiateGovernance`].
pub fn require_approval(
    _role: &str,
    _threshold: impl AContractMayNotInitiateGovernance,
) -> Result<(), Error> {
    Err(Error::GovernanceError)
}

/// Grant a capability. **Refused**: see [`AContractMayNotInitiateGovernance`].
pub fn grant_capability(
    _grantee: &str,
    _capabilities: &[&str],
    _scope: impl AContractMayNotInitiateGovernance,
) -> Result<String, Error> {
    Err(Error::GovernanceError)
}

/// Revoke a capability grant. **Refused**: see [`AContractMayNotInitiateGovernance`].
pub fn revoke_capability(_grant_id: impl AContractMayNotInitiateGovernance) -> Result<(), Error> {
    Err(Error::GovernanceError)
}

/// Assign a role. **Refused**: see [`AContractMayNotInitiateGovernance`].
pub fn assign_role(
    _identity: &str,
    _role: &str,
    _scope: impl AContractMayNotInitiateGovernance,
) -> Result<String, Error> {
    Err(Error::GovernanceError)
}

// =============================================================================
// Permitted but not served: reads the platform does not export yet
// =============================================================================

/// Read an intent's status. **Not served**: see [`ThePlatformDoesNotServeThisRead`].
pub fn get_intent_status(
    _intent_id: impl ThePlatformDoesNotServeThisRead,
) -> Result<Vec<u8>, Error> {
    Err(Error::GovernanceError)
}

/// Whether the approvals backing THE PLAN THIS CONTRACT IS EXECUTING UNDER are
/// still valid.
///
/// # Why it takes no plan id
///
/// This replaces `check_approval(plan_id)`, whose parameter was wrong twice
/// over. A contract can only truthfully be told about its own authorising
/// context — naming another plan would be the same disclosure problem
/// [`evaluate_policy`] answers by pinning the scope. And there was nothing
/// narrower to ask for anyway: the platform's freshness check re-verifies every
/// approval backing the CURRENT PLAN, and the stage identifier it accepts
/// appears only in its error message.
///
/// # What it is for
///
/// The platform runs this same check before every plan step, so a contract that
/// is running has just seen it pass. The value is WITHIN a step: an approval can
/// be revoked while a long step runs, and a contract about to do something
/// irreversible can re-ask rather than assume. It is not a way to notice a
/// denial that would otherwise be missed — execution would halt at the next step
/// boundary regardless.
///
/// # Three answers, not two
///
/// `Ok(true)` satisfied, `Ok(false)` NOT satisfied — the plan's authority is no
/// longer live — and `Err` when the question could not be answered at all. A
/// contract that collapsed the last two would read an unwired node as a revoked
/// approval; one that collapsed them the other way would read a failed lookup as
/// permission.
///
/// The contract's manifest must declare the `"approval:read"` capability.
pub fn approvals_satisfied() -> Result<bool, Error> {
    let r = unsafe { host::host_approvals_satisfied() };
    if r < 0 {
        return Err(Error::from_result_code(r).unwrap_or(Error::GovernanceError));
    }
    Ok(r == 1)
}

/// Read a trust profile. **Not served**.
pub fn get_trust_profile(
    _profile_id: impl ThePlatformDoesNotServeThisRead,
) -> Result<Vec<u8>, Error> {
    Err(Error::GovernanceError)
}

/// Evaluate a trust profile. **Refused as unboundable** — see
/// [`ThisReadCannotBeBoundedToTheCallingContract`].
///
/// Not "not yet". The substrate exists: `RuntimeTrustCheck` would answer this
/// today. What does not exist is a way to stop a contract naming a profile it
/// has no standing to ask about — the check looks up whatever ref it is given,
/// against the whole registry.
///
/// The two candidate bounds were measured and neither holds. `TrustProfiles` on
/// the execution context is an append-only audit log of what plugins recorded
/// relying on, written only on the bridge / arbitration / external paths, so on
/// a plain contract call it is EMPTY — a no-argument function over it would
/// answer "satisfied" meaning "I relied on nothing", which is worse than absent.
/// And the step's own `TrustProfileRef` is a local variable in the plan
/// executor's loop; it is never stored where a contract could reach it.
///
/// The fix is not to add the parameter. It is to carry the step's own trust ref
/// on the execution context, at which point `trust_satisfied()` takes no
/// argument and is bounded the way [`approvals_satisfied`] is.
pub fn evaluate_trust(
    _profile_id: impl ThisReadCannotBeBoundedToTheCallingContract,
) -> Result<Vec<u8>, Error> {
    Err(Error::GovernanceError)
}

/// Whether an identity holds a role. **Not served**.
pub fn has_role(_identity: &str, _role: impl ThePlatformDoesNotServeThisRead) -> bool {
    false
}

/// Read an intent's evidence. **Not served**.
pub fn get_evidence(_intent_id: impl ThePlatformDoesNotServeThisRead) -> Result<Vec<u8>, Error> {
    Err(Error::GovernanceError)
}

/// Ask the policy registry for a decision about THIS CONTRACT'S OWN scope.
///
/// Returns the decision as JSON: `allowed`, `effect`, `matchedRule`, `stratum`
/// and `auditMsg`. The evaluation TRACE is withheld — a contract acting on a
/// decision needs the verdict and the reason, not the engine's internals.
///
/// # Why it takes a policy type and nothing else
///
/// The signature used to be `evaluate_policy(scope, op_type, operands_json)`,
/// and both of the parameters it has lost were ones a contract must not supply.
///
/// * **The scope** is pinned to the calling contract by the host. A contract
///   that could name any scope would be able to probe policy it has no standing
///   to see, one decision at a time.
/// * **The operands** are not data at all. `policy::EvalContext` is an interface
///   that resolves `actor::role`, `actor::has_key`, `approval::has` and the rest
///   against the live execution. A contract supplying them would be supplying
///   ANSWERS rather than a question — it could tell the policy engine it holds a
///   role it does not.
///
/// The contract's manifest must declare the `"policy:evaluate"` capability.
///
/// A failed read is an `Err`, and it is NOT the same as a denial: a denial is an
/// `Ok` whose `allowed` is false. A contract that conflates them treats "I could
/// not find out" as "no", which is safe, or as "yes", which is not.
pub fn evaluate_policy(policy_type: &str) -> Result<Vec<u8>, Error> {
    let type_bytes = policy_type.as_bytes();
    let mut output = vec![0u8; MAX_OUTPUT_SIZE];
    let len = unsafe {
        host::host_policy_evaluate(
            type_bytes.as_ptr(),
            type_bytes.len() as u32,
            output.as_mut_ptr(),
        )
    };
    if len < 0 {
        return Err(Error::from_result_code(len).unwrap_or(Error::GovernanceError));
    }
    output.truncate(len as usize);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Matrix row A.ABI.GOVERNANCE.CONTRACTREADS.

    /// `has_capability` is built on the grant list rather than on a host
    /// function of its own, so the matching is THIS module's responsibility.
    /// Quoting is what stops a prefix matching a longer capability — without it
    /// a contract holding "storage:read_extended" would be read as holding
    /// "storage:read", which is a strictly wider permission than it was granted.
    #[test]
    fn test_capability_matching_is_exact_not_a_prefix() {
        let grants = br#"[{"capability":"storage:read_extended"},{"capability":"token:mint"}]"#;
        assert!(contains(grants, b"\"token:mint\""));
        assert!(contains(grants, b"\"storage:read_extended\""));
        assert!(
            !contains(grants, b"\"storage:read\""),
            "a prefix matched a longer capability, which would read a narrower grant as a wider one"
        );
    }

    #[test]
    fn test_contains_handles_the_degenerate_cases() {
        assert!(!contains(b"", b"x"));
        assert!(!contains(b"abc", b""));
        assert!(!contains(b"ab", b"abc"));
        assert!(contains(b"abc", b"abc"));
        assert!(contains(b"xabcx", b"abc"));
    }

    /// Off-chain (no host), the read fails and `has_capability` must answer
    /// FALSE. A contract must never treat "I could not find out" as "yes".
    #[test]
    fn test_a_failed_read_answers_false() {
        assert!(
            !has_capability("acc://alice.acme", "storage:read"),
            "a capability read that FAILED answered true; fail-closed is the only safe reading"
        );
    }

    /// The refused wrappers still return an error when reached from host code,
    /// where the compile-time bound does not apply. Belt and braces: the
    /// diagnostic stops a contract, and the value stops anything else.
    #[test]
    fn test_the_refused_writes_return_an_error() {
        struct Anything;
        impl refusal_seal::Sealed for Anything {}
        impl AContractMayNotInitiateGovernance for Anything {}
        impl ThePlatformDoesNotServeThisRead for Anything {}
        struct Unbounded;
        impl refusal_seal::Sealed for Unbounded {}
        impl ThisReadCannotBeBoundedToTheCallingContract for Unbounded {}
        assert!(create_object("Policy", Anything).is_err());
        assert!(transition_object("Policy", "p-1", Anything).is_err());
        assert!(require_approval("owner", Anything).is_err());
        assert!(grant_capability("acc://a.acme", &["storage:read"], Anything).is_err());
        assert!(revoke_capability(Anything).is_err());
        assert!(assign_role("acc://a.acme", "owner", Anything).is_err());
        assert!(get_intent_status(Anything).is_err());
        assert!(get_trust_profile(Anything).is_err());
        assert!(evaluate_trust(Unbounded).is_err());
        assert!(!has_role("acc://a.acme", Anything));
        assert!(get_evidence(Anything).is_err());
    }

    /// serialize_call_goal is pure — it builds bytes and calls nothing — which
    /// is why it survives as ordinary API while the writes do not.
    #[test]
    fn test_serialize_call_goal_is_pure_and_well_formed() {
        let out = serialize_call_goal("acc://alice.acme/token", "transfer");
        let s = core::str::from_utf8(&out).expect("utf-8");
        assert!(
            !s.contains("goal_type"),
            "the goal type is a separate host argument, not a duplicate parameter"
        );
        assert!(s.contains("acc://alice.acme/token"));
        // The node's CONTRACT_CALL key is `function`. This asserted `"method"`
        // until the key was measured against the node's declaration: it pinned a
        // key nothing reads (see wire_shape_tests for the check against the node).
        assert!(s.contains("\"function\":\"transfer\""));
        assert!(!s.contains("\"method\""));
    }

    #[test]
    fn test_governed_macro_submits_outside_a_governed_context() {
        use core::sync::atomic::{AtomicBool, Ordering};

        static BODY_RAN: AtomicBool = AtomicBool::new(false);

        #[crate::governed]
        fn child_intent_only() -> Result<(), Error> {
            BODY_RAN.store(true, Ordering::SeqCst);
            Ok(())
        }

        crate::host::set_governed_context_for_test(false);
        BODY_RAN.store(false, Ordering::SeqCst);
        assert!(
            child_intent_only().is_err(),
            "the native host refuses child-intent submission, so this call must not execute the body"
        );
        assert!(
            !BODY_RAN.load(Ordering::SeqCst),
            "an outside #[governed] call executed its body instead of submitting a child intent"
        );

        crate::host::set_governed_context_for_test(true);
        child_intent_only().expect("a governed call executes its body");
        assert!(BODY_RAN.load(Ordering::SeqCst));
        crate::host::set_governed_context_for_test(false);
    }
}

// =============================================================================
// Wire shape: what the SDK emits for a call goal, checked against the node
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod wire_shape_tests {
    use super::*;
    use serde_json::{json, Value};

    /// The node's declaration for a goal, from the vendored `goal-vocabulary.json`
    /// (generated by infrix-core from the code its admission path runs).
    fn declaration(goal: &str) -> Option<Value> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../goal-vocabulary.json");
        let raw = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read the vendored goal vocabulary {path}: {e}"));
        let doc: Value = serde_json::from_str(&raw).expect("goal-vocabulary.json is valid JSON");
        doc["goals"]
            .as_array()
            .expect("goals is a list")
            .iter()
            .find(|g| g["goal"] == goal)
            .cloned()
    }

    /// A key carries a value: present, not null, not a blank string. Mirrors the
    /// node's `FieldPresent` (pkg/intent/goal_params.go).
    fn present(params: &Value, key: &str) -> bool {
        match params.get(key) {
            None | Some(Value::Null) => false,
            Some(Value::String(s)) => !s.trim().is_empty(),
            Some(_) => true,
        }
    }

    /// Problems with `params` as the customParams of `goal`, against the node's
    /// declaration: every required group satisfied, no key the node refuses.
    fn shape_problems(goal: &str, params: &Value) -> Vec<String> {
        let Some(decl) = declaration(goal) else {
            return vec![format!("{goal} is not a goal the node accepts")];
        };
        let mut problems = Vec::new();
        for group in decl["required"].as_array().expect("required is a list") {
            let alts: Vec<&str> = group
                .as_array()
                .expect("a group is a list")
                .iter()
                .map(|a| a.as_str().expect("an alternative is a string"))
                .collect();
            let ok = alts
                .iter()
                .any(|alt| alt.split('+').all(|k| present(params, k)));
            if !ok {
                problems.push(format!(
                    "{goal}: missing required param {}",
                    if alts.len() == 1 {
                        format!("{:?}", alts[0])
                    } else {
                        format!("one of {}", alts.join(" | "))
                    }
                ));
            }
        }
        for refused in decl["refusedKeys"].as_array().expect("refusedKeys is a list") {
            let key = refused.as_str().expect("a refused key is a string");
            if params.get(key).is_some() {
                problems.push(format!("{goal}: the node refuses the key {key:?} by name"));
            }
        }
        problems
    }

    #[test]
    fn test_the_shape_check_is_not_blind() {
        // The defect this SDK shipped: the call target under "method".
        let misnamed = json!({"contract": "acc://a.acme/c", "method": "go"});
        let problems = shape_problems("CONTRACT_CALL", &misnamed);
        assert!(
            problems.iter().any(|p| p.contains("\"function\"")),
            "a call goal naming its target `method` was not flagged: {problems:?}"
        );
        assert!(!shape_problems("CONTRACT_CALL", &json!({})).is_empty());
        assert!(!shape_problems("CONTRACT_CALL", &json!({"contract": "  ", "function": "f"})).is_empty());
        assert!(shape_problems("SEND_TOKENS", &json!({}))
            .iter()
            .any(|p| p.contains("not a goal")));
        assert!(
            !shape_problems("WORKFLOW_START", &json!({"definitionId": "d", "workflowId": "w"}))
                .is_empty()
        );
        // The correct shape is not flagged, so the cases above prove something.
        assert!(shape_problems(
            "CONTRACT_CALL",
            &json!({"contract": "acc://a.acme/c", "function": "go"})
        )
        .is_empty());
    }

    #[test]
    fn test_call_goal_satisfies_the_nodes_contract_call_declaration() {
        let out = serialize_call_goal("acc://alice.acme/token", "transfer");
        let params: Value = serde_json::from_slice(&out).expect("the parameter map is valid JSON");
        let problems = shape_problems(CALL_GOAL_TYPE, &params);
        assert!(
            problems.is_empty(),
            "serialize_call_goal emits a CONTRACT_CALL the node refuses at admission: {problems:?}"
        );
        assert_eq!(params["contract"], "acc://alice.acme/token");
        assert_eq!(params["function"], "transfer");
        assert!(
            params.get("method").is_none(),
            "the node's key is `function`; `method` is read by nothing"
        );
    }

    #[test]
    fn test_call_goal_escapes_json_metacharacters() {
        // A name or address carrying JSON syntax must stay DATA: it must not end
        // the string, add a key, or produce invalid JSON.
        let contract = "acc://a.acme/\"x\\y";
        let function = "f\"\n,\"contract\":\"evil";
        let out = serialize_call_goal(contract, function);
        let params: Value =
            serde_json::from_slice(&out).expect("the parameter map must stay valid JSON");
        assert_eq!(params["contract"], contract);
        assert_eq!(params["function"], function);
        assert_eq!(
            params.as_object().expect("an object").len(),
            2,
            "an injected key appeared"
        );
    }

    #[test]
    fn test_governed_macro_emits_a_call_goal_the_node_accepts() {
        #[crate::governed]
        fn settle_up() -> Result<(), Error> {
            Ok(())
        }

        crate::host::set_governed_context_for_test(false);
        crate::host::set_self_address_for_test("acc://alice.acme/ledger");
        let _ = crate::host::last_intent_submission_for_test();
        assert!(
            settle_up().is_err(),
            "the native host refuses child-intent submission"
        );
        let (goal, params_bytes) = crate::host::last_intent_submission_for_test()
            .expect("the #[governed] expansion handed nothing to the host");
        crate::host::set_self_address_for_test("");

        assert_eq!(
            goal, CALL_GOAL_TYPE,
            "the expansion submits a different goal than it is checked as"
        );
        let params: Value =
            serde_json::from_slice(&params_bytes).expect("the parameter map is valid JSON");
        let problems = shape_problems(&goal, &params);
        assert!(
            problems.is_empty(),
            "the #[governed] expansion submits a goal the node refuses at admission: {problems:?}"
        );
        assert_eq!(params["contract"], "acc://alice.acme/ledger");
        assert_eq!(params["function"], "settle_up");

        // Record what was emitted for infrix-core's fence, which feeds this file
        // to the node's own admission validator. Refresh after an intentional
        // change with INFRIX_UPDATE_GOAL_EMISSIONS=1.
        let rendered = json!({"emissions": [{
            "sdk": "rust",
            "helper": "#[governed]",
            "goal": {"type": goal, "customParams": params},
        }]});
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goal-emissions.json");
        if std::env::var("INFRIX_UPDATE_GOAL_EMISSIONS").as_deref() == Ok("1") {
            let mut text = serde_json::to_string_pretty(&rendered).expect("serialisable");
            text.push('\n');
            std::fs::write(path, text).expect("write goal-emissions.json");
        }
        let on_disk: Value = serde_json::from_str(
            &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}")),
        )
        .expect("goal-emissions.json is valid JSON");
        assert_eq!(
            on_disk, rendered,
            "tests/goal-emissions.json is stale: re-run with INFRIX_UPDATE_GOAL_EMISSIONS=1 and let infrix-core re-check it"
        );
    }
}
