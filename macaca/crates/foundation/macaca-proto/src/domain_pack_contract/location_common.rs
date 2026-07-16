use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Declarative provider-class row used by Location child-pack descriptors.
///
/// Provider classes describe replaceable capability shapes such as tile
/// planning, geocode lookup, or timezone database inspection. They are not
/// vendor names and they never authorize a direct provider call. Runtime-host
/// and plugin composition roots bind concrete adapters after policy, resource,
/// entitlement, trace, audit, and redaction decorators are present.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocationProviderClass<'a> {
    pub(crate) provider_class: &'a str,
    pub(crate) availability: DomainPackProviderCapabilityState,
    pub(crate) metadata: &'a [(&'a str, &'a str)],
}

/// Descriptor input for one Location sub-pack.
///
/// This Bridge-pattern helper keeps descriptor assembly data-driven while each
/// child module owns its Command-pattern DTOs. Adding a new Location sub-pack
/// should extend declarative data and tests, not add a second execution path or
/// a branch on provider, application, city, country, or business workflow.
pub(crate) struct LocationPackDescriptor<'a> {
    pub(crate) pack_id: &'a str,
    pub(crate) child_change_id: &'a str,
    pub(crate) docs_slug: &'a str,
    pub(crate) sdk_slug: &'a str,
    pub(crate) service_id: &'a str,
    pub(crate) commands: &'a [&'a str],
    pub(crate) permission_scopes: &'a [&'a str],
    pub(crate) provider_classes: &'a [LocationProviderClass<'a>],
    pub(crate) health_probe: &'a str,
    pub(crate) unavailable_reason: &'a str,
    pub(crate) replay_schema: &'a str,
    pub(crate) data_classification: &'a str,
    pub(crate) retention_policy: &'a str,
    pub(crate) redaction_policy: &'a str,
    pub(crate) timeout_ms: u64,
    pub(crate) budget_units: u64,
    pub(crate) examples: &'a [&'a str],
    pub(crate) migration_notes: &'a [&'a str],
}

/// Build a descriptor-only Location pack definition.
///
/// Location data can reveal private movement, addresses, routes, place intent,
/// and exact coordinates. These descriptors are therefore preview-unavailable
/// until an approved service provider registers matching command schemas through
/// the canonical service runtime. Proto owns only provider-neutral contracts,
/// compatibility evidence, and bounded diagnostics.
pub(crate) fn location_pack_definition(spec: LocationPackDescriptor<'_>) -> DomainPackDefinition {
    let command_schemas = schema_set(spec.commands);
    let result_schemas = spec
        .commands
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        spec.pack_id,
        DomainPackMetadata {
            family_id: "location".into(),
            parent_pack_id: Some("pack.location.v1".into()),
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
                timeout_ms: Some(spec.timeout_ms),
                max_retries: Some(0),
                budget_units: Some(spec.budget_units),
                allow_network: Some(true),
            },
            data_governance: DomainPackDataGovernance {
                classification: spec.data_classification.into(),
                retention_policy: spec.retention_policy.into(),
                redaction_policy: spec.redaction_policy.into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: format!("sdk.packs.location.{}", spec.sdk_slug),
                docs_url: format!("docs://macaca/developer-packs/location/{}", spec.docs_slug),
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
    spec: &LocationPackDescriptor<'_>,
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
    spec: &LocationPackDescriptor<'_>,
    provider: &LocationProviderClass<'_>,
) -> DomainPackProviderDescriptor {
    let metadata = provider
        .metadata
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect::<BTreeMap<String, String>>();
    let capability = GenericLocationProviderCapability {
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
        capability_hash: location_stable_hash(&capability),
        compatibility_hash: format!("location-{}-provider-v1", spec.docs_slug),
        diagnostics_schema: format!("{}.provider.diagnostics.v1", spec.docs_slug),
        metadata,
    }
}

#[derive(Serialize)]
struct GenericLocationProviderCapability<'a> {
    pack_id: &'a str,
    provider_class: &'a str,
    supported_commands: &'a [&'a str],
    metadata: &'a BTreeMap<String, String>,
    availability: DomainPackProviderCapabilityState,
}

pub(crate) fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Compute deterministic, trace-safe compatibility evidence for Location DTOs.
///
/// This hash is audit identity, not cryptography. Inputs must be bounded
/// descriptors, references, handles, and redacted plans. Raw provider payloads,
/// raw map tiles, raw boundary geometry, precise private coordinates, private
/// address lists, credentials, session tokens, and unbounded route/place data
/// must never enter hash evidence.
pub(crate) fn location_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}

/// Opaque Location command envelope shared by typed command wrappers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationPackCommandEnvelope {
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

/// Bounded page shared by Location result DTOs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationPackPage<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub truncated: bool,
}

/// Sanitized Location error payload for trace-safe diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationPackError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_safe_detail: Option<String>,
}

macro_rules! define_location_command_wrappers {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name {
                pub request: LocationPackCommandEnvelope,
            }
        )+
    };
}

pub(crate) use define_location_command_wrappers;
