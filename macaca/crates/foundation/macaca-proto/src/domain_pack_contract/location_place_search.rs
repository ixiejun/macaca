use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::location_common::{
    define_location_command_wrappers, location_pack_definition, location_stable_hash,
    LocationPackCommandEnvelope, LocationPackDescriptor, LocationPackError, LocationPackPage,
    LocationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const LOCATION_PLACE_SEARCH_PACK_ID: &str = "pack.location.place.search.v1";
pub const LOCATION_PLACE_SEARCH_SERVICE_ID: &str = "service.location.place_search";
pub const LOCATION_PLACE_SEARCH_TRACE_EVENTS: &[&str] = &[
    "place_search.pack_declared",
    "place_search.admission_validated",
    "place_search.policy_decision",
    "place_search.entitlement_checked",
    "place_search.resource_reserved",
    "place_search.command_requested",
    "place_search.provider_selected",
    "place_search.command_succeeded",
    "place_search.command_failed",
    "place_search.unavailable",
    "place_search.attribution_recorded",
    "place_search.session_purged",
    "place_search.snapshot_recorded",
];

pub const LOCATION_PLACE_SEARCH_COMMANDS: &[&str] = &[
    "place_search.search",
    "place_search.nearby",
    "place_search.suggest",
    "place_search.resolve_suggestion",
    "place_search.get_details",
    "place_search.list_categories",
    "place_search.inspect_fields",
    "place_search.inspect_attribution",
    "place_search.purge_session",
];

pub const PLACE_SEARCH_PERMISSION_SCOPES: &[&str] = &[
    "location.place.search.read",
    "location.place.autocomplete.read",
    "location.place.details.read",
    "location.place.categories.read",
    "location.place.media.reference.read",
    "location.place.session.manage",
];

const PLACE_TEXT_METADATA: &[(&str, &str)] = &[
    ("text_search", "planned"),
    ("query_text_in_trace", "hash_only"),
    ("application_ranking", "false"),
];
const PLACE_AUTOCOMPLETE_METADATA: &[(&str, &str)] = &[
    ("autocomplete", "planned"),
    ("session_tokens", "provider_owned"),
    ("purge_supported", "true"),
];
const PLACE_DETAILS_METADATA: &[(&str, &str)] = &[
    ("details", "field_mask_required"),
    ("media_references", "reference_only"),
    ("raw_media_bytes", "false"),
];
const PLACE_MOCK_METADATA: &[(&str, &str)] = &[
    ("fixtures", "synthetic"),
    ("callable", "false"),
    ("network", "false"),
];
const PLACE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("search", "false"),
    ("details", "false"),
    ("reason", "provider_not_installed"),
];

const PLACE_SEARCH_PROVIDER_CLASSES: &[LocationProviderClass<'_>] = &[
    LocationProviderClass {
        provider_class: "text-search",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLACE_TEXT_METADATA,
    },
    LocationProviderClass {
        provider_class: "autocomplete",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLACE_AUTOCOMPLETE_METADATA,
    },
    LocationProviderClass {
        provider_class: "place-details",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLACE_DETAILS_METADATA,
    },
    LocationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLACE_MOCK_METADATA,
    },
    LocationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: PLACE_UNAVAILABLE_METADATA,
    },
];

