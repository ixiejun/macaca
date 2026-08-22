use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::location_common::{
    define_location_command_wrappers, location_pack_definition, location_stable_hash,
    LocationPackCommandEnvelope, LocationPackDescriptor, LocationPackError, LocationPackPage,
    LocationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const LOCATION_TIMEZONE_PACK_ID: &str = "pack.location.timezone.v1";
pub const LOCATION_TIMEZONE_SERVICE_ID: &str = "service.location.timezone";
pub const LOCATION_TIMEZONE_TRACE_EVENTS: &[&str] = &[
    "timezone.pack_declared",
    "timezone.admission_validated",
    "timezone.policy_decision",
    "timezone.entitlement_checked",
    "timezone.resource_reserved",
    "timezone.command_requested",
    "timezone.provider_selected",
    "timezone.command_succeeded",
    "timezone.command_failed",
    "timezone.unavailable",
    "timezone.database_stale",
    "timezone.snapshot_recorded",
];

pub const LOCATION_TIMEZONE_COMMANDS: &[&str] = &[
    "timezone.lookup_by_coordinates",
    "timezone.resolve_zone",
    "timezone.get_offset",
    "timezone.list_transitions",
    "timezone.convert_instant",
    "timezone.resolve_local_time",
    "timezone.get_display_names",
    "timezone.inspect_database",
    "timezone.inspect_mapping",
];

pub const TIMEZONE_PERMISSION_SCOPES: &[&str] = &[
    "location.timezone.lookup.read",
    "location.timezone.offset.read",
    "location.timezone.names.read",
    "location.timezone.database.inspect",
];

const TIMEZONE_TZDB_METADATA: &[(&str, &str)] = &[
    ("tzdb", "declared"),
    ("dataset_version", "reported"),
    ("database_paths", "redacted"),
];
const TIMEZONE_BOUNDARY_METADATA: &[(&str, &str)] = &[
    ("boundary_lookup", "planned"),
    ("raw_boundary_geometry", "false"),
    ("precision_enforced", "true"),
];
const TIMEZONE_MAPPING_METADATA: &[(&str, &str)] = &[
    ("iana_windows_mapping", "planned"),
    ("display_names", "planned"),
    ("cldr_refs", "reference_only"),
];
const TIMEZONE_MOCK_METADATA: &[(&str, &str)] = &[
    ("fixtures", "synthetic"),
    ("callable", "false"),
    ("network", "false"),
];
const TIMEZONE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("lookup", "false"),
    ("offset", "false"),
    ("reason", "provider_not_installed"),
];

const TIMEZONE_PROVIDER_CLASSES: &[LocationProviderClass<'_>] = &[
    LocationProviderClass {
        provider_class: "timezone-database",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TIMEZONE_TZDB_METADATA,
    },
    LocationProviderClass {
        provider_class: "boundary-lookup",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TIMEZONE_BOUNDARY_METADATA,
    },
    LocationProviderClass {
        provider_class: "identifier-mapping",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TIMEZONE_MAPPING_METADATA,
    },
    LocationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TIMEZONE_MOCK_METADATA,
    },
    LocationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: TIMEZONE_UNAVAILABLE_METADATA,
    },
];

