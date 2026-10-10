//! Governance type definitions for the Infrix platform.
//!
//! These types mirror the Go definitions in `pkg/intent/types.go`,
//! `pkg/workflow/plan.go`, `pkg/workflow/outcome.go`, and
//! `pkg/bridge/trust.go`.

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

// =============================================================================
// Intent Types
// =============================================================================

/// Intent goal types: exactly the node's `ValidGoalTypes` (infrix-schema
/// `intent.ValidGoalTypes`). The mediator dispatches by exact wire string,
/// returned here by `as_str()` and parsed back by `from_wire()`; `ALL` lists
/// every goal. A goal missing here cannot be named by a Rust client, and one
/// that is not the node's is refused at admission.
///
/// The enum, `ALL`, `as_str` and `from_wire` are generated, not written:
/// `tools/sdkgen` rewrites the region between the SDKGEN markers from
/// `goal-vocabulary.json` at the repo root, which infrix-core generates from the
/// code its admission path runs
/// (`go test ./pkg/release -run SDKGoalParity -count=1 -update-sdk-vocabulary`).
/// Run `go run .` in `tools/sdkgen` after refreshing that file. Three fences keep
/// this honest: `tools/sdkgen`'s tests, `infrix-types/tests/goal_vocabulary.rs`,
/// and infrix-core's `pkg/release` SDKGoalParity fence, which parses this enum
/// independently of the generator.
///
/// `Transfer` and `EscrowCreate` were removed in Gap 13 first-pass —
/// single-leg transfers and escrow creation now route through
/// `Settlement` with the appropriate method.
// SDKGEN-BEGIN(intent_goal_type)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntentGoalType {
    AgentRun,
    AnchorForce,
    ApprovalInvalidate,
    BootstrapCapability,
    BootstrapRole,
    Borrow,
    Bridge,
    BridgeReceive,
    BridgeSend,
    CapabilityExpire,
    CapabilityGrant,
    CapabilityRevoke,
    Compound,
    ConfidentialExec,
    ContractCall,
    ContractDeploy,
    ContractUpgrade,
    Convert,
    CosmwasmExec,
    CredentialIssue,
    CredentialRevoke,
    Custom,
    DisclosureApprove,
    DisclosureExpire,
    DisclosureGrant,
    DisclosureRegisterKey,
    DisclosureRevoke,
    DisputeResolve,
    EarnYield,
    EbpfExec,
    EvmCall,
    EvmDeploy,
    ExternalAdapterCall,
    GasScheduleUpdate,
    MoveExec,
    ObjectCreate,
    ObjectMutate,
    ObjectTransition,
    PatchPropagation,
    PendingReap,
    PluginRegister,
    PluginUpgrade,
    PolicyBind,
    PolicyChange,
    PolicyUnbind,
    ProvideLiquidity,
    RateLimitUpdate,
    RevertTransaction,
    RoleAssign,
    RoleEmergency,
    RoleExpire,
    RoleNormalize,
    RolePolicyRecheck,
    RoleRevoke,
    RoleSuspend,
    RulePackEval,
    SessionKeyDelegate,
    Settlement,
    SettlementFinalitySweep,
    SettlementNetting,
    ShapeTransition,
    SponsorPause,
    SponsorRegister,
    SponsorResume,
    SponsorRevoke,
    SponsorUpdate,
    Stake,
    SubsystemAction,
    SvmExec,
    Swap,
    SwarmCoordinate,
    SwarmCreate,
    SwarmDissolve,
    SwarmJoin,
    SystemAnchorPeriodic,
    TrustFreshnessSweep,
    TrustProfileCreate,
    TrustProfileUpdate,
    VaultCreate,
    VerifierRun,
    WorkflowStart,
}

