use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::location_common::{
    define_location_command_wrappers, location_pack_definition, location_stable_hash,
    LocationPackCommandEnvelope, LocationPackDescriptor, LocationPackError, LocationPackPage,
    LocationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const LOCATION_MAPS_PACK_ID: &str = "pack.location.maps.v1";
pub const LOCATION_MAPS_SERVICE_ID: &str = "service.location.maps";

pub const LOCATION_MAPS_COMMANDS: &[&str] = &[
    "maps.inspect_provider",
    "maps.discover_styles",
    "maps.discover_tile_matrix",
    "maps.plan_tile_request",
    "maps.get_tile",
    "maps.validate_viewport",
    "maps.plan_annotation",
    "maps.plan_overlay",
    "maps.plan_static_render",
    "maps.render_static_map",
    "maps.inspect_attribution",
    "maps.inspect_cache",
    "maps.get_artifact",
];

const MAPS_PERMISSION_SCOPES: &[&str] = &[
    "location.maps.read",
    "location.maps.tile.read",
    "location.maps.style.read",
    "location.maps.viewport.validate",
    "location.maps.annotation.plan",
    "location.maps.overlay.plan",
    "location.maps.render",
    "location.maps.attribution.read",
    "location.maps.cache.read",
    "location.maps.artifact.read",
];

const MAPS_TILE_METADATA: &[(&str, &str)] = &[
    ("tile_matrix", "declared"),
    ("raw_tiles_in_trace", "false"),
    ("attribution_required", "true"),
];
const MAPS_RENDER_METADATA: &[(&str, &str)] = &[
    ("static_render", "planned"),
    ("artifact_retention", "policy_bound"),
    ("ui_widgets", "false"),
];
const MAPS_OVERLAY_METADATA: &[(&str, &str)] = &[
    ("annotations", "planned"),
    ("overlays", "reference_only"),
    ("raw_geometry", "false"),
];
const MAPS_MOCK_METADATA: &[(&str, &str)] = &[
    ("fixtures", "synthetic"),
    ("callable", "false"),
    ("network", "false"),
];
const MAPS_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("tiles", "false"),
    ("render", "false"),
    ("reason", "provider_not_installed"),
];

const MAPS_PROVIDER_CLASSES: &[LocationProviderClass<'_>] = &[
    LocationProviderClass {
        provider_class: "tile-matrix",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MAPS_TILE_METADATA,
    },
    LocationProviderClass {
        provider_class: "static-render",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MAPS_RENDER_METADATA,
    },
    LocationProviderClass {
        provider_class: "overlay-planner",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MAPS_OVERLAY_METADATA,
    },
    LocationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MAPS_MOCK_METADATA,
    },
    LocationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: MAPS_UNAVAILABLE_METADATA,
    },
];

