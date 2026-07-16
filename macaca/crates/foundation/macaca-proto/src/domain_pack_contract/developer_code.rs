use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_CODE_PACK_ID: &str = "pack.developer.code.v1";
pub const DEVELOPER_CODE_SERVICE_ID: &str = "service.developer.code";

pub const DEVELOPER_CODE_COMMANDS: &[&str] = &[
    "code.inspect_workspace",
    "code.index_workspace",
    "code.parse_document",
    "code.find_symbols",
    "code.find_references",
    "code.get_diagnostics",
    "code.discover_code_actions",
    "code.plan_edit",
    "code.generate_patch",
    "code.validate_patch",
    "code.apply_patch_request",
    "code.inspect_diff",
    "code.estimate_impact",
    "code.suggest_tests",
    "code.import_scan_results",
    "code.inspect_scan_findings",
    "code.inspect_provider",
];

const CODE_PERMISSION_SCOPES: &[&str] = &[
    "code.workspace.read",
    "code.workspace.index",
    "code.document.read",
    "code.document.parse",
    "code.symbol.read",
    "code.diagnostic.read",
    "code.action.read",
    "code.edit.plan",
    "code.patch.generate",
    "code.patch.validate",
    "code.patch.apply",
    "code.diff.read",
    "code.impact.read",
    "code.test.suggest",
    "code.scan.import",
    "code.scan.read",
    "code.provider.inspect",
];

