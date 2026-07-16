use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::location_common::{
    define_location_command_wrappers, location_pack_definition, location_stable_hash,
    LocationPackCommandEnvelope, LocationPackDescriptor, LocationPackError, LocationPackPage,
    LocationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const LOCATION_GEOCODE_PACK_ID: &str = "pack.location.geocode.v1";
pub const LOCATION_GEOCODE_SERVICE_ID: &str = "service.location.geocode";

pub const LOCATION_GEOCODE_COMMANDS: &[&str] = &[
    "geocode.inspect_provider",
    "geocode.discover_schema",
    "geocode.validate_query",
    "geocode.forward",
    "geocode.reverse",
    "geocode.normalize_address",
    "geocode.inspect_confidence",
    "geocode.plan_batch",
    "geocode.request_batch",
    "geocode.inspect_batch",
    "geocode.cancel_batch",
    "geocode.inspect_retention",
    "geocode.inspect_attribution",
    "geocode.get_artifact",
];

const GEOCODE_PERMISSION_SCOPES: &[&str] = &[
    "location.geocode.forward",
    "location.geocode.reverse",
    "location.geocode.normalize",
    "location.geocode.confidence.read",
    "location.geocode.batch",
    "location.geocode.retention.read",
    "location.geocode.attribution.read",
    "location.geocode.artifact.read",
];

const GEOCODE_FORWARD_METADATA: &[(&str, &str)] = &[
    ("forward", "planned"),
    ("structured_address", "reference_only"),
    ("raw_addresses_in_trace", "false"),
];
const GEOCODE_REVERSE_METADATA: &[(&str, &str)] = &[
    ("reverse", "planned"),
    ("precision_enforced", "true"),
    ("private_coordinates", "redacted"),
];
const GEOCODE_BATCH_METADATA: &[(&str, &str)] = &[
    ("batch", "async_planned"),
    ("retention", "policy_bound"),
    ("artifact_handles", "reference_only"),
];
const GEOCODE_MOCK_METADATA: &[(&str, &str)] = &[
    ("fixtures", "synthetic"),
    ("callable", "false"),
    ("network", "false"),
];
const GEOCODE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("forward", "false"),
    ("reverse", "false"),
    ("reason", "provider_not_installed"),
];

const GEOCODE_PROVIDER_CLASSES: &[LocationProviderClass<'_>] = &[
    LocationProviderClass {
        provider_class: "forward-geocoder",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: GEOCODE_FORWARD_METADATA,
    },
    LocationProviderClass {
        provider_class: "reverse-geocoder",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: GEOCODE_REVERSE_METADATA,
    },
    LocationProviderClass {
        provider_class: "batch-geocoder",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: GEOCODE_BATCH_METADATA,
    },
    LocationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: GEOCODE_MOCK_METADATA,
    },
    LocationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: GEOCODE_UNAVAILABLE_METADATA,
    },
];