/// Build the maps descriptor without binding tile, render, cache, or UI provider code.
pub fn location_maps_pack_definition() -> DomainPackDefinition {
    location_pack_definition(LocationPackDescriptor {
        pack_id: LOCATION_MAPS_PACK_ID,
        child_change_id: "openspec:add-pack-location-maps",
        docs_slug: "maps",
        sdk_slug: "maps",
        service_id: LOCATION_MAPS_SERVICE_ID,
        commands: LOCATION_MAPS_COMMANDS,
        permission_scopes: MAPS_PERMISSION_SCOPES,
        provider_classes: MAPS_PROVIDER_CLASSES,
        health_probe: "maps.inspect_provider",
        unavailable_reason: "location_maps_provider_not_installed",
        replay_schema: "location.maps.replay.v1",
        data_classification: "location_map_reference_metadata",
        retention_policy: "tile_style_viewport_attribution_cache_and_artifact_metadata_by_reference",
        redaction_policy: "raw_tiles_private_coordinates_private_overlays_raw_geometry_provider_payloads_and_credentials_redacted",
        timeout_ms: 120_000,
        budget_units: 5,
        examples: &[
            "Declare `pack.location.maps.v1` as optional until a maps provider is installed.",
            "Use tile references, viewport plans, attribution bundles, and artifact handles instead of raw map tiles or UI widgets.",
        ],
        migration_notes: &[
            "Maps commands become callable only after an approved maps service provider registers matching schemas.",
            "Geocoding, routing, place search, timezone lookup, device capture, media rendering, and application UI remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapScope {
    pub tenant_scope: String,
    pub application_scope: String,
    pub coordinate_precision_class: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapProviderCapability {
    pub provider_class: String,
    pub supported_formats: BTreeSet<String>,
    pub supported_projections: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapStyleReference {
    pub style_ref: String,
    pub style_family: String,
    pub version_hash: String,
    pub attribution_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileMatrixDescriptor {
    pub matrix_ref: String,
    pub projection: String,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub tile_size_px: u32,
    pub supported_formats: BTreeSet<String>,
}

impl TileMatrixDescriptor {
    /// Validate zoom bounds before descriptor evidence reaches discovery or tests.
    pub fn has_valid_zoom_bounds(&self) -> bool {
        self.min_zoom <= self.max_zoom && self.max_zoom <= 32 && self.tile_size_px > 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapTileCoordinate {
    pub matrix_ref: String,
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapTileReference {
    pub tile_ref: String,
    pub coordinate: MapTileCoordinate,
    pub format: String,
    pub cache_key_hash: String,
    pub attribution_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapViewport {
    pub viewport_ref: String,
    pub center_precision_class: String,
    pub zoom: u8,
    pub width_px: u32,
    pub height_px: u32,
    pub spatial_reference: String,
}

impl MapViewport {
    /// Enforce bounded render dimensions before a service provider receives work.
    pub fn is_bounded(&self, max_pixels: u64) -> bool {
        u64::from(self.width_px) * u64::from(self.height_px) <= max_pixels
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapAnnotation {
    pub annotation_ref: String,
    pub anchor_ref: String,
    pub label_class: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapOverlay {
    pub overlay_ref: String,
    pub geometry_ref: String,
    pub feature_count: u32,
    pub sensitivity_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticMapRenderRequest {
    pub request_ref: String,
    pub viewport: MapViewport,
    pub style_ref: String,
    pub annotation_refs: Vec<String>,
    pub overlay_refs: Vec<String>,
    pub idempotency_key_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapAttributionBundle {
    pub attribution_ref: String,
    pub display_text_ref: String,
    pub license_refs: BTreeSet<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapCacheStatus {
    pub cache_key_hash: String,
    pub freshness_class: String,
    pub expires_at_epoch_ms: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub attribution_ref: String,
    pub access_policy_ref: String,
}

define_location_command_wrappers!(
    MapsInspectProviderCommand,
    MapsDiscoverStylesCommand,
    MapsDiscoverTileMatrixCommand,
    MapsPlanTileRequestCommand,
    MapsGetTileCommand,
    MapsValidateViewportCommand,
    MapsPlanAnnotationCommand,
    MapsPlanOverlayCommand,
    MapsPlanStaticRenderCommand,
    MapsRenderStaticMapCommand,
    MapsInspectAttributionCommand,
    MapsInspectCacheCommand,
    MapsGetArtifactCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapsResultStatus {
    Success,
    Paged,
    Partial,
    Accepted,
    ApprovalRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    QuotaExceeded,
    RateLimited,
    Timeout,
    Cancelled,
    AttributionMissing,
    CacheStale,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapsResultEnvelope<T> {
    pub status: MapsResultStatus,
    pub data: Option<T>,
    pub page: Option<LocationPackPage<T>>,
    pub error: Option<LocationPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapsDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub style_catalog_hash: String,
    pub tile_matrix_hash: String,
    pub attribution_bundle_hash: String,
    pub cache_policy_hash: String,
    pub redaction_profile_hash: String,
}

pub fn location_maps_descriptor_hashes() -> MapsDescriptorHashes {
    MapsDescriptorHashes {
        command_schema_hash: maps_stable_hash(&LOCATION_MAPS_COMMANDS),
        result_schema_hash: maps_stable_hash(&MapsResultStatus::Success),
        descriptor_hash: maps_stable_hash(&location_maps_pack_definition()),
        provider_capability_hash: maps_stable_hash(&MapProviderCapability {
            provider_class: "mock".into(),
            supported_formats: BTreeSet::from(["png".into(), "webp".into()]),
            supported_projections: BTreeSet::from(["web_mercator".into()]),
            limits: BTreeMap::from([("max_static_pixels".into(), 4_000_000)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        style_catalog_hash: maps_stable_hash(&MapStyleReference {
            style_ref: "style".into(),
            style_family: "standard".into(),
            version_hash: "version".into(),
            attribution_ref: "attr".into(),
        }),
        tile_matrix_hash: maps_stable_hash(&TileMatrixDescriptor {
            matrix_ref: "matrix".into(),
            projection: "web_mercator".into(),
            min_zoom: 0,
            max_zoom: 22,
            tile_size_px: 256,
            supported_formats: BTreeSet::from(["png".into()]),
        }),
        attribution_bundle_hash: maps_stable_hash(&MapAttributionBundle {
            attribution_ref: "attr".into(),
            display_text_ref: "display-ref".into(),
            license_refs: BTreeSet::from(["license".into()]),
            required: true,
        }),
        cache_policy_hash: maps_stable_hash(&MapCacheStatus {
            cache_key_hash: "cache".into(),
            freshness_class: "fresh".into(),
            expires_at_epoch_ms: 10,
            stale: false,
        }),
        redaction_profile_hash: maps_stable_hash("maps-redaction-v1"),
    }
}

pub fn maps_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    location_stable_hash(value)
}