impl IntentGoalType {
    /// Every goal type the node accepts, in vocabulary order.
    pub const ALL: [IntentGoalType; 81] = [
        IntentGoalType::AgentRun,
        IntentGoalType::AnchorForce,
        IntentGoalType::ApprovalInvalidate,
        IntentGoalType::BootstrapCapability,
        IntentGoalType::BootstrapRole,
        IntentGoalType::Borrow,
        IntentGoalType::Bridge,
        IntentGoalType::BridgeReceive,
        IntentGoalType::BridgeSend,
        IntentGoalType::CapabilityExpire,
        IntentGoalType::CapabilityGrant,
        IntentGoalType::CapabilityRevoke,
        IntentGoalType::Compound,
        IntentGoalType::ConfidentialExec,
        IntentGoalType::ContractCall,
        IntentGoalType::ContractDeploy,
        IntentGoalType::ContractUpgrade,
        IntentGoalType::Convert,
        IntentGoalType::CosmwasmExec,
        IntentGoalType::CredentialIssue,
        IntentGoalType::CredentialRevoke,
        IntentGoalType::Custom,
        IntentGoalType::DisclosureApprove,
        IntentGoalType::DisclosureExpire,
        IntentGoalType::DisclosureGrant,
        IntentGoalType::DisclosureRegisterKey,
        IntentGoalType::DisclosureRevoke,
        IntentGoalType::DisputeResolve,
        IntentGoalType::EarnYield,
        IntentGoalType::EbpfExec,
        IntentGoalType::EvmCall,
        IntentGoalType::EvmDeploy,
        IntentGoalType::ExternalAdapterCall,
        IntentGoalType::GasScheduleUpdate,
        IntentGoalType::MoveExec,
        IntentGoalType::ObjectCreate,
        IntentGoalType::ObjectMutate,
        IntentGoalType::ObjectTransition,
        IntentGoalType::PatchPropagation,
        IntentGoalType::PendingReap,
        IntentGoalType::PluginRegister,
        IntentGoalType::PluginUpgrade,
        IntentGoalType::PolicyBind,
        IntentGoalType::PolicyChange,
        IntentGoalType::PolicyUnbind,
        IntentGoalType::ProvideLiquidity,
        IntentGoalType::RateLimitUpdate,
        IntentGoalType::RevertTransaction,
        IntentGoalType::RoleAssign,
        IntentGoalType::RoleEmergency,
        IntentGoalType::RoleExpire,
        IntentGoalType::RoleNormalize,
        IntentGoalType::RolePolicyRecheck,
        IntentGoalType::RoleRevoke,
        IntentGoalType::RoleSuspend,
        IntentGoalType::RulePackEval,
        IntentGoalType::SessionKeyDelegate,
        IntentGoalType::Settlement,
        IntentGoalType::SettlementFinalitySweep,
        IntentGoalType::SettlementNetting,
        IntentGoalType::ShapeTransition,
        IntentGoalType::SponsorPause,
        IntentGoalType::SponsorRegister,
        IntentGoalType::SponsorResume,
        IntentGoalType::SponsorRevoke,
        IntentGoalType::SponsorUpdate,
        IntentGoalType::Stake,
        IntentGoalType::SubsystemAction,
        IntentGoalType::SvmExec,
        IntentGoalType::Swap,
        IntentGoalType::SwarmCoordinate,
        IntentGoalType::SwarmCreate,
        IntentGoalType::SwarmDissolve,
        IntentGoalType::SwarmJoin,
        IntentGoalType::SystemAnchorPeriodic,
        IntentGoalType::TrustFreshnessSweep,
        IntentGoalType::TrustProfileCreate,
        IntentGoalType::TrustProfileUpdate,
        IntentGoalType::VaultCreate,
        IntentGoalType::VerifierRun,
        IntentGoalType::WorkflowStart,
    ];

