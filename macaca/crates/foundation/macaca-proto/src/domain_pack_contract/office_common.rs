use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Declarative provider-class row used by Office pack descriptors.
///
/// Provider classes are capability shapes, not vendor names. Concrete document,
/// spreadsheet, presentation, PDF, form, drive, renderer, or conversion providers
/// must be installed by runtime-host composition roots and remain outside proto.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OfficeProviderClass<'a> {
    pub(crate) provider_class: &'a str,
    pub(crate) availability: DomainPackProviderCapabilityState,
    pub(crate) metadata: &'a [(&'a str, &'a str)],
}

/// Descriptor input for one Office sub-pack.
///
/// This is a small Bridge-pattern helper: child modules own typed DTOs, while
/// this common builder owns repeatable descriptor metadata and provider-class
/// projection. Adding a new Office pack should be data-driven, not a new
/// execution path.
pub(crate) struct OfficePackDescriptor<'a> {
    pub(crate) pack_id: &'a str,
    pub(crate) child_change_id: &'a str,
    pub(crate) docs_slug: &'a str,
    pub(crate) service_id: &'a str,
    pub(crate) commands: &'a [&'a str],
    pub(crate) permission_scopes: &'a [&'a str],
    pub(crate) provider_classes: &'a [OfficeProviderClass<'a>],
    pub(crate) health_probe: &'a str,
    pub(crate) unavailable_reason: &'a str,
    pub(crate) replay_schema: &'a str,
    pub(crate) data_classification: &'a str,
    pub(crate) retention_policy: &'a str,
    pub(crate) redaction_policy: &'a str,
    pub(crate) examples: &'a [&'a str],
    pub(crate) migration_notes: &'a [&'a str],
}

/// Build a descriptor-only Office pack definition.
///
/// The descriptor is preview-unavailable by default. SDKs and admission tooling
/// can discover schemas and docs immediately, while calls remain blocked until
/// an approved service provider registers matching command schemas.
pub(crate) fn office_pack_definition(spec: OfficePackDescriptor<'_>) -> DomainPackDefinition {
    let command_schemas = schema_set(spec.commands);
    let result_schemas = spec
        .commands
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        spec.pack_id,
        DomainPackMetadata {
            family_id: "office".into(),
            parent_pack_id: Some("pack.office.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(spec.service_id.into(), command_schemas)]),
            service_result_schemas: BTreeMap::from([(spec.service_id.into(), result_schemas)]),
            permission_scopes: schema_set(spec.permission_scopes),
            source_attribution: BTreeSet::from([
                "openspec:add-developer-pack-industrial-capability-catalog".into(),
                spec.child_change_id.into(),
            ]),
            migration_notes: spec
                .migration_notes
                .iter()
                .map(|note| (*note).into())
                .collect(),
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(60_000),
                max_retries: Some(0),
                budget_units: Some(2),
                allow_network: Some(true),
            },
            data_governance: DomainPackDataGovernance {
                classification: spec.data_classification.into(),
                retention_policy: spec.retention_policy.into(),
                redaction_policy: spec.redaction_policy.into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: format!("sdk.packs.office.{}", spec.docs_slug.replace('-', ".")),
                docs_url: format!("docs://macaca/developer-packs/office/{}", spec.docs_slug),
                examples: spec
                    .examples
                    .iter()
                    .map(|example| (*example).into())
                    .collect(),
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: spec.health_probe.into(),
                unavailable_reason: spec.unavailable_reason.into(),
                replay_schema: spec.replay_schema.into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(spec.service_id.into(), "^1".into())]),
            },
            provider_descriptors: provider_descriptors(&spec),
        },
        [spec.service_id.to_string()],
    )
}

fn provider_descriptors(
    spec: &OfficePackDescriptor<'_>,
) -> BTreeMap<String, DomainPackProviderDescriptor> {
    spec.provider_classes
        .iter()
        .map(|provider| {
            let descriptor = provider_descriptor(spec, provider);
            (descriptor.provider_class.clone(), descriptor)
        })
        .collect()
}

fn provider_descriptor(
    spec: &OfficePackDescriptor<'_>,
    provider: &OfficeProviderClass<'_>,
) -> DomainPackProviderDescriptor {
    let metadata = provider
        .metadata
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect::<BTreeMap<String, String>>();
    let capability = GenericOfficeProviderCapability {
        pack_id: spec.pack_id,
        provider_class: provider.provider_class,
        supported_commands: spec.commands,
        metadata: &metadata,
        availability: provider.availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider.provider_class.into(),
        service_id: spec.service_id.into(),
        availability: provider.availability,
        capability_hash: office_stable_hash(&capability),
        compatibility_hash: format!("office-{}-provider-v1", spec.docs_slug),
        diagnostics_schema: format!("{}.provider.diagnostics.v1", spec.docs_slug),
        metadata,
    }
}

#[derive(Serialize)]
struct GenericOfficeProviderCapability<'a> {
    pack_id: &'a str,
    provider_class: &'a str,
    supported_commands: &'a [&'a str],
    metadata: &'a BTreeMap<String, String>,
    availability: DomainPackProviderCapabilityState,
}

pub(crate) fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Compute a deterministic, trace-safe hash for descriptor compatibility tests.
///
/// This is audit evidence, not cryptography. Inputs must be descriptor and DTO
/// shapes only; raw Office documents, sheets, slides, PDFs, form responses,
/// provider payloads, exports, tokens, and private comments are forbidden.
pub(crate) fn office_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}

/// Opaque Office command envelope shared by typed command wrappers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeCommandEnvelope {
    pub subject_ref: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficePage<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_safe_detail: Option<String>,
}

macro_rules! define_office_command_wrappers {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name {
                pub request: OfficeCommandEnvelope,
            }
        )+
    };
}

pub(crate) use define_office_command_wrappers;
