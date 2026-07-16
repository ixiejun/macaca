use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_ISSUE_TRACKER_PACK_ID: &str = "pack.developer.issue.tracker.v1";
pub const DEVELOPER_ISSUE_TRACKER_SERVICE_ID: &str = "service.developer.issue_tracker";

pub const DEVELOPER_ISSUE_TRACKER_COMMANDS: &[&str] = &[
    "issue_tracker.inspect_provider",
    "issue_tracker.list_projects",
    "issue_tracker.inspect_schema",
    "issue_tracker.search_issues",
    "issue_tracker.get_issue",
    "issue_tracker.plan_create_issue",
    "issue_tracker.create_issue_request",
    "issue_tracker.plan_update_issue",
    "issue_tracker.update_issue_request",
    "issue_tracker.list_comments",
    "issue_tracker.create_comment_request",
    "issue_tracker.update_comment_request",
    "issue_tracker.plan_transition",
    "issue_tracker.transition_request",
    "issue_tracker.manage_labels",
    "issue_tracker.manage_assignees",
    "issue_tracker.manage_relations",
    "issue_tracker.get_attachment_handle",
    "issue_tracker.inspect_timeline",
];

const ISSUE_PERMISSION_SCOPES: &[&str] = &[
    "issue_tracker.provider.inspect",
    "issue_tracker.project.read",
    "issue_tracker.schema.read",
    "issue_tracker.issue.read",
    "issue_tracker.issue.create",
    "issue_tracker.issue.update",
    "issue_tracker.issue.transition",
    "issue_tracker.comment.read",
    "issue_tracker.comment.write",
    "issue_tracker.label.manage",
    "issue_tracker.assignee.manage",
    "issue_tracker.relation.manage",
    "issue_tracker.attachment.read",
    "issue_tracker.timeline.read",
];