    /// Returns the canonical wire-format string for this goal type: exactly the
    /// node's `ValidGoalTypes` entry. Generated from the vendored
    /// `goal-vocabulary.json` — do not hand-edit; run `go run .` in
    /// tools/sdkgen after that file is refreshed from infrix-core.
    pub fn as_str(&self) -> &'static str {
        match self {
            IntentGoalType::AgentRun => "AGENT_RUN",
            IntentGoalType::AnchorForce => "ANCHOR_FORCE",
            IntentGoalType::ApprovalInvalidate => "APPROVAL_INVALIDATE",
            IntentGoalType::BootstrapCapability => "BOOTSTRAP_CAPABILITY",
            IntentGoalType::BootstrapRole => "BOOTSTRAP_ROLE",
            IntentGoalType::Borrow => "BORROW",
            IntentGoalType::Bridge => "BRIDGE",
            IntentGoalType::BridgeReceive => "BRIDGE_RECEIVE",
            IntentGoalType::BridgeSend => "BRIDGE_SEND",
            IntentGoalType::CapabilityExpire => "CAPABILITY_EXPIRE",
            IntentGoalType::CapabilityGrant => "CAPABILITY_GRANT",
            IntentGoalType::CapabilityRevoke => "CAPABILITY_REVOKE",
            IntentGoalType::Compound => "COMPOUND",
            IntentGoalType::ConfidentialExec => "CONFIDENTIAL_EXEC",
            IntentGoalType::ContractCall => "CONTRACT_CALL",
            IntentGoalType::ContractDeploy => "CONTRACT_DEPLOY",
            IntentGoalType::ContractUpgrade => "CONTRACT_UPGRADE",
            IntentGoalType::Convert => "CONVERT",
            IntentGoalType::CosmwasmExec => "COSMWASM_EXEC",
            IntentGoalType::CredentialIssue => "CREDENTIAL_ISSUE",
            IntentGoalType::CredentialRevoke => "CREDENTIAL_REVOKE",
            IntentGoalType::Custom => "CUSTOM",
            IntentGoalType::DisclosureApprove => "DISCLOSURE_APPROVE",
            IntentGoalType::DisclosureExpire => "DISCLOSURE_EXPIRE",
            IntentGoalType::DisclosureGrant => "DISCLOSURE_GRANT",
            IntentGoalType::DisclosureRegisterKey => "DISCLOSURE_REGISTER_KEY",
            IntentGoalType::DisclosureRevoke => "DISCLOSURE_REVOKE",
            IntentGoalType::DisputeResolve => "DISPUTE_RESOLVE",
            IntentGoalType::EarnYield => "EARN_YIELD",
            IntentGoalType::EbpfExec => "EBPF_EXEC",
            IntentGoalType::EvmCall => "EVM_CALL",
            IntentGoalType::EvmDeploy => "EVM_DEPLOY",
            IntentGoalType::ExternalAdapterCall => "EXTERNAL_ADAPTER_CALL",
            IntentGoalType::GasScheduleUpdate => "GAS_SCHEDULE_UPDATE",
            IntentGoalType::MoveExec => "MOVE_EXEC",
            IntentGoalType::ObjectCreate => "OBJECT_CREATE",
            IntentGoalType::ObjectMutate => "OBJECT_MUTATE",
            IntentGoalType::ObjectTransition => "OBJECT_TRANSITION",
            IntentGoalType::PatchPropagation => "PATCH_PROPAGATION",
            IntentGoalType::PendingReap => "PENDING_REAP",
            IntentGoalType::PluginRegister => "PLUGIN_REGISTER",
            IntentGoalType::PluginUpgrade => "PLUGIN_UPGRADE",
            IntentGoalType::PolicyBind => "POLICY_BIND",
            IntentGoalType::PolicyChange => "POLICY_CHANGE",
            IntentGoalType::PolicyUnbind => "POLICY_UNBIND",
            IntentGoalType::ProvideLiquidity => "PROVIDE_LIQUIDITY",
            IntentGoalType::RateLimitUpdate => "RATE_LIMIT_UPDATE",
            IntentGoalType::RevertTransaction => "REVERT_TRANSACTION",
            IntentGoalType::RoleAssign => "ROLE_ASSIGN",
            IntentGoalType::RoleEmergency => "ROLE_EMERGENCY",
            IntentGoalType::RoleExpire => "ROLE_EXPIRE",
            IntentGoalType::RoleNormalize => "ROLE_NORMALIZE",
            IntentGoalType::RolePolicyRecheck => "ROLE_POLICY_RECHECK",
            IntentGoalType::RoleRevoke => "ROLE_REVOKE",
            IntentGoalType::RoleSuspend => "ROLE_SUSPEND",
            IntentGoalType::RulePackEval => "RULE_PACK_EVAL",
            IntentGoalType::SessionKeyDelegate => "SESSION_KEY_DELEGATE",
            IntentGoalType::Settlement => "SETTLEMENT",
            IntentGoalType::SettlementFinalitySweep => "SETTLEMENT_FINALITY_SWEEP",
            IntentGoalType::SettlementNetting => "SETTLEMENT_NETTING",
            IntentGoalType::ShapeTransition => "SHAPE_TRANSITION",
            IntentGoalType::SponsorPause => "SPONSOR_PAUSE",
            IntentGoalType::SponsorRegister => "SPONSOR_REGISTER",
            IntentGoalType::SponsorResume => "SPONSOR_RESUME",
            IntentGoalType::SponsorRevoke => "SPONSOR_REVOKE",
            IntentGoalType::SponsorUpdate => "SPONSOR_UPDATE",
            IntentGoalType::Stake => "STAKE",
            IntentGoalType::SubsystemAction => "SUBSYSTEM_ACTION",
            IntentGoalType::SvmExec => "SVM_EXEC",
            IntentGoalType::Swap => "SWAP",
            IntentGoalType::SwarmCoordinate => "SWARM_COORDINATE",
            IntentGoalType::SwarmCreate => "SWARM_CREATE",
            IntentGoalType::SwarmDissolve => "SWARM_DISSOLVE",
            IntentGoalType::SwarmJoin => "SWARM_JOIN",
            IntentGoalType::SystemAnchorPeriodic => "SYSTEM_ANCHOR_PERIODIC",
            IntentGoalType::TrustFreshnessSweep => "TRUST_FRESHNESS_SWEEP",
            IntentGoalType::TrustProfileCreate => "TRUST_PROFILE_CREATE",
            IntentGoalType::TrustProfileUpdate => "TRUST_PROFILE_UPDATE",
            IntentGoalType::VaultCreate => "VAULT_CREATE",
            IntentGoalType::VerifierRun => "VERIFIER_RUN",
            IntentGoalType::WorkflowStart => "WORKFLOW_START",
        }
    }

    /// Parses a wire-format string back to a goal type; `None` for anything the
    /// node does not accept.
    pub fn from_wire(wire: &str) -> Option<IntentGoalType> {
        match wire {
            "AGENT_RUN" => Some(IntentGoalType::AgentRun),
            "ANCHOR_FORCE" => Some(IntentGoalType::AnchorForce),
            "APPROVAL_INVALIDATE" => Some(IntentGoalType::ApprovalInvalidate),
            "BOOTSTRAP_CAPABILITY" => Some(IntentGoalType::BootstrapCapability),
            "BOOTSTRAP_ROLE" => Some(IntentGoalType::BootstrapRole),
            "BORROW" => Some(IntentGoalType::Borrow),
            "BRIDGE" => Some(IntentGoalType::Bridge),
            "BRIDGE_RECEIVE" => Some(IntentGoalType::BridgeReceive),
            "BRIDGE_SEND" => Some(IntentGoalType::BridgeSend),
            "CAPABILITY_EXPIRE" => Some(IntentGoalType::CapabilityExpire),
            "CAPABILITY_GRANT" => Some(IntentGoalType::CapabilityGrant),
            "CAPABILITY_REVOKE" => Some(IntentGoalType::CapabilityRevoke),
            "COMPOUND" => Some(IntentGoalType::Compound),
            "CONFIDENTIAL_EXEC" => Some(IntentGoalType::ConfidentialExec),
            "CONTRACT_CALL" => Some(IntentGoalType::ContractCall),
            "CONTRACT_DEPLOY" => Some(IntentGoalType::ContractDeploy),
            "CONTRACT_UPGRADE" => Some(IntentGoalType::ContractUpgrade),
            "CONVERT" => Some(IntentGoalType::Convert),
            "COSMWASM_EXEC" => Some(IntentGoalType::CosmwasmExec),
            "CREDENTIAL_ISSUE" => Some(IntentGoalType::CredentialIssue),
            "CREDENTIAL_REVOKE" => Some(IntentGoalType::CredentialRevoke),
            "CUSTOM" => Some(IntentGoalType::Custom),
            "DISCLOSURE_APPROVE" => Some(IntentGoalType::DisclosureApprove),
            "DISCLOSURE_EXPIRE" => Some(IntentGoalType::DisclosureExpire),
            "DISCLOSURE_GRANT" => Some(IntentGoalType::DisclosureGrant),
            "DISCLOSURE_REGISTER_KEY" => Some(IntentGoalType::DisclosureRegisterKey),
            "DISCLOSURE_REVOKE" => Some(IntentGoalType::DisclosureRevoke),
            "DISPUTE_RESOLVE" => Some(IntentGoalType::DisputeResolve),
            "EARN_YIELD" => Some(IntentGoalType::EarnYield),
            "EBPF_EXEC" => Some(IntentGoalType::EbpfExec),
            "EVM_CALL" => Some(IntentGoalType::EvmCall),
            "EVM_DEPLOY" => Some(IntentGoalType::EvmDeploy),
            "EXTERNAL_ADAPTER_CALL" => Some(IntentGoalType::ExternalAdapterCall),
            "GAS_SCHEDULE_UPDATE" => Some(IntentGoalType::GasScheduleUpdate),
            "MOVE_EXEC" => Some(IntentGoalType::MoveExec),
            "OBJECT_CREATE" => Some(IntentGoalType::ObjectCreate),
            "OBJECT_MUTATE" => Some(IntentGoalType::ObjectMutate),
            "OBJECT_TRANSITION" => Some(IntentGoalType::ObjectTransition),
            "PATCH_PROPAGATION" => Some(IntentGoalType::PatchPropagation),
            "PENDING_REAP" => Some(IntentGoalType::PendingReap),
            "PLUGIN_REGISTER" => Some(IntentGoalType::PluginRegister),
            "PLUGIN_UPGRADE" => Some(IntentGoalType::PluginUpgrade),
            "POLICY_BIND" => Some(IntentGoalType::PolicyBind),
            "POLICY_CHANGE" => Some(IntentGoalType::PolicyChange),
            "POLICY_UNBIND" => Some(IntentGoalType::PolicyUnbind),
            "PROVIDE_LIQUIDITY" => Some(IntentGoalType::ProvideLiquidity),
            "RATE_LIMIT_UPDATE" => Some(IntentGoalType::RateLimitUpdate),
            "REVERT_TRANSACTION" => Some(IntentGoalType::RevertTransaction),
            "ROLE_ASSIGN" => Some(IntentGoalType::RoleAssign),
            "ROLE_EMERGENCY" => Some(IntentGoalType::RoleEmergency),
            "ROLE_EXPIRE" => Some(IntentGoalType::RoleExpire),
            "ROLE_NORMALIZE" => Some(IntentGoalType::RoleNormalize),
            "ROLE_POLICY_RECHECK" => Some(IntentGoalType::RolePolicyRecheck),
            "ROLE_REVOKE" => Some(IntentGoalType::RoleRevoke),
            "ROLE_SUSPEND" => Some(IntentGoalType::RoleSuspend),
            "RULE_PACK_EVAL" => Some(IntentGoalType::RulePackEval),
            "SESSION_KEY_DELEGATE" => Some(IntentGoalType::SessionKeyDelegate),
            "SETTLEMENT" => Some(IntentGoalType::Settlement),
            "SETTLEMENT_FINALITY_SWEEP" => Some(IntentGoalType::SettlementFinalitySweep),
            "SETTLEMENT_NETTING" => Some(IntentGoalType::SettlementNetting),
            "SHAPE_TRANSITION" => Some(IntentGoalType::ShapeTransition),
            "SPONSOR_PAUSE" => Some(IntentGoalType::SponsorPause),
            "SPONSOR_REGISTER" => Some(IntentGoalType::SponsorRegister),
            "SPONSOR_RESUME" => Some(IntentGoalType::SponsorResume),
            "SPONSOR_REVOKE" => Some(IntentGoalType::SponsorRevoke),
            "SPONSOR_UPDATE" => Some(IntentGoalType::SponsorUpdate),
            "STAKE" => Some(IntentGoalType::Stake),
            "SUBSYSTEM_ACTION" => Some(IntentGoalType::SubsystemAction),
            "SVM_EXEC" => Some(IntentGoalType::SvmExec),
            "SWAP" => Some(IntentGoalType::Swap),
            "SWARM_COORDINATE" => Some(IntentGoalType::SwarmCoordinate),
            "SWARM_CREATE" => Some(IntentGoalType::SwarmCreate),
            "SWARM_DISSOLVE" => Some(IntentGoalType::SwarmDissolve),
            "SWARM_JOIN" => Some(IntentGoalType::SwarmJoin),
            "SYSTEM_ANCHOR_PERIODIC" => Some(IntentGoalType::SystemAnchorPeriodic),
            "TRUST_FRESHNESS_SWEEP" => Some(IntentGoalType::TrustFreshnessSweep),
            "TRUST_PROFILE_CREATE" => Some(IntentGoalType::TrustProfileCreate),
            "TRUST_PROFILE_UPDATE" => Some(IntentGoalType::TrustProfileUpdate),
            "VAULT_CREATE" => Some(IntentGoalType::VaultCreate),
            "VERIFIER_RUN" => Some(IntentGoalType::VerifierRun),
            "WORKFLOW_START" => Some(IntentGoalType::WorkflowStart),
            _ => None,
        }
    }
}
// SDKGEN-END(intent_goal_type)

