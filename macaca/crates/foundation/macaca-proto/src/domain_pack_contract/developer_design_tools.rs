use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_DESIGN_TOOLS_PACK_ID: &str = "pack.developer.design.tools.v1";
pub const DEVELOPER_DESIGN_TOOLS_SERVICE_ID: &str = "service.developer.design_tools";

pub const DEVELOPER_DESIGN_TOOLS_COMMANDS: &[&str] = &[
    "design_tools.inspect_provider",
    "design_tools.list_workspaces",
    "design_tools.list_files",
    "design_tools.open_file",
    "design_tools.inspect_page",
    "design_tools.inspect_node",
    "design_tools.inspect_components",
    "design_tools.inspect_tokens",
    "design_tools.plan_token_sync",
    "design_tools.token_sync_request",
    "design_tools.plan_asset_export",
    "design_tools.export_asset_request",
    "design_tools.map_component",
    "design_tools.plan_write_change",
    "design_tools.write_change_request",
    "design_tools.inspect_reviews",
    "design_tools.get_artifact_handle",
];

/// Sanitized event names used for trace and replay without raw design payloads.
pub const DEVELOPER_DESIGN_TOOLS_TRACE_EVENTS: &[&str] = &[
    "design_tools_pack_declared",
    "design_tools_admission_validated",
    "design_tools_provider_inspected",
    "design_tools_workspace_listed",
    "design_tools_file_listed",
    "design_tools_file_opened",
    "design_tools_page_inspected",
    "design_tools_node_inspected",
    "design_tools_components_inspected",
    "design_tools_tokens_inspected",
    "design_tools_token_sync_planned",
    "design_tools_token_sync_requested",
    "design_tools_asset_export_planned",
    "design_tools_asset_export_requested",
    "design_tools_component_mapped",
    "design_tools_write_planned",
    "design_tools_write_requested",
    "design_tools_reviews_inspected",
    "design_tools_artifact_handle_issued",
    "design_tools_policy_decision",
    "design_tools_unavailable",
    "design_tools_snapshot_recorded",
];

const DESIGN_PERMISSION_SCOPES: &[&str] = &[
    "design_tools.provider.inspect",
    "design_tools.workspace.read",
    "design_tools.file.read",
    "design_tools.page.read",
    "design_tools.node.read",
    "design_tools.component.read",
    "design_tools.token.read",
    "design_tools.token.write",
    "design_tools.asset.export",
    "design_tools.component.map",
    "design_tools.design.write",
    "design_tools.review.read",
    "design_tools.artifact.read",
];

const DESIGN_READ_METADATA: &[(&str, &str)] = &[
    ("files", "true"),
    ("nodes", "true"),
    ("raw_design_in_trace", "false"),
];
const TOKEN_METADATA: &[(&str, &str)] =
    &[("tokens", "dtcg_like"), ("token_values_in_trace", "false")];
