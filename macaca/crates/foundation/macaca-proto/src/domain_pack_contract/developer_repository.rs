use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_REPOSITORY_PACK_ID: &str = "pack.developer.repository.v1";
pub const DEVELOPER_REPOSITORY_SERVICE_ID: &str = "service.developer.repository";

pub const DEVELOPER_REPOSITORY_COMMANDS: &[&str] = &[
    "repository.open",
    "repository.inspect",
    "repository.status",
    "repository.list_refs",
    "repository.inspect_history",
    "repository.diff",
    "repository.stage_changes",
    "repository.plan_commit",
    "repository.create_commit_request",
    "repository.list_remotes",
    "repository.fetch",
    "repository.plan_pull",
    "repository.plan_push",
    "repository.push_request",
    "repository.plan_merge",
    "repository.validate_mutation",
    "repository.inspect_remote_metadata",
    "repository.inspect_provider",
];

/// Sanitized repository lifecycle events used for trace, audit, and replay.
pub const DEVELOPER_REPOSITORY_TRACE_EVENTS: &[&str] = &[
    "repository_pack_declared",
    "repository_admission_validated",
    "repository_opened",
    "repository_inspected",
    "repository_status_read",
    "repository_refs_listed",
    "repository_history_inspected",
    "repository_diff_inspected",
    "repository_changes_staged",
    "repository_commit_planned",
    "repository_commit_requested",
    "repository_remotes_listed",
    "repository_fetched",
    "repository_pull_planned",
    "repository_push_planned",
    "repository_push_requested",
    "repository_merge_planned",
    "repository_mutation_validated",
    "repository_remote_metadata_inspected",
    "repository_provider_inspected",
    "repository_policy_decision",
    "repository_unavailable",
    "repository_snapshot_recorded",
];

const REPOSITORY_PERMISSION_SCOPES: &[&str] = &[
    "repository.local.read",
    "repository.local.write",
    "repository.status.read",
    "repository.diff.read",
    "repository.history.read",
    "repository.ref.read",
    "repository.ref.write",
    "repository.stage.write",
    "repository.commit.create",
    "repository.remote.read",
    "repository.remote.fetch",
    "repository.remote.push",
    "repository.remote.metadata",
    "repository.mutation.plan",
    "repository.mutation.validate",
    "repository.provider.inspect",
];

const LOCAL_VCS_METADATA: &[(&str, &str)] = &[
    ("status", "true"),
    ("diff", "true"),
    ("raw_diff_in_trace", "false"),
];
const REMOTE_VCS_METADATA: &[(&str, &str)] = &[
    ("fetch_push", "policy_bound"),
    ("private_urls_in_trace", "false"),
];
const MUTATION_METADATA: &[(&str, &str)] = &[
    ("plan_request_split", "true"),
    ("protected_ref_checks", "true"),
];
const MOCK_METADATA: &[(&str, &str)] = &[
    ("deterministic", "true"),
    ("repository_payloads", "synthetic"),
];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const REPOSITORY_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "local-vcs",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_VCS_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "remote-vcs",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REMOTE_VCS_METADATA,
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

/// Build the repository descriptor without binding Git clients, CLIs, or remote APIs.
pub fn developer_repository_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_REPOSITORY_PACK_ID,
        child_change_id: "openspec:add-pack-developer-repository",
        docs_slug: "repository",
        sdk_slug: "repository",
        service_id: DEVELOPER_REPOSITORY_SERVICE_ID,
        commands: DEVELOPER_REPOSITORY_COMMANDS,
        permission_scopes: REPOSITORY_PERMISSION_SCOPES,
        provider_classes: REPOSITORY_PROVIDER_CLASSES,
        health_probe: "repository.inspect_provider",
        unavailable_reason: "developer_repository_provider_not_installed",
        replay_schema: "developer.repository.replay.v1",
        data_classification: "developer_repository_reference_metadata",
        retention_policy: "repository_refs_status_diffs_mutation_plans_sync_plans_and_remote_metadata_by_reference",
        redaction_policy: "raw_credentials_private_remote_urls_raw_source_raw_diffs_tokens_and_provider_payloads_redacted",
        timeout_ms: 180_000,
        budget_units: 12,
        examples: &[
            "Declare `pack.developer.repository.v1` as optional until a repository provider is installed.",
            "Use repository, ref, diff, mutation-plan, and sync-plan handles instead of raw Git output.",
        ],
        migration_notes: &[
            "Repository commands become callable only after an approved repository service provider registers matching schemas.",
            "Git libraries, CLIs, remote clients, credential managers, and mutation executors stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryHandle {
    pub repository_ref: String,
    pub workspace_ref: String,
    pub vcs_kind: String,
    pub trust_state: String,
}