/// The desired outcome of an intent.
#[derive(Clone, Debug)]
pub struct IntentGoal {
    pub goal_type: IntentGoalType,
    pub source_assets: Vec<AssetAmount>,
    pub target_assets: Vec<AssetAmount>,
    pub target_state: Option<TargetStateSpec>,
    pub via: Option<String>,
    pub custom_type: Option<String>,
}

impl Default for IntentGoal {
    fn default() -> Self {
        Self {
            goal_type: IntentGoalType::Custom,
            source_assets: Vec::new(),
            target_assets: Vec::new(),
            target_state: None,
            via: None,
            custom_type: None,
        }
    }
}

/// Asset and quantity.
#[derive(Clone, Debug)]
pub struct AssetAmount {
    pub asset: String,
    pub amount: u64,
    pub amount_decimal: Option<String>,
    pub is_minimum: bool,
    pub is_maximum: bool,
}

/// Desired on-chain state.
#[derive(Clone, Debug)]
pub struct TargetStateSpec {
    pub state_type: String,
    pub parameters: Vec<(String, String)>,
    pub contract: Option<String>,
}

/// Current status of an intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntentStatus {
    Pending,
    Planning,
    AwaitingApproval,
    Executing,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

/// Result returned from intent submission.
#[derive(Clone, Debug)]
pub struct IntentResult {
    pub intent_id: String,
    pub status: IntentStatus,
    pub plan_id: Option<String>,
    pub outcome_id: Option<String>,
    pub gas_used: Option<u64>,
    pub error: Option<String>,
}

