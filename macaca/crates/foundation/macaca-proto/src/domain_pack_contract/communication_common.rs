use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Declarative provider-class row used by communication pack descriptors.
///
/// The class labels are intentionally generic capability classes. Concrete provider names,
/// credentials, endpoints, and product-specific payloads belong to service providers or optional
/// plugin packages, never to the shared protocol contract.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommunicationProviderClass<'a> {
    pub(crate) provider_class: &'a str,
    pub(crate) availability: DomainPackProviderCapabilityState,
    pub(crate) metadata: &'a [(&'a str, &'a str)],
}

/// Descriptor input for one communication pack.
///
/// This tiny builder input keeps the Communication family open for extension without requiring
/// hardcoded construction logic in the industrial catalog. Each pack module owns its typed DTOs,
/// while this helper owns the repeated descriptor assembly mechanics.
pub(crate) struct CommunicationPackDescriptor<'a> {
    pub(crate) slug: &'a str,
    pub(crate) service_id: &'a str,
    pub(crate) commands: &'a [&'a str],
    pub(crate) permission_scopes: &'a [&'a str],
    pub(crate) provider_classes: &'a [CommunicationProviderClass<'a>],
    pub(crate) health_probe: &'a str,
    pub(crate) unavailable_reason: &'a str,
    pub(crate) replay_schema: &'a str,
    pub(crate) data_classification: &'a str,
    pub(crate) retention_policy: &'a str,
    pub(crate) redaction_policy: &'a str,
    pub(crate) examples: &'a [&'a str],
    pub(crate) migration_notes: &'a [&'a str],
}

/// Build a descriptor-only communication pack definition.
///
/// The returned descriptor is preview-unavailable by default. It is discoverable by SDK clients
/// and admission tooling, but it cannot be invoked until an approved service provider registers a
/// callable descriptor through the runtime-host composition root.
pub(crate) fn communication_pack_definition(
    spec: CommunicationPackDescriptor<'_>,
) -> DomainPackDefinition {
    let pack_id = format!("pack.communication.{}.v1", spec.slug);
    let command_schemas = schema_set(spec.commands);
    let result_schemas = spec
        .commands
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        pack_id,
        DomainPackMetadata {
            family_id: "communication".into(),
            parent_pack_id: Some("pack.communication.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(spec.service_id.into(), command_schemas)]),
            service_result_schemas: BTreeMap::from([(spec.service_id.into(), result_schemas)]),
            permission_scopes: schema_set(spec.permission_scopes),
            source_attribution: BTreeSet::from([
                "openspec:add-developer-pack-industrial-capability-catalog".into(),
                format!("openspec:add-pack-communication-{}", spec.slug),
            ]),
            migration_notes: spec
                .migration_notes
                .iter()
                .map(|note| (*note).into())
                .collect(),
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(30_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: Some(true),
            },
            data_governance: DomainPackDataGovernance {
                classification: spec.data_classification.into(),
                retention_policy: spec.retention_policy.into(),
                redaction_policy: spec.redaction_policy.into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: format!("sdk.packs.communication.{}", spec.slug),
                docs_url: format!("docs://macaca/developer-packs/communication/{}", spec.slug),
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
    spec: &CommunicationPackDescriptor<'_>,
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
    spec: &CommunicationPackDescriptor<'_>,
    provider: &CommunicationProviderClass<'_>,
) -> DomainPackProviderDescriptor {
    let metadata = provider
        .metadata
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect::<BTreeMap<String, String>>();
    let capability = GenericCommunicationProviderCapability {
        slug: spec.slug,
        provider_class: provider.provider_class,
        supported_commands: spec.commands,
        metadata: &metadata,
        availability: provider.availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider.provider_class.into(),
        service_id: spec.service_id.into(),
        availability: provider.availability,
        capability_hash: communication_stable_hash(&capability),
        compatibility_hash: format!("communication-{}-provider-v1", spec.slug),
        diagnostics_schema: format!("{}.provider.diagnostics.v1", spec.slug),
        metadata,
    }
}

#[derive(Serialize)]
struct GenericCommunicationProviderCapability<'a> {
    slug: &'a str,
    provider_class: &'a str,
    supported_commands: &'a [&'a str],
    metadata: &'a BTreeMap<String, String>,
    availability: DomainPackProviderCapabilityState,
}

pub(crate) fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Compute a deterministic, non-secret hash for descriptor and DTO compatibility tests.
///
/// This hash is audit evidence, not a cryptographic primitive. It deliberately hashes only
/// already-sanitized descriptor/DTO shapes so tests and SDK discovery can compare schemas without
/// logging raw credentials, provider payloads, message bodies, attachments, calendar exports, or
/// other application content.
pub(crate) fn communication_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}

/// Validate bounded communication references before they can enter trace or SDK diagnostics.
///
/// Communication packs carry handles, hashes, cursors, and content references. They must never
/// carry raw provider URLs, OAuth tokens, webhook bodies, message bodies, invite payloads, or
/// attachment bytes through shared protocol DTOs.
pub(crate) fn bounded_communication_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}

/// Validate optional credential pointers as secret-store references only.
pub(crate) fn optional_secret_reference_is_safe(secret_ref: Option<&str>) -> bool {
    secret_ref.is_none_or(|reference| {
        bounded_communication_token(reference, 256)
            && matches!(
                reference.split_once(':').map(|(prefix, _)| prefix),
                Some("secret" | "vault" | "kms")
            )
    })
}
