use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_BROWSER_AUTOMATION_PACK_ID: &str = "pack.developer.browser.automation.v1";
pub const DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID: &str = "service.developer.browser_automation";

pub const DEVELOPER_BROWSER_AUTOMATION_COMMANDS: &[&str] = &[
    "browser.inspect_provider",
    "browser.plan_context",
    "browser.open_context_request",
    "browser.open_page",
    "browser.navigate",
    "browser.wait_for",
    "browser.inspect_dom",
    "browser.resolve_locator",
    "browser.plan_action",
    "browser.action_request",
    "browser.plan_evaluate",
    "browser.evaluate_request",
    "browser.capture_screenshot",
    "browser.capture_accessibility",
    "browser.manage_download",
    "browser.manage_upload",
    "browser.inspect_events",
    "browser.manage_storage_state",
    "browser.close_page",
    "browser.close_context",
];

pub const DEVELOPER_BROWSER_AUTOMATION_TRACE_EVENTS: &[&str] = &[
    "browser_pack_declared",
    "browser_pack_admission_validated",
    "browser_pack_policy_decision",
    "browser_pack_provider_inspected",
    "browser_pack_context_opened",
    "browser_pack_page_opened",
    "browser_pack_navigation",
    "browser_pack_action_requested",
    "browser_pack_evaluation_requested",
    "browser_pack_artifact_recorded",
    "browser_pack_unavailable",
    "browser_pack_snapshot_recorded",
];

const BROWSER_PERMISSION_SCOPES: &[&str] = &[
    "browser.provider.inspect",
    "browser.context.open",
    "browser.context.close",
    "browser.page.open",
    "browser.page.close",
    "browser.navigate",
    "browser.wait",
    "browser.dom.inspect",
    "browser.locator.resolve",
    "browser.action.perform",
    "browser.evaluate",
    "browser.screenshot",
    "browser.accessibility.inspect",
    "browser.download.manage",
    "browser.upload.manage",
    "browser.events.inspect",
    "browser.storage.manage",
];