// =============================================================================
// Execution Plan Types
// =============================================================================

/// Pre-execution plan generated from an intent.
#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub id: String,
    pub steps: Vec<PlanStep>,
    pub plan_hash: [u8; 32],
    pub total_gas_estimate: u64,
    pub required_approvals: Vec<PlanApprovalReq>,
    pub deadline: Option<u64>,
    pub drift_threshold: Option<f64>,
    pub trust_assumptions: Vec<String>,
    pub compensation_plan: Vec<PlanStep>,
}

/// Canonical spine stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpineStage {
    Intent,
    Plan,
    Approval,
    Execution,
    Outcome,
    Evidence,
    Anchor,
}

/// A single step in the execution plan.
#[derive(Clone, Debug)]
pub struct PlanStep {
    pub stage_id: String,
    pub stage_name: String,
    /// Canonical spine stage this step represents.
    pub spine_stage: SpineStage,
    pub step_type: PlanStepType,
    pub description: String,
    pub gas_estimate: u64,
    pub policy_condition: Option<String>,
    pub execution_target: Option<String>,
    pub depends_on: Vec<String>,
    pub expected_output: Option<String>,
}

/// Plan step types matching Go PlanStepType constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanStepType {
    ContractCall,
    ContractDeploy,
    ObjectCreate,
    ObjectMutate,
    ObjectTransition,
    PolicyCheck,
    ApprovalGate,
    Settlement,
    BridgeAction,
    ExternalProof,
    Wait,
    Compensate,
    SwarmAction,
    Anchor,
    // Legacy/extended types retained for SDK completeness.
    SettlementLeg,
    EscrowCreate,
    EscrowRelease,
    CapabilityGrant,
    CapabilityRevoke,
    RoleAssign,
    RoleRevoke,
    PolicyEvaluate,
    ApprovalCheckpoint,
    EvidenceAnchor,
    L0Transfer,
    L0DataWrite,
    Compensation,
}

