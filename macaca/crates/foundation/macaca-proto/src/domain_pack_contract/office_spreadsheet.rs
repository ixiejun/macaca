use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::office_common::{
    define_office_command_wrappers, office_pack_definition, office_stable_hash,
    OfficeCommandEnvelope, OfficeError, OfficePackDescriptor, OfficePage, OfficeProviderClass,
};

pub const OFFICE_SPREADSHEET_PACK_ID: &str = "pack.office.spreadsheet.v1";
pub const OFFICE_SPREADSHEET_SERVICE_ID: &str = "service.office.spreadsheet";

pub const OFFICE_SPREADSHEET_COMMANDS: &[&str] = &[
    "spreadsheet.inspect_provider",
    "spreadsheet.create_workbook_request",
    "spreadsheet.import_workbook_request",
    "spreadsheet.open_workbook",
    "spreadsheet.list_worksheets",
    "spreadsheet.inspect_structure",
    "spreadsheet.read_range",
    "spreadsheet.inspect_formulas",
    "spreadsheet.inspect_tables",
    "spreadsheet.inspect_charts",
    "spreadsheet.inspect_pivots",
    "spreadsheet.plan_update",
    "spreadsheet.update_request",
    "spreadsheet.plan_recalculate",
    "spreadsheet.recalculate_request",
    "spreadsheet.plan_export",
    "spreadsheet.export_request",
    "spreadsheet.inspect_events",
    "spreadsheet.get_artifact_handle",
];

const SPREADSHEET_PERMISSION_SCOPES: &[&str] = &[
    "spreadsheet.provider.inspect",
    "spreadsheet.workbook.create",
    "spreadsheet.workbook.import",
    "spreadsheet.workbook.open",
    "spreadsheet.worksheet.read",
    "spreadsheet.range.read",
    "spreadsheet.formula.read",
    "spreadsheet.formula.write",
    "spreadsheet.range.write",
    "spreadsheet.table.manage",
    "spreadsheet.chart.manage",
    "spreadsheet.pivot.manage",
    "spreadsheet.filter.manage",
    "spreadsheet.protection.manage",
    "spreadsheet.calculate",
    "spreadsheet.export",
    "spreadsheet.events.read",
    "spreadsheet.artifact.read",
];

const WORKBOOK_PROVIDER_METADATA: &[(&str, &str)] = &[
    ("ranges", "true"),
    ("formulas", "true"),
    ("tables", "true"),
    ("charts", "true"),
];
const GRID_PROVIDER_METADATA: &[(&str, &str)] = &[
    ("ranges", "true"),
    ("formulas", "limited"),
    ("pivots", "false"),
    ("export", "true"),
];
const SPREADSHEET_MOCK_METADATA: &[(&str, &str)] = &[
    ("ranges", "true"),
    ("formulas", "true"),
    ("tables", "true"),
    ("export", "true"),
];
const SPREADSHEET_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("ranges", "false"),
    ("formulas", "false"),
    ("tables", "false"),
    ("export", "false"),
];

const SPREADSHEET_PROVIDER_CLASSES: &[OfficeProviderClass<'_>] = &[
    OfficeProviderClass {
        provider_class: "workbook-grid",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: WORKBOOK_PROVIDER_METADATA,
    },
    OfficeProviderClass {
        provider_class: "tabular-grid",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: GRID_PROVIDER_METADATA,
    },
    OfficeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SPREADSHEET_MOCK_METADATA,
    },
    OfficeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: SPREADSHEET_UNAVAILABLE_METADATA,
    },
];