const WRITE_EXPORT_METADATA: &[(&str, &str)] = &[
    ("plan_request_split", "true"),
    ("artifacts", "handle_only"),
    ("raw_assets_in_trace", "false"),
];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("design_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const DESIGN_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "design-read",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DESIGN_READ_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "token-sync",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TOKEN_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "write-export",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: WRITE_EXPORT_METADATA,
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

/// Build the design-tools descriptor without binding concrete design platform APIs.
pub fn developer_design_tools_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_DESIGN_TOOLS_PACK_ID,
        child_change_id: "openspec:add-pack-developer-design-tools",
        docs_slug: "design-tools",
        sdk_slug: "design.tools",
        service_id: DEVELOPER_DESIGN_TOOLS_SERVICE_ID,
        commands: DEVELOPER_DESIGN_TOOLS_COMMANDS,
        permission_scopes: DESIGN_PERMISSION_SCOPES,
        provider_classes: DESIGN_PROVIDER_CLASSES,
        health_probe: "design_tools.inspect_provider",
        unavailable_reason: "developer_design_tools_provider_not_installed",
        replay_schema: "developer.design_tools.replay.v1",
        data_classification: "developer_design_tools_reference_metadata",
        retention_policy: "workspace_file_page_node_component_token_export_change_review_and_artifact_metadata_by_reference",
        redaction_policy: "raw_credentials_tokens_private_comments_design_files_assets_customer_data_and_provider_payloads_redacted",
        timeout_ms: 180_000,
        budget_units: 12,
        examples: &[
            "Declare `pack.developer.design.tools.v1` as optional until a design-tool provider is installed.",
            "Use workspace, file, node, component, token, change-set, review, and artifact refs instead of raw design files or assets.",
        ],
        migration_notes: &[
            "Design-tool commands become callable only after an approved design service provider registers matching schemas.",
            "Design platform clients, credentials, raw files, token values, assets, and write executors stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignToolScope {
    pub scope_ref: String,
    pub workspace_scope_ref: String,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignToolProviderCapability {
    pub provider_class: String,
    pub features: BTreeSet<String>,
    pub supports_write: bool,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignWorkspace {
    pub workspace_ref: String,
    pub visibility: String,
    pub file_count_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignFile {
    pub file_ref: String,
    pub workspace_ref: String,
    pub version_hash: String,
    pub permission_state: String,
}

impl DesignFile {
    /// Ensure write plans can prove version freshness without exposing raw files.
    pub fn has_version_precondition(&self) -> bool {
        !self.file_ref.is_empty() && !self.version_hash.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignPage {
    pub page_ref: String,
    pub file_ref: String,
    pub child_count_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignNode {
    pub node_ref: String,
    pub page_ref: String,
    pub node_kind: String,
    pub version_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignComponent {
    pub component_ref: String,
    pub file_ref: String,
    pub variant_hash: String,
    pub mapping_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignStyle {
    pub style_ref: String,
    pub style_kind: String,
    pub value_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignToken {
    pub token_ref: String,
    pub token_type: String,
    pub value_ref: String,
    pub version_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignTokenSyncPlan {
    pub plan_ref: String,
    pub token_schema_hash: String,
    pub conflict_count: u32,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignExportPlan {
    pub plan_ref: String,
    pub source_ref: String,
    pub format: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignArtifactHandle {
    pub artifact_ref: String,
    pub source_ref: String,
    pub content_type: String,
    pub size_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignChangeSet {
    pub change_set_ref: String,
    pub file_ref: String,
    pub version_hash: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignComponentMapping {
    pub mapping_ref: String,
    pub design_component_ref: String,
    pub code_component_ref: String,
    pub compatibility_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignReviewEvent {
    pub event_ref: String,
    pub file_ref: String,
    pub event_kind: String,
    pub comment_ref: Option<String>,
}

define_developer_command_wrappers!(
    DesignToolsInspectProviderCommand,
    DesignToolsListWorkspacesCommand,
    DesignToolsListFilesCommand,
    DesignToolsOpenFileCommand,
    DesignToolsInspectPageCommand,
    DesignToolsInspectNodeCommand,
    DesignToolsInspectComponentsCommand,
    DesignToolsInspectTokensCommand,
    DesignToolsPlanTokenSyncCommand,
    DesignToolsTokenSyncRequestCommand,
    DesignToolsPlanAssetExportCommand,
    DesignToolsExportAssetRequestCommand,
    DesignToolsMapComponentCommand,
    DesignToolsPlanWriteChangeCommand,
    DesignToolsWriteChangeRequestCommand,
    DesignToolsInspectReviewsCommand,
    DesignToolsGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignToolsResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    ExportDenied,
    WriteDenied,
    ArtifactDenied,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignToolsResultEnvelope<T> {
    pub status: DesignToolsResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignToolsDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub file_hash: String,
    pub node_hash: String,
    pub token_hash: String,
    pub change_hash: String,
    pub artifact_hash: String,
}

pub fn developer_design_tools_descriptor_hashes() -> DesignToolsDescriptorHashes {
    DesignToolsDescriptorHashes {
        command_schema_hash: design_tools_stable_hash(&DEVELOPER_DESIGN_TOOLS_COMMANDS),
        result_schema_hash: design_tools_stable_hash(&DesignToolsResultStatus::Success),
        descriptor_hash: design_tools_stable_hash(&developer_design_tools_pack_definition()),
        provider_capability_hash: design_tools_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        file_hash: design_tools_stable_hash(&DesignFile {
            file_ref: "file".into(),
            workspace_ref: "workspace".into(),
            version_hash: "version".into(),
            permission_state: "read".into(),
        }),
        node_hash: design_tools_stable_hash(&DesignNode {
            node_ref: "node".into(),
            page_ref: "page".into(),
            node_kind: "frame".into(),
            version_hash: "version".into(),
        }),
        token_hash: design_tools_stable_hash(&DesignToken {
            token_ref: "token".into(),
            token_type: "color".into(),
            value_ref: "value".into(),
            version_hash: "version".into(),
        }),
        change_hash: design_tools_stable_hash(&DesignChangeSet {
            change_set_ref: "change".into(),
            file_ref: "file".into(),
            version_hash: "version".into(),
            approval_required: true,
        }),
        artifact_hash: design_tools_stable_hash(&DesignArtifactHandle {
            artifact_ref: "artifact".into(),
            source_ref: "node".into(),
            content_type: "image/png".into(),
            size_class: "small".into(),
        }),
    }
}

pub fn design_tools_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