/// Build the geocode descriptor without binding address lookup or provider code.
pub fn location_geocode_pack_definition() -> DomainPackDefinition {
    location_pack_definition(LocationPackDescriptor {
        pack_id: LOCATION_GEOCODE_PACK_ID,
        child_change_id: "openspec:add-pack-location-geocode",
        docs_slug: "geocode",
        sdk_slug: "geocode",
        service_id: LOCATION_GEOCODE_SERVICE_ID,
        commands: LOCATION_GEOCODE_COMMANDS,
        permission_scopes: GEOCODE_PERMISSION_SCOPES,
        provider_classes: GEOCODE_PROVIDER_CLASSES,
        health_probe: "geocode.inspect_provider",
        unavailable_reason: "location_geocode_provider_not_installed",
        replay_schema: "location.geocode.replay.v1",
        data_classification: "location_address_coordinate_reference_metadata",
        retention_policy: "queries_candidates_confidence_retention_attribution_batch_and_artifact_metadata_by_reference",
        redaction_policy: "raw_addresses_private_coordinates_provider_payloads_address_lists_artifacts_and_credentials_redacted",
        timeout_ms: 120_000,
        budget_units: 5,
        examples: &[
            "Declare `pack.location.geocode.v1` as optional until a geocode provider is installed.",
            "Use normalized candidates, confidence metadata, precision classes, retention plans, and artifact handles instead of raw provider address payloads.",
        ],
        migration_notes: &[
            "Geocode commands become callable only after an approved geocode service provider registers matching schemas.",
            "Maps, route, place search, timezone lookup, identity verification, and application address workflows remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeScope {
    pub tenant_scope: String,
    pub country_hint: Option<String>,
    pub language_hint: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeProviderCapability {
    pub provider_class: String,
    pub supported_countries: BTreeSet<String>,
    pub supported_languages: BTreeSet<String>,
    pub supported_precision_classes: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeQuery {
    pub query_ref: String,
    pub freeform_text_hash: Option<String>,
    pub structured_component_refs: Vec<String>,
    pub country_filter: Option<String>,
    pub language: Option<String>,
    pub retention_policy_ref: String,
}

impl GeocodeQuery {
    /// Bound query complexity before external lookup or trace evidence is created.
    pub fn is_bounded(&self, max_components: usize) -> bool {
        self.structured_component_refs.len() <= max_components
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReverseGeocodeQuery {
    pub query_ref: String,
    pub coordinate_ref: String,
    pub precision_class: String,
    pub result_field_mask: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressComponentSet {
    pub component_set_ref: String,
    pub components: BTreeMap<String, String>,
    pub normalized_label_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeGeometry {
    pub geometry_ref: String,
    pub coordinate_ref: String,
    pub precision_class: String,
    pub viewport_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationPrecisionClass {
    pub class_ref: String,
    pub name: String,
    pub exact_coordinate_allowed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeConfidence {
    pub score_basis_points: u16,
    pub confidence_class: String,
    pub ambiguity_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeCandidate {
    pub candidate_ref: String,
    pub address: AddressComponentSet,
    pub geometry: GeocodeGeometry,
    pub confidence: GeocodeConfidence,
    pub attribution_ref: String,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeRetentionPolicy {
    pub policy_ref: String,
    pub storage_mode: String,
    pub ttl_seconds: Option<u64>,
    pub permanent_storage_allowed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeBatchJob {
    pub job_ref: String,
    pub query_count: u32,
    pub state: String,
    pub artifact_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_location_command_wrappers!(
    GeocodeInspectProviderCommand,
    GeocodeDiscoverSchemaCommand,
    GeocodeValidateQueryCommand,
    GeocodeForwardCommand,
    GeocodeReverseCommand,
    GeocodeNormalizeAddressCommand,
    GeocodeInspectConfidenceCommand,
    GeocodePlanBatchCommand,
    GeocodeRequestBatchCommand,
    GeocodeInspectBatchCommand,
    GeocodeCancelBatchCommand,
    GeocodeInspectRetentionCommand,
    GeocodeInspectAttributionCommand,
    GeocodeGetArtifactCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeocodeResultStatus {
    Success,
    Paged,
    Partial,
    Accepted,
    ApprovalRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    Ambiguous,
    NoMatch,
    StaleVersion,
    QuotaExceeded,
    RateLimited,
    Timeout,
    Cancelled,
    RetentionDenied,
    AttributionMissing,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeResultEnvelope<T> {
    pub status: GeocodeResultStatus,
    pub data: Option<T>,
    pub page: Option<LocationPackPage<T>>,
    pub error: Option<LocationPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeocodeDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub supported_locale_hash: String,
    pub precision_class_hash: String,
    pub retention_policy_hash: String,
    pub attribution_bundle_hash: String,
    pub redaction_profile_hash: String,
}

pub fn location_geocode_descriptor_hashes() -> GeocodeDescriptorHashes {
    GeocodeDescriptorHashes {
        command_schema_hash: geocode_stable_hash(&LOCATION_GEOCODE_COMMANDS),
        result_schema_hash: geocode_stable_hash(&GeocodeResultStatus::Success),
        descriptor_hash: geocode_stable_hash(&location_geocode_pack_definition()),
        provider_capability_hash: geocode_stable_hash(&GeocodeProviderCapability {
            provider_class: "mock".into(),
            supported_countries: BTreeSet::from(["ZZ".into()]),
            supported_languages: BTreeSet::from(["en".into()]),
            supported_precision_classes: BTreeSet::from(["city".into(), "street".into()]),
            limits: BTreeMap::from([("max_batch_size".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        supported_locale_hash: geocode_stable_hash(&BTreeSet::from(["ZZ/en".to_string()])),
        precision_class_hash: geocode_stable_hash(&LocationPrecisionClass {
            class_ref: "street".into(),
            name: "street".into(),
            exact_coordinate_allowed: false,
        }),
        retention_policy_hash: geocode_stable_hash(&GeocodeRetentionPolicy {
            policy_ref: "ephemeral".into(),
            storage_mode: "ephemeral".into(),
            ttl_seconds: Some(3600),
            permanent_storage_allowed: false,
        }),
        attribution_bundle_hash: geocode_stable_hash("geocode-attribution-reference"),
        redaction_profile_hash: geocode_stable_hash("geocode-redaction-v1"),
    }
}

pub fn geocode_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    location_stable_hash(value)
}