/// Build the place-search descriptor without binding POI search provider code.
pub fn location_place_search_pack_definition() -> DomainPackDefinition {
    location_pack_definition(LocationPackDescriptor {
        pack_id: LOCATION_PLACE_SEARCH_PACK_ID,
        child_change_id: "openspec:add-pack-location-place-search",
        docs_slug: "place-search",
        sdk_slug: "place.search",
        service_id: LOCATION_PLACE_SEARCH_SERVICE_ID,
        commands: LOCATION_PLACE_SEARCH_COMMANDS,
        permission_scopes: PLACE_SEARCH_PERMISSION_SCOPES,
        provider_classes: PLACE_SEARCH_PROVIDER_CLASSES,
        health_probe: "place_search.inspect_fields",
        unavailable_reason: "location_place_search_provider_not_installed",
        replay_schema: "location.place_search.replay.v1",
        data_classification: "location_place_intent_reference_metadata",
        retention_policy: "queries_suggestions_details_categories_attribution_sessions_and_artifacts_by_reference",
        redaction_policy: "query_text_exact_coordinates_media_references_provider_payloads_session_tokens_place_intent_and_credentials_redacted",
        timeout_ms: 120_000,
        budget_units: 6,
        examples: &[
            "Declare `pack.location.place.search.v1` as optional until a place-search provider is installed.",
            "Use field masks, categories, suggestions, place references, attribution records, and purgeable sessions instead of provider-native place payloads.",
        ],
        migration_notes: &[
            "Place-search commands become callable only after an approved service provider registers matching schemas.",
            "Maps, geocode, route, timezone, device location capture, booking, review authoring, and application-specific ranking remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchCommandContext {
    pub tenant_scope: String,
    pub locale: Option<String>,
    pub region_policy_ref: String,
    pub retention_policy_ref: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchProviderCapability {
    pub provider_class: String,
    pub supported_fields: BTreeSet<String>,
    pub supported_categories: BTreeSet<String>,
    pub cost_classes: BTreeMap<String, String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchSpatialConstraint {
    pub constraint_ref: String,
    pub constraint_kind: String,
    pub coordinate_precision_class: Option<String>,
    pub radius_meters: Option<u32>,
    pub boundary_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceQuery {
    pub query_ref: String,
    pub query_text_hash: Option<String>,
    pub categories: BTreeSet<String>,
    pub field_mask: BTreeSet<String>,
    pub spatial_constraint: Option<PlaceSearchSpatialConstraint>,
    pub page_size: u32,
}

impl PlaceQuery {
    /// Enforce field-mask and page-size bounds before provider dispatch.
    pub fn is_bounded(&self, max_fields: usize, max_page_size: u32) -> bool {
        !self.field_mask.is_empty()
            && self.field_mask.len() <= max_fields
            && self.page_size <= max_page_size
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSummary {
    pub place_ref: String,
    pub display_label_ref: String,
    pub category_refs: BTreeSet<String>,
    pub quality: PlaceSearchQuality,
    pub attribution_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceDetails {
    pub place_ref: String,
    pub fields: BTreeMap<String, String>,
    pub media_refs: Vec<PlaceMediaReference>,
    pub external_refs: Vec<PlaceExternalReference>,
    pub attribution: PlaceAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSuggestion {
    pub suggestion_ref: String,
    pub session_ref: String,
    pub display_label_ref: String,
    pub resolve_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceCategory {
    pub category_ref: String,
    pub parent_ref: Option<String>,
    pub label_ref: String,
    pub provider_mapping_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceAttribution {
    pub attribution_ref: String,
    pub display_required: bool,
    pub license_refs: BTreeSet<String>,
    pub retention_note_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchQuality {
    pub score_basis_points: u16,
    pub quality_class: String,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceMediaReference {
    pub media_ref: String,
    pub media_kind: String,
    pub access_policy_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceExternalReference {
    pub reference_ref: String,
    pub reference_kind: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_location_command_wrappers!(
    PlaceSearchSearchCommand,
    PlaceSearchNearbyCommand,
    PlaceSearchSuggestCommand,
    PlaceSearchResolveSuggestionCommand,
    PlaceSearchGetDetailsCommand,
    PlaceSearchListCategoriesCommand,
    PlaceSearchInspectFieldsCommand,
    PlaceSearchInspectAttributionCommand,
    PlaceSearchPurgeSessionCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceSearchResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    QuotaExceeded,
    StaleReference,
    AmbiguousReference,
    EntitlementRequired,
    AttributionRequired,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchResultEnvelope<T> {
    pub status: PlaceSearchResultStatus,
    pub data: Option<T>,
    pub page: Option<LocationPackPage<T>>,
    pub error: Option<LocationPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSearchDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub field_capability_hash: String,
    pub category_hash: String,
    pub attribution_hash: String,
    pub session_policy_hash: String,
    pub redaction_profile_hash: String,
}

pub fn location_place_search_descriptor_hashes() -> PlaceSearchDescriptorHashes {
    PlaceSearchDescriptorHashes {
        command_schema_hash: place_search_stable_hash(&LOCATION_PLACE_SEARCH_COMMANDS),
        result_schema_hash: place_search_stable_hash(&PlaceSearchResultStatus::Success),
        descriptor_hash: place_search_stable_hash(&location_place_search_pack_definition()),
        provider_capability_hash: place_search_stable_hash(&PlaceSearchProviderCapability {
            provider_class: "mock".into(),
            supported_fields: BTreeSet::from(["summary".into(), "category".into()]),
            supported_categories: BTreeSet::from(["synthetic".into()]),
            cost_classes: BTreeMap::from([("details".into(), "metered".into())]),
            limits: BTreeMap::from([("max_page_size".into(), 20)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        field_capability_hash: place_search_stable_hash(&BTreeSet::from([
            "summary".to_string(),
            "media_reference".to_string(),
        ])),
        category_hash: place_search_stable_hash(&PlaceCategory {
            category_ref: "category".into(),
            parent_ref: None,
            label_ref: "label".into(),
            provider_mapping_ref: "mapping".into(),
        }),
        attribution_hash: place_search_stable_hash(&PlaceAttribution {
            attribution_ref: "attr".into(),
            display_required: true,
            license_refs: BTreeSet::from(["license".into()]),
            retention_note_ref: "retention".into(),
        }),
        session_policy_hash: place_search_stable_hash("place-session-ephemeral"),
        redaction_profile_hash: place_search_stable_hash("place-search-redaction-v1"),
    }
}

pub fn place_search_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    location_stable_hash(value)
}

/// Build a synthetic purge command without exposing provider session tokens.
pub fn synthetic_place_search_purge_command(
    session_ref: impl Into<String>,
) -> PlaceSearchPurgeSessionCommand {
    PlaceSearchPurgeSessionCommand {
        request: LocationPackCommandEnvelope {
            subject_ref: "synthetic-place-search-session".into(),
            parameters: BTreeMap::from([("session_ref".into(), session_ref.into())]),
            cursor: None,
            page_size: None,
            idempotency_key: None,
        },
    }
}