const ISSUE_MODEL_METADATA: &[(&str, &str)] = &[("issue_model", "true"), ("field_schema", "true")];
const COMMENT_ATTACHMENT_METADATA: &[(&str, &str)] = &[
    ("comments", "redacted"),
    ("attachments", "handle_only"),
    ("raw_comments_in_trace", "false"),
];
const WORKFLOW_METADATA: &[(&str, &str)] = &[
    ("transitions", "plan_request_split"),
    ("notifications", "policy_bound"),
];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("issue_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const ISSUE_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "issue-model",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ISSUE_MODEL_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "comment-attachment",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: COMMENT_ATTACHMENT_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "workflow-transition",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: WORKFLOW_METADATA,
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

/// Build the issue-tracker descriptor without binding remote tracker APIs.
pub fn developer_issue_tracker_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_ISSUE_TRACKER_PACK_ID,
        child_change_id: "openspec:add-pack-developer-issue-tracker",
        docs_slug: "issue-tracker",
        sdk_slug: "issue.tracker",
        service_id: DEVELOPER_ISSUE_TRACKER_SERVICE_ID,
        commands: DEVELOPER_ISSUE_TRACKER_COMMANDS,
        permission_scopes: ISSUE_PERMISSION_SCOPES,
        provider_classes: ISSUE_PROVIDER_CLASSES,
        health_probe: "issue_tracker.inspect_provider",
        unavailable_reason: "developer_issue_tracker_provider_not_installed",
        replay_schema: "developer.issue_tracker.replay.v1",
        data_classification: "developer_issue_tracker_reference_metadata",
        retention_policy: "projects_schemas_issues_comments_labels_assignees_relations_attachments_and_timeline_metadata_by_reference",
        redaction_policy: "raw_credentials_tokens_private_comments_customer_data_attachments_provider_payloads_and_notifications_redacted",
        timeout_ms: 120_000,
        budget_units: 10,
        examples: &[
            "Declare `pack.developer.issue.tracker.v1` as optional until an issue tracker provider is installed.",
            "Use project, issue, comment, relation, attachment, and timeline references instead of raw tracker payloads.",
        ],
        migration_notes: &[
            "Issue tracker commands become callable only after an approved issue service provider registers matching schemas.",
            "Tracker clients, credentials, raw comments, attachments, and workflow mutations stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTrackerScope {
    pub scope_ref: String,
    pub project_scope_ref: String,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueProject {
    pub project_ref: String,
    pub scope_ref: String,
    pub visibility: String,
    pub issue_count_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueFieldSchema {
    pub schema_ref: String,
    pub project_ref: String,
    pub field_count: u32,
    pub version_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueItem {
    pub issue_ref: String,
    pub project_ref: String,
    pub version_hash: String,
    pub state_ref: String,
    pub field_summary_ref: String,
}

impl IssueItem {
    /// Ensure mutation requests can prove freshness without exposing field payloads.
    pub fn has_version_precondition(&self) -> bool {
        !self.issue_ref.is_empty() && !self.version_hash.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueComment {
    pub comment_ref: String,
    pub issue_ref: String,
    pub body_ref: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueLabel {
    pub label_ref: String,
    pub project_ref: String,
    pub color_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueMilestone {
    pub milestone_ref: String,
    pub project_ref: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueWorkflowState {
    pub state_ref: String,
    pub category: String,
    pub terminal: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTransitionPlan {
    pub plan_ref: String,
    pub issue_ref: String,
    pub from_state_ref: String,
    pub to_state_ref: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueUpdatePlan {
    pub plan_ref: String,
    pub issue_ref: String,
    pub field_updates_ref: String,
    pub version_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueSearchQuery {
    pub query_ref: String,
    pub project_ref: String,
    pub filter_hash: String,
    pub page_size: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRelation {
    pub relation_ref: String,
    pub source_issue_ref: String,
    pub target_issue_ref: String,
    pub relation_kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueAttachment {
    pub attachment_ref: String,
    pub issue_ref: String,
    pub content_type: String,
    pub size_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTimelineEvent {
    pub event_ref: String,
    pub issue_ref: String,
    pub event_kind: String,
    pub cursor: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTrackerProviderCapability {
    pub provider_class: String,
    pub features: BTreeSet<String>,
    pub supports_webhooks: bool,
    pub state: DomainPackProviderCapabilityState,
}

define_developer_command_wrappers!(
    IssueTrackerInspectProviderCommand,
    IssueTrackerListProjectsCommand,
    IssueTrackerInspectSchemaCommand,
    IssueTrackerSearchIssuesCommand,
    IssueTrackerGetIssueCommand,
    IssueTrackerPlanCreateIssueCommand,
    IssueTrackerCreateIssueRequestCommand,
    IssueTrackerPlanUpdateIssueCommand,
    IssueTrackerUpdateIssueRequestCommand,
    IssueTrackerListCommentsCommand,
    IssueTrackerCreateCommentRequestCommand,
    IssueTrackerUpdateCommentRequestCommand,
    IssueTrackerPlanTransitionCommand,
    IssueTrackerTransitionRequestCommand,
    IssueTrackerManageLabelsCommand,
    IssueTrackerManageAssigneesCommand,
    IssueTrackerManageRelationsCommand,
    IssueTrackerGetAttachmentHandleCommand,
    IssueTrackerInspectTimelineCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueTrackerResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    TransitionDenied,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTrackerResultEnvelope<T> {
    pub status: IssueTrackerResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTrackerDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub project_hash: String,
    pub issue_hash: String,
    pub transition_hash: String,
    pub attachment_hash: String,
    pub timeline_hash: String,
}

pub fn developer_issue_tracker_descriptor_hashes() -> IssueTrackerDescriptorHashes {
    let issue = IssueItem {
        issue_ref: "issue".into(),
        project_ref: "project".into(),
        version_hash: "version".into(),
        state_ref: "state".into(),
        field_summary_ref: "fields".into(),
    };
    IssueTrackerDescriptorHashes {
        command_schema_hash: issue_tracker_stable_hash(&DEVELOPER_ISSUE_TRACKER_COMMANDS),
        result_schema_hash: issue_tracker_stable_hash(&IssueTrackerResultStatus::Success),
        descriptor_hash: issue_tracker_stable_hash(&developer_issue_tracker_pack_definition()),
        provider_capability_hash: issue_tracker_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        project_hash: issue_tracker_stable_hash(&IssueProject {
            project_ref: "project".into(),
            visibility: "private".into(),
            ..Default::default()
        }),
        issue_hash: issue_tracker_stable_hash(&issue),
        transition_hash: issue_tracker_stable_hash(&IssueTransitionPlan {
            plan_ref: "transition".into(),
            issue_ref: "issue".into(),
            from_state_ref: "open".into(),
            to_state_ref: "closed".into(),
            approval_required: true,
        }),
        attachment_hash: issue_tracker_stable_hash(&IssueAttachment {
            attachment_ref: "attachment".into(),
            issue_ref: "issue".into(),
            content_type: "application/octet-stream".into(),
            size_class: "small".into(),
        }),
        timeline_hash: issue_tracker_stable_hash(&IssueTimelineEvent {
            event_ref: "event".into(),
            issue_ref: "issue".into(),
            event_kind: "updated".into(),
            cursor: "cursor".into(),
        }),
    }
}

pub fn issue_tracker_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