/// Build the timezone descriptor without binding tzdb, boundary, host-native, or API providers.
pub fn location_timezone_pack_definition() -> DomainPackDefinition {
    location_pack_definition(LocationPackDescriptor {
        pack_id: LOCATION_TIMEZONE_PACK_ID,
        child_change_id: "openspec:add-pack-location-timezone",
        docs_slug: "timezone",
        sdk_slug: "timezone",
        service_id: LOCATION_TIMEZONE_SERVICE_ID,
        commands: LOCATION_TIMEZONE_COMMANDS,
        permission_scopes: TIMEZONE_PERMISSION_SCOPES,
        provider_classes: TIMEZONE_PROVIDER_CLASSES,
        health_probe: "timezone.inspect_database",
        unavailable_reason: "location_timezone_provider_not_installed",
        replay_schema: "location.timezone.replay.v1",
        data_classification: "location_timezone_reference_metadata",
        retention_policy: "timezone_zone_offset_transition_mapping_database_and_boundary_metadata_by_reference",
        redaction_policy: "exact_coordinates_raw_boundary_geometry_database_paths_host_identifiers_provider_payloads_and_credentials_redacted",
        timeout_ms: 90_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.location.timezone.v1` as optional until a timezone provider is installed.",
            "Use zone references, offset records, transition pages, display-name references, and database metadata instead of raw boundary geometry or host database paths.",
        ],
        migration_notes: &[
            "Timezone commands become callable only after an approved timezone service provider registers matching schemas.",
            "Foundation time, workflow schedules, calendars, maps, geocode, and device location capture remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneCommandContext {
    pub tenant_scope: String,
    pub identifier_system: String,
    pub freshness_policy_ref: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneProviderCapability {
    pub provider_class: String,
    pub identifier_systems: BTreeSet<String>,
    pub supported_resolvers: BTreeSet<String>,
    pub dataset_versions: BTreeMap<String, String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneCoordinateQuery {
    pub query_ref: String,
    pub coordinate_ref: String,
    pub precision_class: String,
    pub boundary_dataset_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneZone {
    pub zone_ref: String,
    pub iana_id: String,
    pub windows_id: Option<String>,
    pub canonical: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneLookupResult {
    pub lookup_ref: String,
    pub zone: TimezoneZone,
    pub boundary_provenance: TimezoneBoundaryProvenance,
    pub confidence_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneOffset {
    pub zone_ref: String,
    pub instant_epoch_ms: i64,
    pub offset_seconds: i32,
    pub dst_seconds: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneTransition {
    pub transition_ref: String,
    pub zone_ref: String,
    pub instant_epoch_ms: i64,
    pub offset_before_seconds: i32,
    pub offset_after_seconds: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneLocalResolution {
    pub resolution_ref: String,
    pub zone_ref: String,
    pub resolver_strategy: String,
    pub status: String,
    pub instant_refs: Vec<String>,
}

impl TimezoneLocalResolution {
    /// Require an explicit gap/fold strategy to avoid hidden scheduling semantics.
    pub fn has_explicit_strategy(&self) -> bool {
        matches!(
            self.resolver_strategy.as_str(),
            "reject" | "earlier" | "later" | "compatible" | "explicit_offset"
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneDisplayNames {
    pub zone_ref: String,
    pub locale: String,
    pub names: BTreeMap<String, String>,
    pub dataset_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneDatabaseInfo {
    pub database_ref: String,
    pub dataset_name: String,
    pub version: String,
    pub stale: bool,
    pub generated_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneBoundaryProvenance {
    pub boundary_ref: String,
    pub dataset_ref: String,
    pub ambiguity_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneIdentifierMapping {
    pub mapping_ref: String,
    pub source_system: String,
    pub target_system: String,
    pub entries: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_location_command_wrappers!(
    TimezoneLookupByCoordinatesCommand,
    TimezoneResolveZoneCommand,
    TimezoneGetOffsetCommand,
    TimezoneListTransitionsCommand,
    TimezoneConvertInstantCommand,
    TimezoneResolveLocalTimeCommand,
    TimezoneGetDisplayNamesCommand,
    TimezoneInspectDatabaseCommand,
    TimezoneInspectMappingCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimezoneResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    InvalidZone,
    InvalidCoordinate,
    AmbiguousBoundary,
    StaleDatabase,
    NonexistentLocalTime,
    AmbiguousLocalTime,
    QuotaExceeded,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneResultEnvelope<T> {
    pub status: TimezoneResultStatus,
    pub data: Option<T>,
    pub page: Option<LocationPackPage<T>>,
    pub error: Option<LocationPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimezoneDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub tzdb_version_hash: String,
    pub transition_fixture_hash: String,
    pub mapping_hash: String,
    pub boundary_provenance_hash: String,
    pub redaction_profile_hash: String,
}

pub fn location_timezone_descriptor_hashes() -> TimezoneDescriptorHashes {
    TimezoneDescriptorHashes {
        command_schema_hash: timezone_stable_hash(&LOCATION_TIMEZONE_COMMANDS),
        result_schema_hash: timezone_stable_hash(&TimezoneResultStatus::Success),
        descriptor_hash: timezone_stable_hash(&location_timezone_pack_definition()),
        provider_capability_hash: timezone_stable_hash(&TimezoneProviderCapability {
            provider_class: "mock".into(),
            identifier_systems: BTreeSet::from(["iana".into(), "windows".into()]),
            supported_resolvers: BTreeSet::from(["reject".into(), "compatible".into()]),
            dataset_versions: BTreeMap::from([("tzdb".into(), "synthetic".into())]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        tzdb_version_hash: timezone_stable_hash(&TimezoneDatabaseInfo {
            database_ref: "db".into(),
            dataset_name: "tzdb".into(),
            version: "synthetic".into(),
            stale: false,
            generated_at_epoch_ms: 10,
        }),
        transition_fixture_hash: timezone_stable_hash(&TimezoneTransition {
            transition_ref: "transition".into(),
            zone_ref: "zone".into(),
            instant_epoch_ms: 1,
            offset_before_seconds: 0,
            offset_after_seconds: 3600,
        }),
        mapping_hash: timezone_stable_hash(&TimezoneIdentifierMapping {
            mapping_ref: "mapping".into(),
            source_system: "windows".into(),
            target_system: "iana".into(),
            entries: BTreeMap::from([("Synthetic Standard Time".into(), "Etc/UTC".into())]),
        }),
        boundary_provenance_hash: timezone_stable_hash(&TimezoneBoundaryProvenance {
            boundary_ref: "boundary".into(),
            dataset_ref: "dataset".into(),
            ambiguity_class: "none".into(),
        }),
        redaction_profile_hash: timezone_stable_hash("timezone-redaction-v1"),
    }
}

pub fn timezone_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    location_stable_hash(value)
}