const LANGUAGE_INTELLIGENCE_METADATA: &[(&str, &str)] = &[
    ("workspace_index", "true"),
    ("symbols", "true"),
    ("diagnostics", "true"),
    ("raw_source_in_trace", "false"),
];
const PATCH_PLANNER_METADATA: &[(&str, &str)] = &[
    ("patch_plan", "true"),
    ("dry_run", "true"),
    ("raw_patches_in_trace", "false"),
];
const SCAN_ADAPTER_METADATA: &[(&str, &str)] = &[("scan_import", "true"), ("sarif_like", "true")];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("source_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const CODE_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "language-intelligence",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LANGUAGE_INTELLIGENCE_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "patch-planner",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PATCH_PLANNER_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "scan-adapter",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SCAN_ADAPTER_METADATA,
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

/// Build the code-intelligence descriptor without binding parser, LSP, scanner, or model providers.
pub fn developer_code_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_CODE_PACK_ID,
        child_change_id: "openspec:add-pack-developer-code",
        docs_slug: "code",
        sdk_slug: "code",
        service_id: DEVELOPER_CODE_SERVICE_ID,
        commands: DEVELOPER_CODE_COMMANDS,
        permission_scopes: CODE_PERMISSION_SCOPES,
        provider_classes: CODE_PROVIDER_CLASSES,
        health_probe: "code.inspect_provider",
        unavailable_reason: "developer_code_provider_not_installed",
        replay_schema: "developer.code.replay.v1",
        data_classification: "developer_code_reference_metadata",
        retention_policy: "workspace_document_symbol_diagnostic_patch_diff_scan_and_impact_metadata_by_reference",
        redaction_policy: "raw_source_raw_patches_raw_diffs_scan_payloads_credentials_prompts_and_provider_payloads_redacted",
        timeout_ms: 180_000,
        budget_units: 12,
        examples: &[
            "Declare `pack.developer.code.v1` as optional until a code intelligence provider is installed.",
            "Use workspace, document, patch, diff, scan, and impact references instead of raw source or patch bodies.",
        ],
        migration_notes: &[
            "Code commands become callable only after an approved code service provider registers matching schemas.",
            "Parser engines, language servers, scanners, model clients, and patch appliers stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeWorkspace {
    pub workspace_ref: String,
    pub root_scope_ref: String,
    pub trust_state: String,
    pub language_count: u32,
    pub file_count: u64,
    pub index_state: String,
}

impl CodeWorkspace {
    /// Validate workspace metadata without reading source files.
    pub fn is_bounded(&self, max_files: u64) -> bool {
        !self.workspace_ref.is_empty() && self.file_count <= max_files
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeDocument {
    pub document_ref: String,
    pub workspace_ref: String,
    pub path_ref: String,
    pub language_id: String,
    pub version_hash: String,
    pub sensitivity_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeRange {
    pub range_ref: String,
    pub document_ref: String,
    pub start_line: u32,
    pub end_line: u32,
    pub byte_range_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTreeSummary {
    pub tree_ref: String,
    pub language_id: String,
    pub root_kind: String,
    pub node_count: u64,
    pub parse_error_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub symbol_ref: String,
    pub name_hash: String,
    pub kind: String,
    pub range: CodeRange,
    pub confidence_micros: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeDiagnostic {
    pub diagnostic_ref: String,
    pub severity: String,
    pub source_ref: String,
    pub rule_ref: String,
    pub range: CodeRange,
    pub fix_available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeAction {
    pub action_ref: String,
    pub title_ref: String,
    pub kind: String,
    pub safety_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEditPlan {
    pub plan_ref: String,
    pub affected_document_refs: Vec<String>,
    pub risk_flags: BTreeSet<String>,
    pub approval_required: bool,
    pub rollback_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodePatch {
    pub patch_ref: String,
    pub patch_format: String,
    pub affected_document_refs: Vec<String>,
    pub content_hash: String,
    pub dry_run_status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeDiff {
    pub diff_ref: String,
    pub base_ref: String,
    pub target_ref: String,
    pub file_change_count: u32,
    pub hunk_summary_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeImpactReport {
    pub report_ref: String,
    pub affected_symbol_count: u32,
    pub affected_flow_count: u32,
    pub confidence_micros: u32,
    pub suggested_tests_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeTestSuggestion {
    pub suggestion_ref: String,
    pub command_ref: String,
    pub test_kind: String,
    pub rationale_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeScanFinding {
    pub finding_ref: String,
    pub rule_ref: String,
    pub severity: String,
    pub location: CodeRange,
    pub baseline_status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeProviderCapability {
    pub provider_class: String,
    pub languages: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub patch_formats: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

define_developer_command_wrappers!(
    CodeInspectWorkspaceCommand,
    CodeIndexWorkspaceCommand,
    CodeParseDocumentCommand,
    CodeFindSymbolsCommand,
    CodeFindReferencesCommand,
    CodeGetDiagnosticsCommand,
    CodeDiscoverCodeActionsCommand,
    CodePlanEditCommand,
    CodeGeneratePatchCommand,
    CodeValidatePatchCommand,
    CodeApplyPatchRequestCommand,
    CodeInspectDiffCommand,
    CodeEstimateImpactCommand,
    CodeSuggestTestsCommand,
    CodeImportScanResultsCommand,
    CodeInspectScanFindingsCommand,
    CodeInspectProviderCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeResultStatus {
    Success,
    Paged,
    Partial,
    DryRun,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ValidationIssue,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeResultEnvelope<T> {
    pub status: CodeResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub workspace_hash: String,
    pub document_hash: String,
    pub patch_hash: String,
    pub diff_hash: String,
    pub scan_hash: String,
}

pub fn developer_code_descriptor_hashes() -> CodeDescriptorHashes {
    let range = CodeRange {
        range_ref: "range".into(),
        document_ref: "document".into(),
        start_line: 1,
        end_line: 2,
        byte_range_hash: "range-hash".into(),
    };
    CodeDescriptorHashes {
        command_schema_hash: code_stable_hash(&DEVELOPER_CODE_COMMANDS),
        result_schema_hash: code_stable_hash(&CodeResultStatus::Success),
        descriptor_hash: code_stable_hash(&developer_code_pack_definition()),
        provider_capability_hash: code_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        workspace_hash: code_stable_hash(&CodeWorkspace {
            workspace_ref: "workspace".into(),
            file_count: 10,
            ..Default::default()
        }),
        document_hash: code_stable_hash(&CodeDocument {
            document_ref: "document".into(),
            workspace_ref: "workspace".into(),
            path_ref: "path-ref".into(),
            language_id: "generic".into(),
            version_hash: "version".into(),
            sensitivity_class: "private".into(),
        }),
        patch_hash: code_stable_hash(&CodePatch {
            patch_ref: "patch".into(),
            patch_format: "unified".into(),
            content_hash: "patch-hash".into(),
            dry_run_status: "valid".into(),
            ..Default::default()
        }),
        diff_hash: code_stable_hash(&CodeDiff {
            diff_ref: "diff".into(),
            file_change_count: 1,
            ..Default::default()
        }),
        scan_hash: code_stable_hash(&CodeScanFinding {
            finding_ref: "finding".into(),
            rule_ref: "rule".into(),
            severity: "warning".into(),
            location: range,
            baseline_status: "new".into(),
        }),
    }
}

pub fn code_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