const BROWSER_RUNTIME_METADATA: &[(&str, &str)] = &[
    ("contexts", "true"),
    ("pages", "true"),
    ("raw_dom_in_trace", "false"),
];
const ACTION_RUNTIME_METADATA: &[(&str, &str)] = &[
    ("plan_request_split", "true"),
    ("script_evaluation", "sandboxed"),
];
const ARTIFACT_METADATA: &[(&str, &str)] = &[
    ("screenshots", "handle_only"),
    ("downloads", "handle_only"),
    ("raw_artifacts_in_trace", "false"),
];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("browser_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const BROWSER_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "browser-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: BROWSER_RUNTIME_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "action-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACTION_RUNTIME_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "artifact-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ARTIFACT_METADATA,
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

/// Build the browser automation descriptor without binding browser engines or drivers.
pub fn developer_browser_automation_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_BROWSER_AUTOMATION_PACK_ID,
        child_change_id: "openspec:add-pack-developer-browser-automation",
        docs_slug: "browser-automation",
        sdk_slug: "browser.automation",
        service_id: DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID,
        commands: DEVELOPER_BROWSER_AUTOMATION_COMMANDS,
        permission_scopes: BROWSER_PERMISSION_SCOPES,
        provider_classes: BROWSER_PROVIDER_CLASSES,
        health_probe: "browser.inspect_provider",
        unavailable_reason: "developer_browser_automation_provider_not_installed",
        replay_schema: "developer.browser_automation.replay.v1",
        data_classification: "developer_browser_automation_reference_metadata",
        retention_policy: "context_page_frame_locator_action_artifact_event_storage_and_snapshot_metadata_by_reference",
        redaction_policy: "cookies_credentials_storage_dom_screenshots_downloads_uploads_network_payloads_and_provider_payloads_redacted",
        timeout_ms: 240_000,
        budget_units: 16,
        examples: &[
            "Declare `pack.developer.browser.automation.v1` as optional until a browser automation provider is installed.",
            "Use context, page, frame, locator, action, artifact, event, and storage handles instead of raw browser payloads.",
        ],
        migration_notes: &[
            "Browser automation commands become callable only after an approved browser service provider registers matching schemas.",
            "Browser drivers, engines, remote grids, cookies, DOM, network payloads, and artifacts stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserAutomationScope {
    pub scope_ref: String,
    pub origin_policy_ref: String,
    pub artifact_policy_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserProviderCapability {
    pub provider_class: String,
    pub supports_contexts: bool,
    pub supports_actions: bool,
    pub supports_artifacts: bool,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserContextProfile {
    pub profile_ref: String,
    pub isolation_mode: String,
    pub storage_policy: String,
    pub origin_policy_ref: String,
}

impl BrowserContextProfile {
    /// Ensure context creation remains plan-owned and policy scoped.
    pub fn is_isolated(&self) -> bool {
        !self.profile_ref.is_empty() && !self.isolation_mode.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPage {
    pub page_ref: String,
    pub context_ref: String,
    pub url_hash: String,
    pub lifecycle_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserFrame {
    pub frame_ref: String,
    pub page_ref: String,
    pub origin_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserLocator {
    pub locator_ref: String,
    pub page_ref: String,
    pub strategy: String,
    pub mapping_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNavigationPlan {
    pub plan_ref: String,
    pub page_ref: String,
    pub target_url_hash: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserActionPlan {
    pub plan_ref: String,
    pub locator_ref: String,
    pub action_kind: String,
    pub side_effect_class: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserEvaluationPlan {
    pub plan_ref: String,
    pub page_ref: String,
    pub sandbox_ref: String,
    pub script_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserWaitCondition {
    pub condition_ref: String,
    pub condition_kind: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserArtifactHandle {
    pub artifact_ref: String,
    pub source_ref: String,
    pub artifact_kind: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNetworkEvent {
    pub event_ref: String,
    pub page_ref: String,
    pub url_hash: String,
    pub resource_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserConsoleEvent {
    pub event_ref: String,
    pub page_ref: String,
    pub level: String,
    pub text_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserDialogEvent {
    pub event_ref: String,
    pub page_ref: String,
    pub dialog_kind: String,
    pub message_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTraceEvent {
    pub event_ref: String,
    pub context_ref: String,
    pub event_kind: String,
    pub artifact_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserStorageHandle {
    pub storage_ref: String,
    pub context_ref: String,
    pub storage_kind: String,
    pub sensitivity_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionSnapshot {
    pub snapshot_ref: String,
    pub context_ref: String,
    pub page_count: u32,
    pub artifact_summary_ref: String,
}

define_developer_command_wrappers!(
    BrowserInspectProviderCommand,
    BrowserPlanContextCommand,
    BrowserOpenContextRequestCommand,
    BrowserOpenPageCommand,
    BrowserNavigateCommand,
    BrowserWaitForCommand,
    BrowserInspectDomCommand,
    BrowserResolveLocatorCommand,
    BrowserPlanActionCommand,
    BrowserActionRequestCommand,
    BrowserPlanEvaluateCommand,
    BrowserEvaluateRequestCommand,
    BrowserCaptureScreenshotCommand,
    BrowserCaptureAccessibilityCommand,
    BrowserManageDownloadCommand,
    BrowserManageUploadCommand,
    BrowserInspectEventsCommand,
    BrowserManageStorageStateCommand,
    BrowserClosePageCommand,
    BrowserCloseContextCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserResultStatus {
    Success,
    Streaming,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleHandle,
    NotFound,
    AmbiguousLocator,
    NavigationFailed,
    ActionabilityFailed,
    ScriptDenied,
    ArtifactDenied,
    StorageDenied,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserResultEnvelope<T> {
    pub status: BrowserResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub context_hash: String,
    pub page_hash: String,
    pub action_hash: String,
    pub artifact_hash: String,
    pub snapshot_hash: String,
}

pub fn developer_browser_automation_descriptor_hashes() -> BrowserDescriptorHashes {
    BrowserDescriptorHashes {
        command_schema_hash: browser_stable_hash(&DEVELOPER_BROWSER_AUTOMATION_COMMANDS),
        result_schema_hash: browser_stable_hash(&BrowserResultStatus::Success),
        descriptor_hash: browser_stable_hash(&developer_browser_automation_pack_definition()),
        provider_capability_hash: browser_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        context_hash: browser_stable_hash(&BrowserContextProfile {
            profile_ref: "context".into(),
            isolation_mode: "isolated".into(),
            storage_policy: "ephemeral".into(),
            origin_policy_ref: "origin-policy".into(),
        }),
        page_hash: browser_stable_hash(&BrowserPage {
            page_ref: "page".into(),
            context_ref: "context".into(),
            url_hash: "url".into(),
            lifecycle_state: "open".into(),
        }),
        action_hash: browser_stable_hash(&BrowserActionPlan {
            plan_ref: "action".into(),
            locator_ref: "locator".into(),
            action_kind: "click".into(),
            side_effect_class: "bounded".into(),
            approval_required: false,
        }),
        artifact_hash: browser_stable_hash(&BrowserArtifactHandle {
            artifact_ref: "artifact".into(),
            source_ref: "page".into(),
            artifact_kind: "screenshot".into(),
            redaction_class: "redacted".into(),
        }),
        snapshot_hash: browser_stable_hash(&BrowserSessionSnapshot {
            snapshot_ref: "snapshot".into(),
            context_ref: "context".into(),
            page_count: 1,
            artifact_summary_ref: "artifacts".into(),
        }),
    }
}

pub fn browser_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
