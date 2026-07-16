use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::workflow_common::{
    define_workflow_command_wrappers, workflow_pack_definition, workflow_stable_hash,
    WorkflowCommandEnvelope, WorkflowError, WorkflowPackDescriptor, WorkflowPage,
    WorkflowProviderClass,
};

pub const WORKFLOW_REVIEW_PACK_ID: &str = "pack.workflow.review.v1";
pub const WORKFLOW_REVIEW_SERVICE_ID: &str = "service.workflow.review";

pub const WORKFLOW_REVIEW_COMMANDS: &[&str] = &[
    "review.request_review",
    "review.record_finding",
    "review.request_fix",
    "review.request_rereview",
    "review.approve",
    "review.close_review",
    "review.dismiss",
    "review.list_findings",
    "review.evaluate_gate",
    "review.inspect_provider",
];

const REVIEW_PERMISSION_SCOPES: &[&str] = &[
    "workflow.review.request",
    "workflow.review.write",
    "workflow.review.approve",
    "workflow.review.dismiss",
    "workflow.review.finding.read",
    "workflow.review.admin",
];

const DURABLE_METADATA: &[(&str, &str)] = &[
    ("review_rounds", "true"),
    ("finding_lifecycle", "true"),
    ("raw_subject_in_trace", "false"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_workflow", "true"),
    ("revision_gate", "required"),
    ("provider_payloads_in_trace", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] =
    &[("plugin", "true"), ("closure_gate_conformance", "required")];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("reviews", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const REVIEW_PROVIDER_CLASSES: &[WorkflowProviderClass<'_>] = &[
    WorkflowProviderClass {
        provider_class: "durable-review",
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

/// Build the review descriptor without binding code-review or domain-specific review providers.
pub fn workflow_review_pack_definition() -> DomainPackDefinition {
    workflow_pack_definition(WorkflowPackDescriptor {
        pack_id: WORKFLOW_REVIEW_PACK_ID,
        child_change_id: "openspec:add-pack-workflow-review",
        docs_slug: "review",
        sdk_slug: "review",
        service_id: WORKFLOW_REVIEW_SERVICE_ID,
        commands: WORKFLOW_REVIEW_COMMANDS,
        permission_scopes: REVIEW_PERMISSION_SCOPES,
        provider_classes: REVIEW_PROVIDER_CLASSES,
        health_probe: "review.inspect_provider",
        unavailable_reason: "workflow_review_provider_not_installed",
        replay_schema: "workflow.review.replay.v1",
        data_classification: "workflow_review_reference_metadata",
        retention_policy: "review_request_round_finding_fix_outcome_gate_and_revision_metadata_by_reference",
        redaction_policy: "raw_subjects_findings_comments_prompts_provider_payloads_credentials_and_unbounded_logs_redacted",
        timeout_ms: 180_000,
        budget_units: 9,
        examples: &[
            "Declare `pack.workflow.review.v1` as optional until a review provider is installed.",
            "Use review request, round, finding, fix, outcome, and gate references instead of raw review payloads.",
        ],
        migration_notes: &[
            "Review commands become callable only after an approved review service provider registers matching schemas.",
            "Code review, document review, safety review, and business review are providers/applications, not OS branches.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub request_ref: String,
    pub subject_ref: String,
    pub subject_revision_hash: String,
    pub requester_ref: String,
    pub state: String,
    pub schema_version: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRound {
    pub round_ref: String,
    pub request_ref: String,
    pub round_index: u32,
    pub reviewer_pool_ref: String,
    pub started_epoch_ms: u64,
    pub closed_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub finding_ref: String,
    pub request_ref: String,
    pub severity: String,
    pub status: String,
    pub subject_span_ref: Option<String>,
    pub evidence_ref: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixRequest {
    pub fix_ref: String,
    pub finding_refs: Vec<String>,
    pub requested_by_ref: String,
    pub due_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewOutcome {
    pub outcome_ref: String,
    pub request_ref: String,
    pub outcome: String,
    pub approved_revision_hash: Option<String>,
    pub dismissal_reason_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewClosureGate {
    pub gate_ref: String,
    pub request_ref: String,
    pub subject_revision_hash: String,
    pub unresolved_blocking_count: u32,
    pub outcome_ref: Option<String>,
}

impl ReviewClosureGate {
    /// A review can close only when blocking findings are resolved for the same revision.
    pub fn can_close(&self) -> bool {
        self.unresolved_blocking_count == 0 && self.outcome_ref.is_some()
    }
}

pub type WorkflowReviewError = WorkflowError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowReviewResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Failure,
    StaleRevision,
    BlockingFindings,
    DismissalDenied,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowReviewResultEnvelope<T> {
    pub status: WorkflowReviewResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WorkflowPage<T>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowReviewError>,
}

define_workflow_command_wrappers!(
    ReviewRequestReviewCommand,
    ReviewRecordFindingCommand,
    ReviewRequestFixCommand,
    ReviewRequestRereviewCommand,
    ReviewApproveCommand,
    ReviewCloseReviewCommand,
    ReviewDismissCommand,
    ReviewListFindingsCommand,
    ReviewEvaluateGateCommand,
    ReviewInspectProviderCommand,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowReviewDescriptorHashes {
    pub descriptor_hash: String,
    pub commands_hash: String,
    pub permissions_hash: String,
    pub providers_hash: String,
    pub finding_hash: String,
    pub closure_gate_hash: String,
}

pub fn workflow_review_descriptor_hashes() -> WorkflowReviewDescriptorHashes {
    WorkflowReviewDescriptorHashes {
        descriptor_hash: workflow_stable_hash(&workflow_review_pack_definition()),
        commands_hash: workflow_stable_hash(WORKFLOW_REVIEW_COMMANDS),
        permissions_hash: workflow_stable_hash(REVIEW_PERMISSION_SCOPES),
        providers_hash: workflow_stable_hash(REVIEW_PROVIDER_CLASSES),
        finding_hash: workflow_stable_hash(&ReviewFinding {
            finding_ref: "review:finding".into(),
            request_ref: "review:request".into(),
            severity: "blocking".into(),
            status: "open".into(),
            evidence_ref: "evidence:finding".into(),
            blocking: true,
            ..Default::default()
        }),
        closure_gate_hash: workflow_stable_hash(&ReviewClosureGate {
            gate_ref: "review:gate".into(),
            request_ref: "review:request".into(),
            subject_revision_hash: "revision:hash".into(),
            unresolved_blocking_count: 0,
            outcome_ref: Some("review:outcome".into()),
        }),
    }
}
