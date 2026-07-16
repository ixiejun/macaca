use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::workflow_common::{
    define_workflow_command_wrappers, workflow_pack_definition, workflow_stable_hash,
    WorkflowCommandEnvelope, WorkflowError, WorkflowPackDescriptor, WorkflowPage,
    WorkflowProviderClass,
};

pub const WORKFLOW_DELEGATION_PACK_ID: &str = "pack.workflow.delegation.v1";
pub const WORKFLOW_DELEGATION_SERVICE_ID: &str = "service.workflow.delegation";

pub const WORKFLOW_DELEGATION_COMMANDS: &[&str] = &[
    "delegation.delegate",
    "delegation.accept_delegation",
    "delegation.handoff",
    "delegation.inspect_capacity",
    "delegation.collect_result",
    "delegation.cancel_delegation",
    "delegation.renew_lease",
    "delegation.inspect_provider",
];

const DELEGATION_PERMISSION_SCOPES: &[&str] = &[
    "workflow.delegation.create",
    "workflow.delegation.accept",
    "workflow.delegation.cancel",
    "workflow.delegation.read",
    "workflow.delegation.admin",
];

const DURABLE_METADATA: &[(&str, &str)] = &[
    ("leases", "true"),
    ("capacity", "true"),
    ("handoff_history", "bounded"),
    ("raw_work_payloads_in_trace", "false"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_workflow", "true"),
    ("atomic_claim", "required"),
    ("provider_payloads_in_trace", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] = &[("plugin", "true"), ("lease_conformance", "required")];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("delegations", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const DELEGATION_PROVIDER_CLASSES: &[WorkflowProviderClass<'_>] = &[
    WorkflowProviderClass {
        provider_class: "durable-delegation",
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

/// Build the delegation descriptor without binding an agent runtime or scheduler provider.
pub fn workflow_delegation_pack_definition() -> DomainPackDefinition {
    workflow_pack_definition(WorkflowPackDescriptor {
        pack_id: WORKFLOW_DELEGATION_PACK_ID,
        child_change_id: "openspec:add-pack-workflow-delegation",
        docs_slug: "delegation",
        sdk_slug: "delegation",
        service_id: WORKFLOW_DELEGATION_SERVICE_ID,
        commands: WORKFLOW_DELEGATION_COMMANDS,
        permission_scopes: DELEGATION_PERMISSION_SCOPES,
        provider_classes: DELEGATION_PROVIDER_CLASSES,
        health_probe: "delegation.inspect_provider",
        unavailable_reason: "workflow_delegation_provider_not_installed",
        replay_schema: "workflow.delegation.replay.v1",
        data_classification: "workflow_delegation_reference_metadata",
        retention_policy: "delegation_request_claim_lease_handoff_capacity_result_and_cancellation_metadata_by_reference",
        redaction_policy: "raw_work_payloads_agent_private_state_provider_payloads_credentials_results_and_unbounded_logs_redacted",
        timeout_ms: 180_000,
        budget_units: 10,
        examples: &[
            "Declare `pack.workflow.delegation.v1` as optional until a delegation provider is installed.",
            "Use request, claim, lease, capacity, handoff, and result references instead of raw work payloads.",
        ],
        migration_notes: &[
            "Delegation commands become callable only after an approved delegation service provider registers matching schemas.",
            "Agent selection, worker execution, and application task boards stay behind their own service boundaries.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub request_ref: String,
    pub work_ref: String,
    pub requester_ref: String,
    pub candidate_pool_ref: String,
    pub state: String,
    pub schema_version: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationClaim {
    pub claim_ref: String,
    pub request_ref: String,
    pub assignee_ref: String,
    pub capacity_snapshot_ref: String,
    pub accepted_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationLease {
    pub lease_ref: String,
    pub claim_ref: String,
    pub owner_ref: String,
    pub expires_at_epoch_ms: u64,
    pub renewable: bool,
    pub revoked: bool,
}

impl DelegationLease {
    /// Lease checks use caller-supplied time so tests and replay stay deterministic.
    pub fn is_active_at(&self, now_epoch_ms: u64) -> bool {
        !self.revoked && now_epoch_ms < self.expires_at_epoch_ms
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationHandoff {
    pub handoff_ref: String,
    pub request_ref: String,
    pub from_owner_ref: String,
    pub to_candidate_ref: String,
    pub checkpoint_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacitySnapshot {
    pub capacity_ref: String,
    pub subject_ref: String,
    pub available_units: u32,
    pub reserved_units: u32,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationResult {
    pub result_ref: String,
    pub request_ref: String,
    pub outcome: String,
    pub artifact_refs: Vec<String>,
    pub terminal: bool,
}

pub type WorkflowDelegationError = WorkflowError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowDelegationResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Failure,
    LeaseExpired,
    CapacityExhausted,
    IneligibleAssignee,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDelegationResultEnvelope<T> {
    pub status: WorkflowDelegationResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WorkflowPage<T>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowDelegationError>,
}

define_workflow_command_wrappers!(
    DelegationDelegateCommand,
    DelegationAcceptDelegationCommand,
    DelegationHandoffCommand,
    DelegationInspectCapacityCommand,
    DelegationCollectResultCommand,
    DelegationCancelDelegationCommand,
    DelegationRenewLeaseCommand,
    DelegationInspectProviderCommand,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDelegationDescriptorHashes {
    pub descriptor_hash: String,
    pub commands_hash: String,
    pub permissions_hash: String,
    pub providers_hash: String,
    pub request_hash: String,
    pub lease_hash: String,
}

pub fn workflow_delegation_descriptor_hashes() -> WorkflowDelegationDescriptorHashes {
    WorkflowDelegationDescriptorHashes {
        descriptor_hash: workflow_stable_hash(&workflow_delegation_pack_definition()),
        commands_hash: workflow_stable_hash(WORKFLOW_DELEGATION_COMMANDS),
        permissions_hash: workflow_stable_hash(DELEGATION_PERMISSION_SCOPES),
        providers_hash: workflow_stable_hash(DELEGATION_PROVIDER_CLASSES),
        request_hash: workflow_stable_hash(&DelegationRequest {
            request_ref: "delegation:request".into(),
            work_ref: "work:generic".into(),
            schema_version: "v1".into(),
            ..Default::default()
        }),
        lease_hash: workflow_stable_hash(&DelegationLease {
            lease_ref: "delegation:lease".into(),
            claim_ref: "delegation:claim".into(),
            owner_ref: "agent:owner".into(),
            expires_at_epoch_ms: 1,
            renewable: true,
            revoked: false,
        }),
    }
}
