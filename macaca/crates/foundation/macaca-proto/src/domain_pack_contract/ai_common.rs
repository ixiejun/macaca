use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Declarative provider-class row used by AI child-pack descriptors.
///
/// Provider classes describe replaceable capability shapes such as hosted model,
/// local runtime, remote service, plugin, mock, and unavailable. They are
/// descriptor metadata only; runtime-host and plugin composition roots own
/// concrete provider adapters after trace, policy, entitlement, resource,
/// metering, approval, and redaction decorators are installed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AiProviderClass<'a> {
    pub(crate) provider_class: &'a str,
    pub(crate) availability: DomainPackProviderCapabilityState,
    pub(crate) metadata: &'a [(&'a str, &'a str)],
}

/// Descriptor input for one AI sub-pack.
///
/// This Bridge-pattern helper keeps descriptor assembly declarative while child
/// modules own their Command-pattern DTOs. Adding a new AI pack should add data
/// and tests, not branches on provider, model, application, workflow, prompt,
/// customer, dataset, or business-domain names.
pub(crate) struct AiPackDescriptor<'a> {
    pub(crate) pack_id: &'a str,
    pub(crate) child_change_id: &'a str,
    pub(crate) docs_slug: &'a str,
    pub(crate) sdk_slug: &'a str,
    pub(crate) service_id: &'a str,
    pub(crate) commands: &'a [&'a str],
    pub(crate) permission_scopes: &'a [&'a str],
    pub(crate) provider_classes: &'a [AiProviderClass<'a>],
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

/// Build a descriptor-only AI pack definition.
///
/// AI packs can process prompts, private documents, media, model outputs,
/// datasets, embeddings, evaluation reports, and provider diagnostics. The
/// descriptor therefore remains preview-unavailable until an approved
/// serviceized provider registers matching command schemas through the
/// canonical service runtime.
pub(crate) fn ai_pack_definition(spec: AiPackDescriptor<'_>) -> DomainPackDefinition {
    let command_schemas = schema_set(spec.commands);
    let result_schemas = spec
        .commands
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        spec.pack_id,
        DomainPackMetadata {
            family_id: "ai".into(),
            parent_pack_id: Some("pack.ai.v1".into()),
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
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: spec.data_classification.into(),
                retention_policy: spec.retention_policy.into(),
                redaction_policy: spec.redaction_policy.into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: format!("sdk.packs.ai.{}", spec.sdk_slug),
                docs_url: format!("docs://macaca/developer-packs/ai/{}", spec.docs_slug),
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
    spec: &AiPackDescriptor<'_>,
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
    spec: &AiPackDescriptor<'_>,
    provider: &AiProviderClass<'_>,
) -> DomainPackProviderDescriptor {
    let metadata = provider
        .metadata
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect::<BTreeMap<String, String>>();
    let capability = GenericAiProviderCapability {
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
        capability_hash: ai_stable_hash(&capability),
        compatibility_hash: format!("ai-{}-provider-v1", spec.docs_slug),
        diagnostics_schema: format!("{}.provider.diagnostics.v1", spec.docs_slug),
        metadata,
    }
}

#[derive(Serialize)]
struct GenericAiProviderCapability<'a> {
    pack_id: &'a str,
    provider_class: &'a str,
    supported_commands: &'a [&'a str],
    metadata: &'a BTreeMap<String, String>,
    availability: DomainPackProviderCapabilityState,
}

pub(crate) fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Validate a bounded reference token before it can enter trace, audit, or snapshots.
///
/// AI pack contracts intentionally exchange references and hashes instead of
/// raw prompts, media bytes, audio, vectors, datasets, model names, credentials,
/// provider payloads, or unbounded diagnostics.
pub(crate) fn ai_bounded_token(value: &str, max_len: usize) -> bool {
    !value.is_empty() && value.len() <= max_len && !value.contains('\n')
}

/// Compute deterministic, trace-safe compatibility evidence for AI DTOs.
///
/// This hash is audit identity, not cryptography. Inputs must remain bounded
/// and reference-only: raw prompts, raw completions, raw images, raw audio,
/// embedding vectors, datasets, provider payloads, credentials, secrets, model
/// names, and unbounded evaluation artifacts must never enter hash evidence.
pub(crate) fn ai_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}

/// Opaque AI command envelope shared by typed command wrappers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackCommandEnvelope {
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

/// Bounded page shared by AI result DTOs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackPage<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub truncated: bool,
}

/// Sanitized AI error payload for trace-safe diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_safe_detail: Option<String>,
}

macro_rules! define_ai_command_wrappers {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name {
                pub request: AiPackCommandEnvelope,
            }
        )+
    };
}

pub(crate) use define_ai_command_wrappers;
