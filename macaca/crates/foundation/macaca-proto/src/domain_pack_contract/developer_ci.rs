use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_CI_PACK_ID: &str = "pack.developer.ci.v1";
pub const DEVELOPER_CI_SERVICE_ID: &str = "service.developer.ci";

pub const DEVELOPER_CI_COMMANDS: &[&str] = &[
    "ci.inspect_provider",
    "ci.list_projects",
    "ci.list_pipelines",
    "ci.list_runs",
    "ci.inspect_run",
    "ci.inspect_status",
    "ci.plan_trigger",
    "ci.trigger_run_request",
    "ci.plan_cancel",
    "ci.cancel_run_request",
    "ci.plan_rerun",
    "ci.rerun_request",
    "ci.list_logs",
    "ci.get_log",
    "ci.list_artifacts",
    "ci.get_artifact_handle",
    "ci.inspect_tests",
    "ci.inspect_environment",
];
pub const DEVELOPER_CI_TRACE_EVENTS: &[&str] = &[
    "ci_pack_declared",
    "ci_pack_admission_validated",
    "ci_pack_policy_decision",
    "ci_pack_provider_inspected",
    "ci_pack_run_requested",
    "ci_pack_log_read",
    "ci_pack_artifact_handle",
    "ci_pack_unavailable",
    "ci_pack_snapshot_recorded",
];

const CI_PERMISSION_SCOPES: &[&str] = &[
    "ci.provider.inspect",
    "ci.project.read",
    "ci.pipeline.read",
    "ci.run.read",
    "ci.status.read",
    "ci.trigger.plan",
    "ci.trigger.request",
    "ci.cancel.plan",
    "ci.cancel.request",
    "ci.rerun.plan",
    "ci.rerun.request",
    "ci.log.read",
    "ci.artifact.read",
    "ci.test.read",
    "ci.environment.read",
];