/// Approval requirement within a plan.
#[derive(Clone, Debug)]
pub struct PlanApprovalReq {
    pub stage_id: String,
    pub roles: Vec<String>,
    pub identities: Vec<String>,
    pub threshold: u32,
}

// =============================================================================
// Outcome Types
// =============================================================================

/// Outcome finality states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutcomeFinality {
    Provisional,
    LocallyFinal,
    ExternalContingent,
    Compensated,
    Disputed,
    L0AnchoredFinal,
}

/// Post-execution outcome record.
#[derive(Clone, Debug)]
pub struct OutcomeRecord {
    pub id: String,
    pub plan_id: String,
    pub overall_status: String,
    pub step_outcomes: Vec<StepOutcome>,
    pub total_gas_used: u64,
    pub total_gas_planned: u64,
    pub gas_drift: f64,
    pub drift_analysis: Option<DriftAnalysis>,
    pub outcome_hash: [u8; 32],
    pub plan_hash_verified: bool,
    /// Finality state of this outcome.
    pub finality: OutcomeFinality,
}

/// Actual result for a single plan step.
#[derive(Clone, Debug)]
pub struct StepOutcome {
    pub stage_id: String,
    pub planned_gas: u64,
    pub actual_gas: u64,
    pub gas_drift: f64,
    pub status: String,
    pub error: Option<String>,
    pub output_hash: Option<[u8; 32]>,
}

