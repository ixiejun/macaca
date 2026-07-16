use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::workflow_common::{
    define_workflow_command_wrappers, workflow_pack_definition, workflow_stable_hash,
    WorkflowCommandEnvelope, WorkflowError, WorkflowPackDescriptor, WorkflowPage,
    WorkflowProviderClass,
};

pub const WORKFLOW_APPROVAL_PACK_ID: &str = "pack.workflow.approval.v1";
pub const WORKFLOW_APPROVAL_SERVICE_ID: &str = "service.workflow.approval";

pub const WORKFLOW_APPROVAL_COMMANDS: &[&str] = &[
    "approval.request_approval",
    "approval.record_decision",
    "approval.escalate",
    "approval.cancel_approval",
    "approval.inspect_evidence",
    "approval.list_pending",
    "approval.evaluate_gate",
    "approval.inspect_provider",
];

const APPROVAL_PERMISSION_SCOPES: &[&str] = &[
    "workflow.approval.request",
    "workflow.approval.decide",
    "workflow.approval.escalate",
    "workflow.approval.read",
    "workflow.approval.admin",
];

const DURABLE_METADATA: &[(&str, &str)] = &[
    ("decision_records", "true"),
    ("assignment_history", "bounded"),
    ("raw_evidence_in_trace", "false"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_workflow", "true"),
    ("eligibility_recheck", "required"),
    ("provider_payloads_in_trace", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] = &[
    ("plugin", "true"),
    ("decision_gate_conformance", "required"),
];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("approvals", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const APPROVAL_PROVIDER_CLASSES: &[WorkflowProviderClass<'_>] = &[
    WorkflowProviderClass {
        provider_class: "durable-approval",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DURABLE_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "remote-workflow-engine",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REMOTE_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "plugin",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLUGIN_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the approval descriptor without binding a human workflow or form provider.
pub fn workflow_approval_pack_definition() -> DomainPackDefinition {
    workflow_pack_definition(WorkflowPackDescriptor {
        pack_id: WORKFLOW_APPROVAL_PACK_ID,
        child_change_id: "openspec:add-pack-workflow-approval",
        docs_slug: "approval",
        sdk_slug: "approval",
        service_id: WORKFLOW_APPROVAL_SERVICE_ID,
        commands: WORKFLOW_APPROVAL_COMMANDS,
        permission_scopes: APPROVAL_PERMISSION_SCOPES,
        provider_classes: APPROVAL_PROVIDER_CLASSES,
        health_probe: "approval.inspect_provider",
        unavailable_reason: "workflow_approval_provider_not_installed",
        replay_schema: "workflow.approval.replay.v1",
        data_classification: "workflow_approval_reference_metadata",
        retention_policy: "approval_request_assignment_decision_evidence_gate_and_escalation_metadata_by_reference",
        redaction_policy: "raw_evidence_prompts_identity_payloads_provider_payloads_credentials_and_unbounded_comments_redacted",
        timeout_ms: 180_000,
        budget_units: 8,
        examples: &[
            "Declare `pack.workflow.approval.v1` as optional until an approval provider is installed.",
            "Use approval request, assignment, decision, evidence, and gate references instead of raw form payloads.",
        ],
        migration_notes: &[
            "Approval commands become callable only after an approved approval service provider registers matching schemas.",
            "Protected side-effect services verify decision gates through service calls; shells only render approval state.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_ref: String,
    pub subject_ref: String,
    pub policy_hash: String,
    pub requester_ref: String,
    pub state: String,
    pub deadline_epoch_ms: Option<u64>,
    pub schema_version: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalAssignment {
    pub assignment_ref: String,
    pub request_ref: String,
    pub eligible_principal_refs: Vec<String>,
    pub claimed_by_ref: Option<String>,
    pub escalated_from_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub decision_ref: String,
    pub request_ref: String,
    pub approver_ref: String,
    pub outcome: String,
    pub reason_ref: Option<String>,
    pub consumed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEvidenceBundle {
    pub evidence_ref: String,
    pub request_ref: String,
    pub evidence_hash: String,
    pub source_trace_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionGate {
    pub gate_ref: String,
    pub request_ref: String,
    pub required_outcome: String,
    pub decision_ref: Option<String>,
    pub valid_until_epoch_ms: Option<u64>,
    pub consumption_mode: String,
}

impl ApprovalDecisionGate {
    /// Downstream side effects must link to a concrete decision before use.
    pub fn is_satisfied(&self) -> bool {
        self.decision_ref.is_some() && !self.required_outcome.is_empty()
    }
}

pub type WorkflowApprovalError = WorkflowError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowApprovalResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Failure,
    Expired,
    Cancelled,
    EligibilityRevoked,
    DuplicateDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowApprovalResultEnvelope<T> {
    pub status: WorkflowApprovalResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WorkflowPage<T>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowApprovalError>,
}

define_workflow_command_wrappers!(
    ApprovalRequestApprovalCommand,
    ApprovalRecordDecisionCommand,
    ApprovalEscalateCommand,
    ApprovalCancelApprovalCommand,
    ApprovalInspectEvidenceCommand,
    ApprovalListPendingCommand,
    ApprovalEvaluateGateCommand,
    ApprovalInspectProviderCommand,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowApprovalDescriptorHashes {
    pub descriptor_hash: String,
    pub commands_hash: String,
    pub permissions_hash: String,
    pub providers_hash: String,
    pub request_hash: String,
    pub decision_gate_hash: String,
}

pub fn workflow_approval_descriptor_hashes() -> WorkflowApprovalDescriptorHashes {
    WorkflowApprovalDescriptorHashes {
        descriptor_hash: workflow_stable_hash(&workflow_approval_pack_definition()),
        commands_hash: workflow_stable_hash(WORKFLOW_APPROVAL_COMMANDS),
        permissions_hash: workflow_stable_hash(APPROVAL_PERMISSION_SCOPES),
        providers_hash: workflow_stable_hash(APPROVAL_PROVIDER_CLASSES),
        request_hash: workflow_stable_hash(&ApprovalRequest {
            request_ref: "approval:request".into(),
            subject_ref: "subject:generic".into(),
            policy_hash: "policy:hash".into(),
            schema_version: "v1".into(),
            ..Default::default()
        }),
        decision_gate_hash: workflow_stable_hash(&ApprovalDecisionGate {
            gate_ref: "approval:gate".into(),
            request_ref: "approval:request".into(),
            required_outcome: "approved".into(),
            decision_ref: Some("approval:decision".into()),
            consumption_mode: "single_use".into(),
            ..Default::default()
        }),
    }
}
