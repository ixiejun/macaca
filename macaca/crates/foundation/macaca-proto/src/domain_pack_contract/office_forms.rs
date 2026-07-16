use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::office_common::{
    define_office_command_wrappers, office_pack_definition, office_stable_hash,
    OfficeCommandEnvelope, OfficeError, OfficePackDescriptor, OfficePage, OfficeProviderClass,
};

pub const OFFICE_FORMS_PACK_ID: &str = "pack.office.forms.v1";
pub const OFFICE_FORMS_SERVICE_ID: &str = "service.office.forms";

pub const OFFICE_FORMS_COMMANDS: &[&str] = &[
    "forms.inspect_provider",
    "forms.create_form_request",
    "forms.import_form_request",
    "forms.open_form",
    "forms.inspect_metadata",
    "forms.inspect_schema",
    "forms.plan_schema_edit",
    "forms.schema_edit_request",
    "forms.create_response_session",
    "forms.validate_response_draft",
    "forms.submit_response_request",
    "forms.get_submission_receipt",
    "forms.list_responses",
    "forms.get_response",
    "forms.plan_response_export",
    "forms.response_export_request",
    "forms.plan_event_subscription",
    "forms.event_subscription_request",
    "forms.inspect_events",
    "forms.get_artifact_handle",
];

const FORMS_PERMISSION_SCOPES: &[&str] = &[
    "forms.provider.inspect",
    "forms.form.create",
    "forms.form.import",
    "forms.form.open",
    "forms.metadata.read",
    "forms.schema.read",
    "forms.schema.write",
    "forms.response.session",
    "forms.response.validate",
    "forms.response.submit",
    "forms.response.read",
    "forms.response.export",
    "forms.event.subscribe",
    "forms.event.read",
    "forms.artifact.read",
];

const FORM_SCHEMA_METADATA: &[(&str, &str)] = &[
    ("schema_edit", "true"),
    ("validation", "true"),
    ("conditional_logic", "true"),
    ("publish", "true"),
];
const FORM_RESPONSE_METADATA: &[(&str, &str)] = &[
    ("response_submit", "true"),
    ("response_read", "true"),
    ("response_export", "true"),
    ("webhook", "true"),
];
const FORMS_MOCK_METADATA: &[(&str, &str)] = &[
    ("schema_edit", "true"),
    ("responses", "true"),
    ("events", "true"),
    ("export", "true"),
];
const FORMS_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("schema_edit", "false"),
    ("responses", "false"),
    ("events", "false"),
    ("export", "false"),
];

const FORMS_PROVIDER_CLASSES: &[OfficeProviderClass<'_>] = &[
    OfficeProviderClass {
        provider_class: "form-schema",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: FORM_SCHEMA_METADATA,
    },
    OfficeProviderClass {
        provider_class: "form-response",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: FORM_RESPONSE_METADATA,
    },
    OfficeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: FORMS_MOCK_METADATA,
    },
    OfficeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: FORMS_UNAVAILABLE_METADATA,
    },
];