/// How far execution diverged from the plan.
#[derive(Clone, Debug)]
pub struct DriftAnalysis {
    pub exceeded_threshold: bool,
    pub max_step_drift: f64,
    pub drifting_steps: Vec<String>,
    pub summary: String,
}

// =============================================================================
// Approval Types
// =============================================================================

/// A signed approval envelope.
#[derive(Clone, Debug)]
pub struct ApprovalEnvelope {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub plan_hash: [u8; 32],
    pub signer_identity: String,
    pub signer_role: Option<String>,
    pub signature: Option<Vec<u8>>,
    pub status: String,
}

// =============================================================================
// Trust Types
// =============================================================================

/// Trust profile for a bridge adapter.
#[derive(Clone, Debug)]
pub struct TrustProfile {
    pub id: String,
    pub source_domain: String,
    pub chain_id: Option<String>,
    pub proof_type: String,
    pub trust_assumption: String,
    pub finality_model: String,
    pub min_confirmations: Option<u64>,
    pub validator_set_size: Option<u32>,
    pub quorum_threshold: Option<String>,
}

/// Result of evaluating a trust profile.
#[derive(Clone, Debug)]
pub struct TrustEvaluation {
    pub profile_id: String,
    pub passed: bool,
    pub checks: Vec<TrustCheck>,
}

/// Individual trust check result.
#[derive(Clone, Debug)]
pub struct TrustCheck {
    pub requirement: String,
    pub actual: String,
    pub passed: bool,
}

/// Bridge proof types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeProofType {
    MerkleInclusion,
    ValidatorSigned,
    LightClient,
    ZkBridge,
    Optimistic,
    Oracle,
}

/// Trust assumptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrustAssumption {
    Trustless,
    PowMajority,
    HonestMajority,
    BftQuorum,
    HonestMinority,
    SingleHonest,
    Optimistic,
    TrustedOperator,
    TrustedOracle,
    Cryptographic,
    EconomicSecurity,
}

// =============================================================================
// Capability Types
// =============================================================================

/// A capability grant.
#[derive(Clone, Debug)]
pub struct CapabilityGrant {
    pub id: String,
    pub grantor: String,
    pub grantee: String,
    pub capabilities: Vec<String>,
    pub scope: String,
    pub status: String,
}

// =============================================================================
// Role Types
// =============================================================================

/// A role binding.
#[derive(Clone, Debug)]
pub struct RoleBinding {
    pub id: String,
    pub holder_identity: String,
    pub role_name: String,
    pub scope_type: String,
    pub scope_target: String,
    pub status: String,
}

// =============================================================================
// Settlement Types
// =============================================================================

