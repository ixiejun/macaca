use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::location_common::{
    define_location_command_wrappers, location_pack_definition, location_stable_hash,
    LocationPackCommandEnvelope, LocationPackDescriptor, LocationPackError, LocationPackPage,
    LocationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const LOCATION_ROUTE_PACK_ID: &str = "pack.location.route.v1";
pub const LOCATION_ROUTE_SERVICE_ID: &str = "service.location.route";

pub const LOCATION_ROUTE_COMMANDS: &[&str] = &[
    "route.inspect_provider",
    "route.discover_profiles",
    "route.validate_request",
    "route.plan",
    "route.estimate_eta",
    "route.inspect",
    "route.plan_matrix",
    "route.request_matrix",
    "route.inspect_matrix",
    "route.cancel_matrix",
    "route.plan_optimization",
    "route.request_optimization",
    "route.inspect_optimization",
    "route.cancel_optimization",
    "route.inspect_retention",
    "route.inspect_attribution",
    "route.get_artifact",
];

const ROUTE_PERMISSION_SCOPES: &[&str] = &[
    "location.route.plan",
    "location.route.eta",
    "location.route.matrix",
    "location.route.optimize",
    "location.route.inspect",
    "location.route.retention.read",
    "location.route.attribution.read",
    "location.route.artifact.read",
];

const ROUTE_PLAN_METADATA: &[(&str, &str)] = &[
    ("routing", "planned"),
    ("raw_geometry_in_trace", "false"),
    ("private_routes", "redacted"),
];
const ROUTE_MATRIX_METADATA: &[(&str, &str)] = &[
    ("matrix", "async_planned"),
    ("cell_budget", "declared"),
    ("artifact_handles", "reference_only"),
];
const ROUTE_OPTIMIZATION_METADATA: &[(&str, &str)] = &[
    ("optimization", "async_planned"),
    ("dispatch_policy", "application_owned"),
    ("approval", "policy_bound"),
];
const ROUTE_MOCK_METADATA: &[(&str, &str)] = &[
    ("fixtures", "synthetic"),
    ("callable", "false"),
    ("network", "false"),
];
const ROUTE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("routing", "false"),
    ("matrix", "false"),
    ("reason", "provider_not_installed"),
];

const ROUTE_PROVIDER_CLASSES: &[LocationProviderClass<'_>] = &[
    LocationProviderClass {
        provider_class: "route-planner",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ROUTE_PLAN_METADATA,
    },
    LocationProviderClass {
        provider_class: "matrix-engine",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ROUTE_MATRIX_METADATA,
    },
    LocationProviderClass {
        provider_class: "waypoint-optimizer",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ROUTE_OPTIMIZATION_METADATA,
    },
    LocationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ROUTE_MOCK_METADATA,
    },
    LocationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: ROUTE_UNAVAILABLE_METADATA,
    },
];