const PIPELINE_METADATA: &[(&str, &str)] = &[("pipeline_model", "true"), ("trigger_plan", "true")];
const LOG_ARTIFACT_METADATA: &[(&str, &str)] = &[
    ("logs", "redacted"),
    ("artifacts", "handle_only"),
    ("raw_logs_in_trace", "false"),
];
const MUTATION_METADATA: &[(&str, &str)] = &[
    ("cancel_rerun", "policy_bound"),
    ("approval", "when_external"),
];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("ci_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const CI_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "pipeline-service",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PIPELINE_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "log-artifact-service",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOG_ARTIFACT_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "mutation-planner",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MUTATION_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the CI descriptor without binding concrete CI provider APIs.
pub fn developer_ci_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_CI_PACK_ID,
        child_change_id: "openspec:add-pack-developer-ci",
        docs_slug: "ci",
        sdk_slug: "ci",
        service_id: DEVELOPER_CI_SERVICE_ID,
        commands: DEVELOPER_CI_COMMANDS,
        permission_scopes: CI_PERMISSION_SCOPES,
        provider_classes: CI_PROVIDER_CLASSES,
        health_probe: "ci.inspect_provider",
        unavailable_reason: "developer_ci_provider_not_installed",
        replay_schema: "developer.ci.replay.v1",
        data_classification: "developer_ci_reference_metadata",
        retention_policy: "project_pipeline_run_job_status_log_artifact_test_environment_and_plan_metadata_by_reference",
        redaction_policy: "raw_credentials_tokens_logs_artifacts_provider_payloads_and_deployment_secrets_redacted",
        timeout_ms: 240_000,
        budget_units: 14,
        examples: &[
            "Declare `pack.developer.ci.v1` as optional until a CI provider is installed.",
            "Use project, pipeline, run, log cursor, artifact handle, and environment references instead of raw logs or artifacts.",
        ],
        migration_notes: &[
            "CI commands become callable only after an approved CI service provider registers matching schemas.",
            "CI provider clients, credentials, raw logs, artifacts, and trigger/cancel executors stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiProviderScope {
    pub scope_ref: String,
    pub project_scope_ref: String,
    pub credential_ref: Option<String>,
    pub network_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiProject {
    pub project_ref: String,
    pub scope_ref: String,
    pub visibility: String,
    pub pipeline_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiPipelineDefinition {
    pub pipeline_ref: String,
    pub project_ref: String,
    pub trigger_modes: Vec<String>,
    pub definition_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiRun {
    pub run_ref: String,
    pub pipeline_ref: String,
    pub status: CiStatus,
    pub attempt: u32,
    pub checkpoint_ref: Option<String>,
}

impl CiRun {
    /// Validate run metadata without contacting a CI provider.
    pub fn has_terminal_or_active_status(&self) -> bool {
        !self.status.status_ref.is_empty() && !self.run_ref.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiJob {
    pub job_ref: String,
    pub run_ref: String,
    pub status: CiStatus,
    pub step_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiStep {
    pub step_ref: String,
    pub job_ref: String,
    pub name_hash: String,
    pub status: CiStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiStatus {
    pub status_ref: String,
    pub state: String,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiTriggerPlan {
    pub plan_ref: String,
    pub pipeline_ref: String,
    pub parameter_schema_ref: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiMutationPlan {
    pub plan_ref: String,
    pub run_ref: String,
    pub mutation_kind: String,
    pub precondition_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiLogChunk {
    pub chunk_ref: String,
    pub run_ref: String,
    pub cursor: String,
    pub redacted_text_ref: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiArtifact {
    pub artifact_ref: String,
    pub run_ref: String,
    pub content_type: String,
    pub size_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiTestReport {
    pub report_ref: String,
    pub run_ref: String,
    pub passed: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiEnvironment {
    pub environment_ref: String,
    pub protection_state: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiProviderCapability {
    pub provider_class: String,
    pub supports_trigger: bool,
    pub supports_cancel: bool,
    pub supports_rerun: bool,
    pub state: DomainPackProviderCapabilityState,
}

define_developer_command_wrappers!(
    CiInspectProviderCommand,
    CiListProjectsCommand,
    CiListPipelinesCommand,
    CiListRunsCommand,
    CiInspectRunCommand,
    CiInspectStatusCommand,
    CiPlanTriggerCommand,
    CiTriggerRunRequestCommand,
    CiPlanCancelCommand,
    CiCancelRunRequestCommand,
    CiPlanRerunCommand,
    CiRerunRequestCommand,
    CiListLogsCommand,
    CiGetLogCommand,
    CiListArtifactsCommand,
    CiGetArtifactHandleCommand,
    CiInspectTestsCommand,
    CiInspectEnvironmentCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiResultStatus {
    Success,
    Paged,
    Partial,
    Streaming,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleStatus,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiResultEnvelope<T> {
    pub status: CiResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub pipeline_hash: String,
    pub run_hash: String,
    pub trigger_plan_hash: String,
    pub artifact_hash: String,
    pub test_hash: String,
}

pub fn developer_ci_descriptor_hashes() -> CiDescriptorHashes {
    let status = CiStatus {
        status_ref: "status".into(),
        state: "running".into(),
        conclusion: None,
    };
    CiDescriptorHashes {
        command_schema_hash: ci_stable_hash(&DEVELOPER_CI_COMMANDS),
        result_schema_hash: ci_stable_hash(&CiResultStatus::Success),
        descriptor_hash: ci_stable_hash(&developer_ci_pack_definition()),
        provider_capability_hash: ci_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        pipeline_hash: ci_stable_hash(&CiPipelineDefinition {
            pipeline_ref: "pipeline".into(),
            project_ref: "project".into(),
            trigger_modes: vec!["manual".into()],
            definition_hash: "definition".into(),
        }),
        run_hash: ci_stable_hash(&CiRun {
            run_ref: "run".into(),
            pipeline_ref: "pipeline".into(),
            status: status.clone(),
            attempt: 1,
            checkpoint_ref: Some("checkpoint".into()),
        }),
        trigger_plan_hash: ci_stable_hash(&CiTriggerPlan {
            plan_ref: "trigger".into(),
            pipeline_ref: "pipeline".into(),
            parameter_schema_ref: "schema".into(),
            approval_required: false,
        }),
        artifact_hash: ci_stable_hash(&CiArtifact {
            artifact_ref: "artifact".into(),
            run_ref: "run".into(),
            content_type: "application/octet-stream".into(),
            size_class: "small".into(),
        }),
        test_hash: ci_stable_hash(&CiTestReport {
            report_ref: "report".into(),
            run_ref: "run".into(),
            passed: 1,
            failed: 0,
        }),
    }
}

pub fn ci_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