/// Build the forms descriptor without binding Google Forms, Typeform, Jotform, or webhooks.
pub fn office_forms_pack_definition() -> DomainPackDefinition {
    office_pack_definition(OfficePackDescriptor {
        pack_id: OFFICE_FORMS_PACK_ID,
        child_change_id: "openspec:add-pack-office-forms",
        docs_slug: "forms",
        service_id: OFFICE_FORMS_SERVICE_ID,
        commands: OFFICE_FORMS_COMMANDS,
        permission_scopes: FORMS_PERMISSION_SCOPES,
        provider_classes: FORMS_PROVIDER_CLASSES,
        health_probe: "forms.inspect_provider",
        unavailable_reason: "office_forms_provider_not_installed",
        replay_schema: "office.forms.replay.v1",
        data_classification: "office_forms_metadata",
        retention_policy: "form_schema_responses_exports_events_and_artifacts_by_reference",
        redaction_policy: "credentials_webhook_secrets_respondent_pii_raw_answers_exports_and_provider_payloads_redacted",
        examples: &[
            "Declare `pack.office.forms.v1` as optional until a forms provider is installed.",
            "Use form handles, schema hashes, response references, event cursors, and artifacts instead of raw responses.",
        ],
        migration_notes: &[
            "Forms become callable only after an approved forms service provider registers command schemas.",
            "Provider-native question schemas, respondent data, and webhook payloads must stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormsScope {
    pub tenant_scope: String,
    pub form_scope: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormsProviderCapability {
    pub provider_class: String,
    pub field_types: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_fields: u32,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormHandle {
    pub form_id: String,
    pub schema_version_hash: String,
    pub published: bool,
    pub scope: FormsScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormMetadata {
    pub form_id: String,
    pub title_ref: String,
    pub description_ref: Option<String>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormSchema {
    pub schema_id: String,
    pub version_hash: String,
    pub sections: Vec<FormSection>,
}

impl FormSchema {
    pub fn field_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.fields.len())
            .sum()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormSection {
    pub section_id: String,
    pub title_ref: Option<String>,
    pub fields: Vec<FormField>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    pub field_id: String,
    pub field_kind: String,
    pub label_ref: String,
    pub required: bool,
    pub options: Vec<FormFieldOption>,
    pub validation: Vec<FormValidationRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormFieldOption {
    pub option_id: String,
    pub label_ref: String,
    pub value_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormValidationRule {
    pub rule_id: String,
    pub rule_kind: String,
    pub parameter_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormLogicRule {
    pub rule_id: String,
    pub condition_hash: String,
    pub target_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormPublishSettings {
    pub published: bool,
    pub access_policy: String,
    pub collect_identity: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RespondentSession {
    pub session_id: String,
    pub form_id: String,
    pub expires_at_epoch_ms: u64,
    pub consent_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormResponseDraft {
    pub draft_id: String,
    pub form_id: String,
    pub schema_version_hash: String,
    pub values: Vec<FormResponseValue>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormResponseValue {
    pub field_id: String,
    pub value_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormSubmissionReceipt {
    pub receipt_id: String,
    pub form_id: String,
    pub submitted_at_epoch_ms: u64,
    pub response_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormResponseExportPlan {
    pub export_id: String,
    pub filter_hash: String,
    pub target_format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormEventSubscriptionPlan {
    pub subscription_id: String,
    pub event_kinds: BTreeSet<String>,
    pub endpoint_ref: String,
    pub signing_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormEvent {
    pub event_id: String,
    pub form_id: String,
    pub event_kind: String,
    pub cursor_hash: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_office_command_wrappers!(
    FormsInspectProviderCommand,
    FormsCreateFormRequestCommand,
    FormsImportFormRequestCommand,
    FormsOpenFormCommand,
    FormsInspectMetadataCommand,
    FormsInspectSchemaCommand,
    FormsPlanSchemaEditCommand,
    FormsSchemaEditRequestCommand,
    FormsCreateResponseSessionCommand,
    FormsValidateResponseDraftCommand,
    FormsSubmitResponseRequestCommand,
    FormsGetSubmissionReceiptCommand,
    FormsListResponsesCommand,
    FormsGetResponseCommand,
    FormsPlanResponseExportCommand,
    FormsResponseExportRequestCommand,
    FormsPlanEventSubscriptionCommand,
    FormsEventSubscriptionRequestCommand,
    FormsInspectEventsCommand,
    FormsGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormsResultStatus {
    Success,
    Paged,
    Partial,
    Asynchronous,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    ValidationFailed,
    PublishDenied,
    SubmitDenied,
    ExportDenied,
    WebhookDenied,
    WebhookSignatureInvalid,
    ResponseRedacted,
    Quota,
    Timeout,
    Cancellation,
    ApprovalRequired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormsResultEnvelope<T> {
    pub status: FormsResultStatus,
    pub data: Option<T>,
    pub page: Option<OfficePage<T>>,
    pub error: Option<OfficeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormsDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub form_version_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn office_forms_descriptor_hashes() -> FormsDescriptorHashes {
    FormsDescriptorHashes {
        command_schema_hash: forms_stable_hash(&OFFICE_FORMS_COMMANDS),
        result_schema_hash: forms_stable_hash(&FormsResultStatus::Success),
        descriptor_hash: forms_stable_hash(&office_forms_pack_definition()),
        provider_capability_schema_hash: forms_stable_hash(&FormsProviderCapability {
            provider_class: "mock".into(),
            field_types: BTreeSet::from(["text".into(), "choice".into(), "date".into()]),
            features: BTreeSet::from(["schema".into(), "responses".into(), "events".into()]),
            max_fields: 500,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        form_version_hash: forms_stable_hash(&FormHandle {
            form_id: "form".into(),
            schema_version_hash: "schema-v1".into(),
            published: false,
            scope: FormsScope::default(),
        }),
        unavailable_schema_hash: forms_stable_hash(&OfficeError {
            code: "unavailable".into(),
            message: "office forms provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("office_forms_provider_not_installed".into()),
        }),
    }
}

pub fn forms_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    office_stable_hash(value)
}
