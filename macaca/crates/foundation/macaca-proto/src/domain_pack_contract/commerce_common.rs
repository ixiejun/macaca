use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Declarative provider-class row used by Commerce child-pack descriptors.
///
/// Provider classes describe replaceable capability shapes such as catalog read
/// models or payment-intent state machines. They are not vendor identifiers, and
/// concrete adapters must be bound only by runtime-host or plugin composition
/// roots after policy, entitlement, trace, and resource decorators are present.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommerceProviderClass<'a> {
    pub(crate) provider_class: &'a str,
    pub(crate) availability: DomainPackProviderCapabilityState,
    pub(crate) metadata: &'a [(&'a str, &'a str)],
}

/// Descriptor input for one Commerce sub-pack.
///
/// This Bridge-pattern helper keeps descriptor assembly data-driven while each
/// child module owns its Command-pattern DTOs. Adding a new Commerce sub-pack
/// should add declarative data and tests, not a second execution path.
pub(crate) struct CommercePackDescriptor<'a> {
    pub(crate) pack_id: &'a str,
    pub(crate) child_change_id: &'a str,
    pub(crate) docs_slug: &'a str,
    pub(crate) sdk_slug: &'a str,
    pub(crate) service_id: &'a str,
    pub(crate) commands: &'a [&'a str],
    pub(crate) permission_scopes: &'a [&'a str],
    pub(crate) provider_classes: &'a [CommerceProviderClass<'a>],
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

/// Build a descriptor-only Commerce pack definition.
///
/// The descriptor stays preview-unavailable until an approved service provider
/// registers compatible command schemas. This keeps SDK discovery, admission
/// diagnostics, and developer documentation useful without making proto own
/// commerce execution, checkout workflows, payment gateways, or provider code.
pub(crate) fn commerce_pack_definition(spec: CommercePackDescriptor<'_>) -> DomainPackDefinition {
    let command_schemas = schema_set(spec.commands);
    let result_schemas = spec
        .commands
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        spec.pack_id,
        DomainPackMetadata {
            family_id: "commerce".into(),
            parent_pack_id: Some("pack.commerce.v1".into()),
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
                client_namespace: format!("sdk.packs.commerce.{}", spec.sdk_slug),
                docs_url: format!("docs://macaca/developer-packs/commerce/{}", spec.docs_slug),
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
    spec: &CommercePackDescriptor<'_>,
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
    spec: &CommercePackDescriptor<'_>,
    provider: &CommerceProviderClass<'_>,
) -> DomainPackProviderDescriptor {
    let metadata = provider
        .metadata
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect::<BTreeMap<String, String>>();
    let capability = GenericCommerceProviderCapability {
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
        capability_hash: commerce_stable_hash(&capability),
        compatibility_hash: format!("commerce-{}-provider-v1", spec.docs_slug),
        diagnostics_schema: format!("{}.provider.diagnostics.v1", spec.docs_slug),
        metadata,
    }
}

#[derive(Serialize)]
struct GenericCommerceProviderCapability<'a> {
    pack_id: &'a str,
    provider_class: &'a str,
    supported_commands: &'a [&'a str],
    metadata: &'a BTreeMap<String, String>,
    availability: DomainPackProviderCapabilityState,
}

pub(crate) fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Compute deterministic, trace-safe compatibility evidence for Commerce DTOs.
///
/// This hash is audit identity, not cryptography. Inputs must be descriptors,
/// handles, bounded plans, references, and hashes. Raw buyer PII, payment
/// credentials, provider payloads, webhook bodies, checkout URLs, receipts,
/// printable documents, signed store payloads, license secrets, and unbounded
/// exports must never be hashed into trace or SDK evidence.
pub(crate) fn commerce_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}

fn bounded_commerce_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}

/// Opaque Commerce command envelope shared by typed command wrappers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommercePackCommandEnvelope {
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

impl CommercePackCommandEnvelope {
    /// Validate generic command guards before a commerce provider Strategy can run.
    ///
    /// The envelope intentionally contains only references, cursors, and bounded
    /// parameters. This helper lets pack-specific preflight code reject oversized
    /// pagination, missing subjects, and unsafe idempotency material without
    /// parsing provider payloads or branching on concrete vendor behavior.
    pub fn has_bounded_preconditions(&self, max_page_size: u32, max_parameters: usize) -> bool {
        bounded_commerce_token(&self.subject_ref, 256)
            && self.parameters.len() <= max_parameters
            && self.parameters.iter().all(|(key, value)| {
                bounded_commerce_token(key, 96) && bounded_commerce_token(value, 512)
            })
            && self
                .cursor
                .as_deref()
                .is_none_or(|cursor| bounded_commerce_token(cursor, 256))
            && self
                .page_size
                .is_none_or(|page_size| page_size > 0 && page_size <= max_page_size)
            && self
                .idempotency_key
                .as_deref()
                .is_none_or(|key| bounded_commerce_token(key, 128))
    }
}

/// Bounded page shared by Commerce result DTOs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommercePackPage<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub truncated: bool,
}

/// Sanitized Commerce error payload for trace-safe diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommercePackError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_safe_detail: Option<String>,
}

macro_rules! define_commerce_command_wrappers {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name {
                pub request: CommercePackCommandEnvelope,
            }
        )+
    };
}

pub(crate) use define_commerce_command_wrappers;