/// Settlement instruction.
#[derive(Clone, Debug)]
pub struct SettlementInstruction {
    pub id: String,
    pub legs: Vec<SettlementLeg>,
    pub status: String,
    pub required_approvals: u32,
    pub current_approvals: u32,
}

/// A single leg of a settlement.
#[derive(Clone, Debug)]
pub struct SettlementLeg {
    pub leg_id: String,
    pub from_account: String,
    pub to_account: String,
    pub asset: String,
    pub amount: u64,
    pub sequence: u32,
}

// =============================================================================
// Escrow Types
// =============================================================================

/// An escrow.
#[derive(Clone, Debug)]
pub struct Escrow {
    pub id: String,
    pub depositor: String,
    pub beneficiary: String,
    pub asset: String,
    pub amount: u64,
    pub status: String,
}

// =============================================================================
// Evidence Types
// =============================================================================

/// Evidence bundle.
#[derive(Clone, Debug)]
pub struct EvidenceBundle {
    pub id: String,
    pub intent_id: String,
    pub plan_id: String,
    pub chain_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub anchor_status: String,
}

/// Anchored record.
#[derive(Clone, Debug)]
pub struct AnchoredRecord {
    pub id: String,
    pub artifact_type: String,
    pub artifact_hash: [u8; 32],
    pub l0_tx_hash: [u8; 32],
    pub l0_data_account: String,
    pub status: String,
}

// =============================================================================
// Disclosure Types
// =============================================================================

/// Disclosure grant.
#[derive(Clone, Debug)]
pub struct DisclosureGrant {
    pub id: String,
    pub grantor_identity: String,
    pub grantee_identity: String,
    pub target_type: String,
    pub target_id: String,
    pub disclosed_fields: Vec<String>,
    pub status: String,
}

// =============================================================================
// Policy Types
// =============================================================================

/// Result of evaluating a policy.
#[derive(Clone, Debug)]
pub struct PolicyEvaluationResult {
    pub allowed: bool,
    pub matched_rules: Vec<PolicyDecision>,
    pub denied_by: Option<String>,
    pub requires_approval: bool,
    pub required_roles: Vec<String>,
}

/// A single policy decision.
#[derive(Clone, Debug)]
pub struct PolicyDecision {
    pub rule_id: String,
    pub rule_name: String,
    pub action: String,
    pub result: String,
    pub evaluation_ms: u64,
}

/// A single link in an evidence chain.
#[derive(Clone, Debug)]
pub struct EvidenceLink {
    pub sequence: u32,
    pub link_type: String,
    pub content_hash: [u8; 32],
    pub prev_hash: [u8; 32],
    pub timestamp: String,
    pub stage_id: Option<String>,
}

/// Approval reference within an outcome.
#[derive(Clone, Debug)]
pub struct ApprovalRef {
    pub stage_id: String,
    pub identity: String,
    pub role: String,
    pub plan_hash: [u8; 32],
    pub signed_at: String,
}

// =============================================================================
// Gap 15: Cross-cutting governance types
// =============================================================================

/// Anchor class — classifies the anchoring treatment for an artifact type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorClass {
    NoAnchor,
    DigestOnly,
    Batch,
    Full,
}

/// Privacy class — disclosure privacy classification for object fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrivacyClass {
    Public,
    Internal,
    Confidential,
    Restricted,
    Secret,
    NeverDisclosable,
    ZkpOnly,
}

/// Settlement method — how value is moved in a settlement instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettlementMethod {
    Atomic,
    Dvp,
    Phased,
    Netting,
    Bridge,
    Escrow,
    Regulated,
}

/// Execution family — the category of execution runtime for a plan step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionFamily {
    Wasm,
    ObjectOp,
    Settlement,
    Bridge,
    ApprovalGate,
    PolicyCheck,
    DisclosureAction,
    SwarmAction,
    Anchor,
    Wait,
    ExternalProof,
    RulePack,
    VerifierPlugin,
    ExternalAdapter,
    AgentModule,
    Confidential,
}

/// Trust response action — deterministic downstream effect of trust drift.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrustResponseAction {
    PausePlan,
    InvalidateApproval,
    DowngradeEvidence,
    BlockFinality,
}
