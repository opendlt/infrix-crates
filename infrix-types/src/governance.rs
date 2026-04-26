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

/// Intent goal type categories. MUST stay in perfect parity with the
/// Go source of truth (`pkg/intent/types.go`'s `Goal*` constants and
/// `ValidGoalTypes`). The mediator dispatches by exact wire string,
/// returned here by `as_str()`. Drift is fenced by
/// `pkg/intent/sdk_goal_parity_test.go::TestSDKGoalParity_Rust`.
///
/// `Transfer` and `EscrowCreate` were removed in Gap 13 first-pass —
/// single-leg transfers and escrow creation now route through
/// `Settlement` with the appropriate method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntentGoalType {
    Convert,
    EarnYield,
    Borrow,
    ProvideLiquidity,
    Swap,
    Stake,
    Bridge,
    Compound,
    Custom,
    ObjectCreate,
    ObjectMutate,
    PolicyBind,
    CapabilityGrant,
    WorkflowStart,
    CredentialIssue,
    VaultCreate,
    Settlement,
    SettlementNetting,
    ObjectTransition,
    PolicyChange,
    ContractUpgrade,
    PatchPropagation,
    RevertTransaction,
    RoleAssign,
    RoleRevoke,
    RoleSuspend,
    RoleEmergency,
    RoleNormalize,
    DisclosureGrant,
    DisclosureRevoke,
    ContractDeploy,
    ContractCall,
    SwarmCreate,
    SwarmJoin,
    SwarmCoordinate,
    SwarmDissolve,
    ShapeTransition,
    BridgeSend,
    BridgeReceive,
    CapabilityRevoke,
    PolicyUnbind,
    AnchorForce,
    TrustProfileCreate,
    TrustProfileUpdate,
    BootstrapRole,
    SystemAnchorPeriodic,
    ApprovalInvalidate,
    RoleExpire,
    CapabilityExpire,
    SponsorRegister,
    SponsorUpdate,
    SponsorRevoke,
    SponsorPause,
    SponsorResume,
    DisputeResolve,
    RulePackEval,
    VerifierRun,
    ExternalAdapterCall,
    AgentRun,
    ConfidentialExec,
    SubsystemAction,
    /// Spec §5.3 plugin upgrade lifecycle. Mints a CompatibilityReport
    /// sized by RiskClass that drives the approval requirement.
    PluginUpgrade,
    PluginRegister,
}

impl IntentGoalType {
    /// Returns the canonical wire-format string for this goal type.
    /// Matches the string values declared in `pkg/intent/types.go`
    /// exactly. The Gap 15 cross-SDK parity fence parses these
    /// literals to verify there is no drift.
    pub fn as_str(&self) -> &'static str {
        match self {
            IntentGoalType::Convert => "CONVERT",
            IntentGoalType::EarnYield => "EARN_YIELD",
            IntentGoalType::Borrow => "BORROW",
            IntentGoalType::ProvideLiquidity => "PROVIDE_LIQUIDITY",
            IntentGoalType::Swap => "SWAP",
            IntentGoalType::Stake => "STAKE",
            IntentGoalType::Bridge => "BRIDGE",
            IntentGoalType::Compound => "COMPOUND",
            IntentGoalType::Custom => "CUSTOM",
            IntentGoalType::ObjectCreate => "OBJECT_CREATE",
            IntentGoalType::ObjectMutate => "OBJECT_MUTATE",
            IntentGoalType::PolicyBind => "POLICY_BIND",
            IntentGoalType::CapabilityGrant => "CAPABILITY_GRANT",
            IntentGoalType::WorkflowStart => "WORKFLOW_START",
            IntentGoalType::CredentialIssue => "CREDENTIAL_ISSUE",
            IntentGoalType::VaultCreate => "VAULT_CREATE",
            IntentGoalType::Settlement => "SETTLEMENT",
            IntentGoalType::SettlementNetting => "SETTLEMENT_NETTING",
            IntentGoalType::ObjectTransition => "OBJECT_TRANSITION",
            IntentGoalType::PolicyChange => "POLICY_CHANGE",
            IntentGoalType::ContractUpgrade => "CONTRACT_UPGRADE",
            IntentGoalType::PatchPropagation => "PATCH_PROPAGATION",
            IntentGoalType::RevertTransaction => "REVERT_TRANSACTION",
            IntentGoalType::RoleAssign => "ROLE_ASSIGN",
            IntentGoalType::RoleRevoke => "ROLE_REVOKE",
            IntentGoalType::RoleSuspend => "ROLE_SUSPEND",
            IntentGoalType::RoleEmergency => "ROLE_EMERGENCY",
            IntentGoalType::RoleNormalize => "ROLE_NORMALIZE",
            IntentGoalType::DisclosureGrant => "DISCLOSURE_GRANT",
            IntentGoalType::DisclosureRevoke => "DISCLOSURE_REVOKE",
            IntentGoalType::ContractDeploy => "CONTRACT_DEPLOY",
            IntentGoalType::ContractCall => "CONTRACT_CALL",
            IntentGoalType::SwarmCreate => "SWARM_CREATE",
            IntentGoalType::SwarmJoin => "SWARM_JOIN",
            IntentGoalType::SwarmCoordinate => "SWARM_COORDINATE",
            IntentGoalType::SwarmDissolve => "SWARM_DISSOLVE",
            IntentGoalType::ShapeTransition => "SHAPE_TRANSITION",
            IntentGoalType::BridgeSend => "BRIDGE_SEND",
            IntentGoalType::BridgeReceive => "BRIDGE_RECEIVE",
            IntentGoalType::CapabilityRevoke => "CAPABILITY_REVOKE",
            IntentGoalType::PolicyUnbind => "POLICY_UNBIND",
            IntentGoalType::AnchorForce => "ANCHOR_FORCE",
            IntentGoalType::TrustProfileCreate => "TRUST_PROFILE_CREATE",
            IntentGoalType::TrustProfileUpdate => "TRUST_PROFILE_UPDATE",
            IntentGoalType::BootstrapRole => "BOOTSTRAP_ROLE",
            IntentGoalType::SystemAnchorPeriodic => "SYSTEM_ANCHOR_PERIODIC",
            IntentGoalType::ApprovalInvalidate => "APPROVAL_INVALIDATE",
            IntentGoalType::RoleExpire => "ROLE_EXPIRE",
            IntentGoalType::CapabilityExpire => "CAPABILITY_EXPIRE",
            IntentGoalType::SponsorRegister => "SPONSOR_REGISTER",
            IntentGoalType::SponsorUpdate => "SPONSOR_UPDATE",
            IntentGoalType::SponsorRevoke => "SPONSOR_REVOKE",
            IntentGoalType::SponsorPause => "SPONSOR_PAUSE",
            IntentGoalType::SponsorResume => "SPONSOR_RESUME",
            IntentGoalType::DisputeResolve => "DISPUTE_RESOLVE",
            IntentGoalType::RulePackEval => "RULE_PACK_EVAL",
            IntentGoalType::VerifierRun => "VERIFIER_RUN",
            IntentGoalType::ExternalAdapterCall => "EXTERNAL_ADAPTER_CALL",
            IntentGoalType::AgentRun => "AGENT_RUN",
            IntentGoalType::ConfidentialExec => "CONFIDENTIAL_EXEC",
            IntentGoalType::SubsystemAction => "SUBSYSTEM_ACTION",
            IntentGoalType::PluginUpgrade => "PLUGIN_UPGRADE",
            IntentGoalType::PluginRegister => "PLUGIN_REGISTER",
        }
    }
}

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