impl RepositoryHandle {
    /// Validate repository metadata without opening a real repository.
    pub fn is_scoped(&self) -> bool {
        !self.repository_ref.is_empty() && !self.workspace_ref.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryRemote {
    pub remote_ref: String,
    pub repository_ref: String,
    pub url_hash: String,
    pub auth_mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryRef {
    pub ref_ref: String,
    pub repository_ref: String,
    pub ref_kind: String,
    pub object_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryBranch {
    pub branch_ref: String,
    pub head: RepositoryRef,
    pub protected: bool,
    pub upstream_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryTag {
    pub tag_ref: String,
    pub target: RepositoryRef,
    pub annotated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryCommit {
    pub commit_ref: String,
    pub object_hash: String,
    pub parent_count: u32,
    pub message_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryStatusEntry {
    pub entry_ref: String,
    pub path_ref: String,
    pub state: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryDiff {
    pub diff_ref: String,
    pub file_change_count: u32,
    pub hunk_summary_ref: String,
    pub binary_change_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryMutationPlan {
    pub plan_ref: String,
    pub mutation_kind: String,
    pub current_object_ref: String,
    pub protected_ref: bool,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySyncPlan {
    pub plan_ref: String,
    pub remote_ref: String,
    pub source_ref: String,
    pub target_ref: String,
    pub diverged: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryProviderCapability {
    pub provider_class: String,
    pub vcs_kinds: Vec<String>,
    pub remote_support: bool,
    pub mutation_support: bool,
    pub state: DomainPackProviderCapabilityState,
}

define_developer_command_wrappers!(
    RepositoryOpenCommand,
    RepositoryInspectCommand,
    RepositoryStatusCommand,
    RepositoryListRefsCommand,
    RepositoryInspectHistoryCommand,
    RepositoryDiffCommand,
    RepositoryStageChangesCommand,
    RepositoryPlanCommitCommand,
    RepositoryCreateCommitRequestCommand,
    RepositoryListRemotesCommand,
    RepositoryFetchCommand,
    RepositoryPlanPullCommand,
    RepositoryPlanPushCommand,
    RepositoryPushRequestCommand,
    RepositoryPlanMergeCommand,
    RepositoryValidateMutationCommand,
    RepositoryInspectRemoteMetadataCommand,
    RepositoryInspectProviderCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryResultStatus {
    Success,
    Paged,
    Partial,
    DryRun,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    Diverged,
    DirtyWorktree,
    ProtectedRef,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryResultEnvelope<T> {
    pub status: RepositoryResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub repository_hash: String,
    pub ref_hash: String,
    pub diff_hash: String,
    pub mutation_hash: String,
    pub sync_hash: String,
}

pub fn developer_repository_descriptor_hashes() -> RepositoryDescriptorHashes {
    let repo = RepositoryHandle {
        repository_ref: "repository".into(),
        workspace_ref: "workspace".into(),
        vcs_kind: "generic-vcs".into(),
        trust_state: "trusted".into(),
    };
    let reference = RepositoryRef {
        ref_ref: "ref".into(),
        repository_ref: "repository".into(),
        ref_kind: "branch".into(),
        object_hash: "object".into(),
    };
    RepositoryDescriptorHashes {
        command_schema_hash: repository_stable_hash(&DEVELOPER_REPOSITORY_COMMANDS),
        result_schema_hash: repository_stable_hash(&RepositoryResultStatus::Success),
        descriptor_hash: repository_stable_hash(&developer_repository_pack_definition()),
        provider_capability_hash: repository_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        repository_hash: repository_stable_hash(&repo),
        ref_hash: repository_stable_hash(&reference),
        diff_hash: repository_stable_hash(&RepositoryDiff {
            diff_ref: "diff".into(),
            file_change_count: 1,
            ..Default::default()
        }),
        mutation_hash: repository_stable_hash(&RepositoryMutationPlan {
            plan_ref: "mutation".into(),
            mutation_kind: "commit".into(),
            current_object_ref: "object".into(),
            protected_ref: false,
            approval_required: false,
        }),
        sync_hash: repository_stable_hash(&RepositorySyncPlan {
            plan_ref: "sync".into(),
            remote_ref: "remote".into(),
            source_ref: "source".into(),
            target_ref: "target".into(),
            diverged: false,
        }),
    }
}

pub fn repository_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