pub fn office_spreadsheet_pack_definition() -> DomainPackDefinition {
    office_pack_definition(OfficePackDescriptor {
        pack_id: OFFICE_SPREADSHEET_PACK_ID,
        child_change_id: "openspec:add-pack-office-spreadsheet",
        docs_slug: "spreadsheet",
        service_id: OFFICE_SPREADSHEET_SERVICE_ID,
        commands: OFFICE_SPREADSHEET_COMMANDS,
        permission_scopes: SPREADSHEET_PERMISSION_SCOPES,
        provider_classes: SPREADSHEET_PROVIDER_CLASSES,
        health_probe: "spreadsheet.inspect_provider",
        unavailable_reason: "office_spreadsheet_provider_not_installed",
        replay_schema: "office.spreadsheet.replay.v1",
        data_classification: "office_spreadsheet_metadata",
        retention_policy: "workbook_values_formulas_artifacts_and_exports_by_reference",
        redaction_policy: "credentials_provider_payloads_private_financial_data_hidden_sheets_and_raw_formulas_redacted",
        examples: &[
            "Declare `pack.office.spreadsheet.v1` as optional until a spreadsheet provider is installed.",
            "Use workbook, range, update-plan, calculation, export, and artifact handles instead of raw workbook data.",
        ],
        migration_notes: &[
            "Spreadsheets become callable only after an approved spreadsheet service provider registers command schemas.",
            "Provider-native workbook sessions, formulas, and sheet payloads must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetScope {
    pub tenant_scope: String,
    pub workbook_scope: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetProviderCapability {
    pub provider_class: String,
    pub formats: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_cells: u64,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbookHandle {
    pub workbook_id: String,
    pub version_hash: String,
    pub format: String,
    pub scope: SpreadsheetScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorksheetHandle {
    pub worksheet_id: String,
    pub workbook_id: String,
    pub title_ref: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetRange {
    pub worksheet_id: String,
    pub anchor_hash: String,
    pub row_start: u32,
    pub row_end: u32,
    pub column_start: u32,
    pub column_end: u32,
}

impl SpreadsheetRange {
    pub fn cell_count(&self) -> u64 {
        let rows = self
            .row_end
            .saturating_sub(self.row_start)
            .saturating_add(1) as u64;
        let columns = self
            .column_end
            .saturating_sub(self.column_start)
            .saturating_add(1) as u64;
        rows.saturating_mul(columns)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetCell {
    pub address: String,
    pub value_ref: Option<String>,
    pub formula_ref: Option<String>,
    pub style_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetValueMatrix {
    pub range: SpreadsheetRange,
    pub values_ref: String,
    pub row_count: u32,
    pub column_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetFormula {
    pub formula_id: String,
    pub formula_ref: String,
    pub dependency_hash: String,
    pub safety_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetNamedRange {
    pub name: String,
    pub range: SpreadsheetRange,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetTable {
    pub table_id: String,
    pub range: SpreadsheetRange,
    pub schema_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetChart {
    pub chart_id: String,
    pub source_range: SpreadsheetRange,
    pub chart_kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetPivot {
    pub pivot_id: String,
    pub source_range: SpreadsheetRange,
    pub schema_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetFilterSort {
    pub range: SpreadsheetRange,
    pub filter_hash: String,
    pub sort_hash: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetValidationProtection {
    pub range: SpreadsheetRange,
    pub validation_hash: Option<String>,
    pub protection_policy: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetUpdateOperation {
    pub operation_id: String,
    pub operation_kind: String,
    pub range: Option<SpreadsheetRange>,
    pub payload_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetUpdatePlan {
    pub plan_id: String,
    pub base_version_hash: String,
    pub operations: Vec<SpreadsheetUpdateOperation>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetCalculationPlan {
    pub plan_id: String,
    pub target_range: Option<SpreadsheetRange>,
    pub calculation_mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetExportPlan {
    pub export_id: String,
    pub target_format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetCollaborationEvent {
    pub event_id: String,
    pub workbook_id: String,
    pub event_kind: String,
    pub cursor_hash: Option<String>,
}

define_office_command_wrappers!(
    SpreadsheetInspectProviderCommand,
    SpreadsheetCreateWorkbookRequestCommand,
    SpreadsheetImportWorkbookRequestCommand,
    SpreadsheetOpenWorkbookCommand,
    SpreadsheetListWorksheetsCommand,
    SpreadsheetInspectStructureCommand,
    SpreadsheetReadRangeCommand,
    SpreadsheetInspectFormulasCommand,
    SpreadsheetInspectTablesCommand,
    SpreadsheetInspectChartsCommand,
    SpreadsheetInspectPivotsCommand,
    SpreadsheetPlanUpdateCommand,
    SpreadsheetUpdateRequestCommand,
    SpreadsheetPlanRecalculateCommand,
    SpreadsheetRecalculateRequestCommand,
    SpreadsheetPlanExportCommand,
    SpreadsheetExportRequestCommand,
    SpreadsheetInspectEventsCommand,
    SpreadsheetGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    FormatUnsupported,
    FormulaDenied,
    CalculationDenied,
    ExportDenied,
    WriteDenied,
    ProtectionDenied,
    Quota,
    Timeout,
    Cancellation,
    ApprovalRequired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetResultEnvelope<T> {
    pub status: SpreadsheetResultStatus,
    pub data: Option<T>,
    pub page: Option<OfficePage<T>>,
    pub error: Option<OfficeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub workbook_version_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn office_spreadsheet_descriptor_hashes() -> SpreadsheetDescriptorHashes {
    SpreadsheetDescriptorHashes {
        command_schema_hash: spreadsheet_stable_hash(&OFFICE_SPREADSHEET_COMMANDS),
        result_schema_hash: spreadsheet_stable_hash(&SpreadsheetResultStatus::Success),
        descriptor_hash: spreadsheet_stable_hash(&office_spreadsheet_pack_definition()),
        provider_capability_schema_hash: spreadsheet_stable_hash(&SpreadsheetProviderCapability {
            provider_class: "mock".into(),
            formats: BTreeSet::from(["xlsx".into(), "csv".into()]),
            features: BTreeSet::from(["ranges".into(), "formulas".into(), "export".into()]),
            max_cells: 1_000_000,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        workbook_version_hash: spreadsheet_stable_hash(&WorkbookHandle {
            workbook_id: "workbook".into(),
            version_hash: "v1".into(),
            format: "xlsx".into(),
            scope: SpreadsheetScope::default(),
        }),
        unavailable_schema_hash: spreadsheet_stable_hash(&OfficeError {
            code: "unavailable".into(),
            message: "office spreadsheet provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("office_spreadsheet_provider_not_installed".into()),
        }),
    }
}

pub fn spreadsheet_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    office_stable_hash(value)
}