/// Build the route descriptor without binding navigation, matrix, or optimization providers.
pub fn location_route_pack_definition() -> DomainPackDefinition {
    location_pack_definition(LocationPackDescriptor {
        pack_id: LOCATION_ROUTE_PACK_ID,
        child_change_id: "openspec:add-pack-location-route",
        docs_slug: "route",
        sdk_slug: "route",
        service_id: LOCATION_ROUTE_SERVICE_ID,
        commands: LOCATION_ROUTE_COMMANDS,
        permission_scopes: ROUTE_PERMISSION_SCOPES,
        provider_classes: ROUTE_PROVIDER_CLASSES,
        health_probe: "route.inspect_provider",
        unavailable_reason: "location_route_provider_not_installed",
        replay_schema: "location.route.replay.v1",
        data_classification: "location_route_reference_metadata",
        retention_policy: "waypoints_profiles_metrics_retention_attribution_matrix_optimization_and_artifact_metadata_by_reference",
        redaction_policy: "private_routes_exact_coordinates_raw_geometry_provider_payloads_route_batches_artifacts_and_credentials_redacted",
        timeout_ms: 180_000,
        budget_units: 8,
        examples: &[
            "Declare `pack.location.route.v1` as optional until a route provider is installed.",
            "Use waypoint references, profile descriptors, route metrics, matrix jobs, optimization jobs, and artifact handles instead of raw navigation provider payloads.",
        ],
        migration_notes: &[
            "Route commands become callable only after an approved route service provider registers matching schemas.",
            "Geocoding, maps, place search, timezone lookup, device capture, toll settlement, and application dispatch logic remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteScope {
    pub tenant_scope: String,
    pub region_policy_ref: String,
    pub coordinate_precision_class: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteProviderCapability {
    pub provider_class: String,
    pub supported_profiles: BTreeSet<String>,
    pub supported_constraints: BTreeSet<String>,
    pub supported_geometry_formats: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteWaypoint {
    pub waypoint_ref: String,
    pub coordinate_ref: String,
    pub role: String,
    pub precision_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteTravelProfile {
    pub profile_ref: String,
    pub mode: String,
    pub traffic_model: Option<String>,
    pub vehicle_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteConstraintSet {
    pub constraint_ref: String,
    pub avoid_classes: BTreeSet<String>,
    pub require_classes: BTreeSet<String>,
    pub policy_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePlan {
    pub plan_ref: String,
    pub waypoints: Vec<RouteWaypoint>,
    pub profile: RouteTravelProfile,
    pub constraints: RouteConstraintSet,
    pub alternatives_requested: u8,
    pub retention_policy_ref: String,
}

impl RoutePlan {
    /// Enforce waypoint and alternative bounds before route calculation is dispatched.
    pub fn is_bounded(&self, max_waypoints: usize, max_alternatives: u8) -> bool {
        self.waypoints.len() <= max_waypoints && self.alternatives_requested <= max_alternatives
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteLeg {
    pub leg_ref: String,
    pub from_waypoint_ref: String,
    pub to_waypoint_ref: String,
    pub metric_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteStep {
    pub step_ref: String,
    pub maneuver_ref: String,
    pub distance_meters: u64,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteManeuver {
    pub maneuver_ref: String,
    pub kind: String,
    pub instruction_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteGeometry {
    pub geometry_ref: String,
    pub format: String,
    pub encoded_ref: String,
    pub precision_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMetricSet {
    pub metric_ref: String,
    pub distance_meters: u64,
    pub duration_seconds: u64,
    pub confidence_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMatrixJob {
    pub job_ref: String,
    pub origin_count: u32,
    pub destination_count: u32,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaypointOptimizationJob {
    pub job_ref: String,
    pub waypoint_count: u32,
    pub objective: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_location_command_wrappers!(
    RouteInspectProviderCommand,
    RouteDiscoverProfilesCommand,
    RouteValidateRequestCommand,
    RoutePlanCommand,
    RouteEstimateEtaCommand,
    RouteInspectCommand,
    RoutePlanMatrixCommand,
    RouteRequestMatrixCommand,
    RouteInspectMatrixCommand,
    RouteCancelMatrixCommand,
    RoutePlanOptimizationCommand,
    RouteRequestOptimizationCommand,
    RouteInspectOptimizationCommand,
    RouteCancelOptimizationCommand,
    RouteInspectRetentionCommand,
    RouteInspectAttributionCommand,
    RouteGetArtifactCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteResultStatus {
    Success,
    Paged,
    Partial,
    Accepted,
    ApprovalRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    NoRoute,
    Ambiguous,
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
pub struct RouteResultEnvelope<T> {
    pub status: RouteResultStatus,
    pub data: Option<T>,
    pub page: Option<LocationPackPage<T>>,
    pub error: Option<LocationPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub profile_constraint_hash: String,
    pub geometry_format_hash: String,
    pub retention_policy_hash: String,
    pub attribution_bundle_hash: String,
    pub redaction_profile_hash: String,
}

pub fn location_route_descriptor_hashes() -> RouteDescriptorHashes {
    RouteDescriptorHashes {
        command_schema_hash: route_stable_hash(&LOCATION_ROUTE_COMMANDS),
        result_schema_hash: route_stable_hash(&RouteResultStatus::Success),
        descriptor_hash: route_stable_hash(&location_route_pack_definition()),
        provider_capability_hash: route_stable_hash(&RouteProviderCapability {
            provider_class: "mock".into(),
            supported_profiles: BTreeSet::from(["walking".into(), "driving".into()]),
            supported_constraints: BTreeSet::from(["avoid_tolls".into()]),
            supported_geometry_formats: BTreeSet::from(["encoded_reference".into()]),
            limits: BTreeMap::from([("max_waypoints".into(), 25)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        profile_constraint_hash: route_stable_hash(&RouteConstraintSet {
            constraint_ref: "constraints".into(),
            avoid_classes: BTreeSet::from(["tolls".into()]),
            require_classes: BTreeSet::new(),
            policy_ref: "policy".into(),
        }),
        geometry_format_hash: route_stable_hash(&RouteGeometry {
            geometry_ref: "geometry".into(),
            format: "encoded_reference".into(),
            encoded_ref: "geometry-ref".into(),
            precision_class: "route_generalized".into(),
        }),
        retention_policy_hash: route_stable_hash("route-retention-ephemeral"),
        attribution_bundle_hash: route_stable_hash("route-attribution-reference"),
        redaction_profile_hash: route_stable_hash("route-redaction-v1"),
    }
}

pub fn route_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    location_stable_hash(value)
}
