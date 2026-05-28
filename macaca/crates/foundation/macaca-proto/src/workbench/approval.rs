//! Provider-neutral contracts for `service.approval`.
//!
//! The approval service owns human or guardian approval state for privileged
//! side effects.  Shells may render prompts and submit decisions, but policy,
//! lifecycle transitions, audit records, and reviewer profile resolution stay
//! behind this service boundary.  DTOs carry summaries and references only; raw
//! command input, file content, provider payloads, prompts, and secrets must be
//! stored as protected artifacts outside approval events and audit logs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::WorkbenchServiceDescriptor;

pub const SERVICE_ID: &str = "service.approval";
pub const CAPABILITY_ID: &str = "capability.approval";
pub const REQUEST_CREATE_COMMAND: &str = "approval.request.create";
pub const REQUEST_LIST_COMMAND: &str = "approval.request.list";
pub const REQUEST_READ_COMMAND: &str = "approval.request.read";
pub const REQUEST_RESOLVE_COMMAND: &str = "approval.request.resolve";
pub const REQUEST_CANCEL_COMMAND: &str = "approval.request.cancel";
pub const REQUEST_EXPIRE_COMMAND: &str = "approval.request.expire";
pub const POLICY_EXPLAIN_COMMAND: &str = "approval.policy.explain";
pub const DECISION_AUDIT_COMMAND: &str = "approval.decision.audit";

pub const COMMANDS: &[&str] = &[
    REQUEST_CREATE_COMMAND,
    REQUEST_LIST_COMMAND,
    REQUEST_READ_COMMAND,
    REQUEST_RESOLVE_COMMAND,
    REQUEST_CANCEL_COMMAND,
    REQUEST_EXPIRE_COMMAND,
    POLICY_EXPLAIN_COMMAND,
    DECISION_AUDIT_COMMAND,
];

/// Explicit lifecycle states for replayable approval records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Cancelled,
}

/// Shell-submitted decision.  The service still validates whether the decision
/// is legal for the current state and profile before mutating the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

/// Provider-neutral side-effect class used by approval policy and audit.
///
/// The enum intentionally describes OS capability classes rather than
/// application workflows, keeping the service reusable for coding, migration,
/// data, operations, and other application families.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSideEffectClass {
    FileWrite,
    ProcessSpawn,
    GitPatch,
    PluginInstall,
    McpAuth,
    McpToolCall,
    RemoteEnvironment,
    ToolInvocation,
    Other,
}

/// Command payload for creating a new approval request before a side effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestCreate {
    pub action_summary: String,
    pub side_effect_class: ApprovalSideEffectClass,
    pub approval_profile_ref: Option<String>,
    pub reviewer_hint: Option<String>,
    pub reason_code: String,
    pub target_refs: Vec<String>,
    pub trace_refs: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: BTreeMap<String, String>,
}

/// List filters are narrow by design so callers cannot scrape raw prompts or
/// provider payloads through approval queues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestList {
    pub status: Option<ApprovalRequestStatus>,
    pub side_effect_class: Option<ApprovalSideEffectClass>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestRead {
    pub approval_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestResolve {
    pub approval_ref: String,
    pub decision: ApprovalDecision,
    pub reviewer_ref: String,
    pub reason_code: String,
    pub decision_summary: Option<String>,
    pub trace_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestCancel {
    pub approval_ref: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestExpire {
    pub approval_ref: Option<String>,
    pub now: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPolicyExplain {
    pub side_effect_class: ApprovalSideEffectClass,
    pub approval_profile_ref: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionAuditRead {
    pub approval_ref: String,
}

/// Replayable, sanitized approval state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub approval_ref: String,
    pub status: ApprovalRequestStatus,
    pub action_summary: String,
    pub side_effect_class: ApprovalSideEffectClass,
    pub approval_profile_ref: String,
    pub reviewer_class: String,
    pub reviewer_ref: Option<String>,
    pub reason_code: String,
    pub decision_summary: Option<String>,
    pub target_refs: Vec<String>,
    pub trace_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Audit memento for approval lifecycle replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalAuditRecord {
    pub audit_ref: String,
    pub approval_ref: String,
    pub action_summary: String,
    pub side_effect_class: ApprovalSideEffectClass,
    pub reviewer_class: String,
    pub decision: ApprovalRequestStatus,
    pub reason_code: String,
    pub trace_refs: Vec<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPolicyExplanation {
    pub required: bool,
    pub approval_profile_ref: String,
    pub reviewer_class: String,
    pub reason_code: String,
    pub managed_by_service: bool,
}

/// Bounded event for shell subscriptions and app-protocol notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEvent {
    pub event_ref: String,
    pub approval_ref: String,
    pub status: ApprovalRequestStatus,
    pub side_effect_class: ApprovalSideEffectClass,
    pub reviewer_class: String,
    pub reason_code: String,
    pub trace_id: String,
    pub emitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalServiceResponse {
    Created(ApprovalRecord),
    Listed {
        records: Vec<ApprovalRecord>,
        truncated: bool,
    },
    Read(ApprovalRecord),
    Resolved(ApprovalRecord),
    Cancelled(ApprovalRecord),
    Expired {
        records: Vec<ApprovalRecord>,
    },
    PolicyExplanation(ApprovalPolicyExplanation),
    DecisionAudit(ApprovalAuditRecord),
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.approval.events.v1",
        "service.approval.audit.v1",
        "service.approval.snapshot.v1",
    )
}
