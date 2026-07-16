use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::identity_common::{
    define_identity_command_wrappers, identity_pack_definition, identity_stable_hash,
    IdentityPackCommandEnvelope, IdentityPackDescriptor, IdentityPackError, IdentityPackPage,
    IdentityProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const IDENTITY_PROFILE_PACK_ID: &str = "pack.identity.profile.v1";
pub const IDENTITY_PROFILE_SERVICE_ID: &str = "service.identity.profile";

pub const IDENTITY_PROFILE_COMMANDS: &[&str] = &[
    "profile.inspect_provider",
    "profile.describe_schema",
    "profile.read_profile",
    "profile.search_profiles",
    "profile.plan_patch",
    "profile.update_profile",
    "profile.inspect_privacy_fields",
    "profile.list_preferences",
    "profile.set_preference",
    "profile.plan_avatar_update",
    "profile.set_avatar_reference",
    "profile.clear_avatar_reference",
    "profile.sync_profile",
    "profile.plan_export",
    "profile.export_profile",
    "profile.get_artifact_handle",
];

const PROFILE_PERMISSION_SCOPES: &[&str] = &[
    "identity.profile.read",
    "identity.profile.write",
    "identity.profile.preferences",
    "identity.profile.avatar",
    "identity.profile.privacy",
    "identity.profile.export",
];

const PROFILE_SCHEMA_METADATA: &[(&str, &str)] = &[
    ("schemas", "true"),
    ("field_masks", "true"),
    ("metadata_namespaces", "scoped"),
];
const PROFILE_PRIVACY_METADATA: &[(&str, &str)] = &[
    ("privacy_classes", "true"),
    ("minimization", "required"),
    ("exports", "handle_only"),
];
const PROFILE_AVATAR_METADATA: &[(&str, &str)] = &[
    ("avatar_reference", "true"),
    ("raw_bytes", "false"),
    ("media_handoff", "reference_only"),
];
const PROFILE_MOCK_METADATA: &[(&str, &str)] = &[
    ("profiles", "synthetic"),
    ("schemas", "synthetic"),
    ("callable", "false"),
];
const PROFILE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("profiles", "false"),
    ("write", "false"),
    ("reason", "provider_not_installed"),
];

const PROFILE_PROVIDER_CLASSES: &[IdentityProviderClass<'_>] = &[
    IdentityProviderClass {
        provider_class: "profile-schema",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PROFILE_SCHEMA_METADATA,
    },
    IdentityProviderClass {
        provider_class: "profile-privacy",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PROFILE_PRIVACY_METADATA,
    },
    IdentityProviderClass {
        provider_class: "profile-avatar",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PROFILE_AVATAR_METADATA,
    },
    IdentityProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PROFILE_MOCK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: PROFILE_UNAVAILABLE_METADATA,
    },
];

pub fn identity_profile_pack_definition() -> DomainPackDefinition {
    identity_pack_definition(IdentityPackDescriptor {
        pack_id: IDENTITY_PROFILE_PACK_ID,
        child_change_id: "openspec:add-pack-identity-profile",
        docs_slug: "profile",
        sdk_slug: "profile",
        service_id: IDENTITY_PROFILE_SERVICE_ID,
        commands: IDENTITY_PROFILE_COMMANDS,
        permission_scopes: PROFILE_PERMISSION_SCOPES,
        provider_classes: PROFILE_PROVIDER_CLASSES,
        health_probe: "profile.inspect_provider",
        unavailable_reason: "identity_profile_provider_not_installed",
        replay_schema: "identity.profile.replay.v1",
        data_classification: "identity_profile_reference_metadata",
        retention_policy: "profile_field_schema_privacy_avatar_preference_and_export_metadata_by_reference",
        redaction_policy: "raw_credentials_tokens_identity_documents_provider_payloads_unbounded_profile_exports_and_raw_avatar_bytes_redacted",
        timeout_ms: 90_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.identity.profile.v1` as optional until a profile provider is installed.",
            "Use field masks, privacy classes, avatar references, and export handles instead of raw profile payloads.",
        ],
        migration_notes: &[
            "Profile commands become callable only after an approved profile service provider registers matching schemas.",
            "Account lifecycle, auth handoff, sessions, organizations, tenants, secrets, media processing, and application preferences remain outside this pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileScope {
    pub tenant_scope: String,
    pub provider_scope: String,
    pub account_ref: Option<String>,
    pub subject_ref: Option<String>,
    pub profile_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileProviderCapability {
    pub provider_class: String,
    pub feature_flags: BTreeSet<String>,
    pub supported_value_types: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRecord {
    pub profile_ref: String,
    pub account_ref: String,
    pub subject_ref: String,
    pub fields: Vec<ProfileField>,
    pub metadata_namespaces: Vec<ProfileMetadataNamespace>,
    pub preferences: Vec<ProfilePreference>,
    pub avatar_ref: Option<AvatarReference>,
    pub privacy_map: BTreeMap<String, String>,
    pub version_token_hash: String,
    pub freshness: ProfileFreshness,
    pub attribution: ProfileAttribution,
    pub redaction_class: String,
}

impl ProfileRecord {
    /// Keep profile projections bounded so field-heavy directories cannot leak unbounded data.
    pub fn is_bounded(&self, max_fields: usize, max_preferences: usize) -> bool {
        self.fields.len() <= max_fields && self.preferences.len() <= max_preferences
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileField {
    pub field_key: String,
    pub value_type: String,
    pub value_ref: String,
    pub source_ref: String,
    pub verification_state: String,
    pub visibility: String,
    pub privacy_class: String,
    pub retention_class: String,
    pub mutable: bool,
    pub locale: Option<String>,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSchemaDescriptor {
    pub schema_ref: String,
    pub field_definitions: BTreeMap<String, String>,
    pub extension_refs: BTreeSet<String>,
    pub validation_rule_refs: BTreeSet<String>,
    pub default_field_mask: BTreeSet<String>,
    pub compatibility_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileMetadataNamespace {
    pub namespace_ref: String,
    pub namespace_kind: String,
    pub access_policy_ref: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilePreference {
    pub preference_ref: String,
    pub key: String,
    pub value_ref: String,
    pub scope_ref: String,
    pub source_ref: String,
    pub retention_class: String,
    pub privacy_class: String,
    pub application_business_boundary: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarReference {
    pub avatar_ref: String,
    pub hosted_url_ref: Option<String>,
    pub media_artifact_handle: Option<String>,
    pub checksum: String,
    pub dimensions: String,
    pub content_type: String,
    pub expires_at_epoch_ms: Option<u64>,
    pub retention_class: String,
    pub source_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileAuditReference {
    pub event_ref: String,
    pub actor_ref: String,
    pub field_mask: BTreeSet<String>,
    pub checksum: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileExportPlan {
    pub export_ref: String,
    pub scope_ref: String,
    pub field_mask: BTreeSet<String>,
    pub format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_identity_command_wrappers!(
    ProfileInspectProviderCommand,
    ProfileDescribeSchemaCommand,
    ProfileReadProfileCommand,
    ProfileSearchProfilesCommand,
    ProfilePlanPatchCommand,
    ProfileUpdateProfileCommand,
    ProfileInspectPrivacyFieldsCommand,
    ProfileListPreferencesCommand,
    ProfileSetPreferenceCommand,
    ProfilePlanAvatarUpdateCommand,
    ProfileSetAvatarReferenceCommand,
    ProfileClearAvatarReferenceCommand,
    ProfileSyncProfileCommand,
    ProfilePlanExportCommand,
    ProfileExportProfileCommand,
    ProfileGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileResultStatus {
    Success,
    Paged,
    Partial,
    Accepted,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    ApprovalRequired,
    VersionConflict,
    ArtifactRedacted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileResultEnvelope<T> {
    pub status: ProfileResultStatus,
    pub data: Option<T>,
    pub page: Option<IdentityPackPage<T>>,
    pub error: Option<IdentityPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub profile_hash: String,
    pub schema_hash: String,
    pub avatar_hash: String,
    pub artifact_hash: String,
}

pub fn identity_profile_descriptor_hashes() -> ProfileDescriptorHashes {
    let freshness = ProfileFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    let avatar = AvatarReference {
        avatar_ref: "avatar".into(),
        media_artifact_handle: Some("media-artifact".into()),
        checksum: "checksum".into(),
        dimensions: "128x128".into(),
        content_type: "image/png".into(),
        retention_class: "short".into(),
        source_ref: "source".into(),
        redaction_profile: "avatar_reference_only".into(),
        ..Default::default()
    };
    ProfileDescriptorHashes {
        command_schema_hash: profile_stable_hash(&IDENTITY_PROFILE_COMMANDS),
        result_schema_hash: profile_stable_hash(&ProfileResultStatus::Success),
        descriptor_hash: profile_stable_hash(&identity_profile_pack_definition()),
        provider_capability_hash: profile_stable_hash(&ProfileProviderCapability {
            provider_class: "mock".into(),
            feature_flags: BTreeSet::from(["schema".into(), "avatar".into()]),
            supported_value_types: BTreeSet::from(["string".into(), "reference".into()]),
            limits: BTreeMap::from([("max_fields".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        profile_hash: profile_stable_hash(&ProfileRecord {
            profile_ref: "profile".into(),
            account_ref: "account".into(),
            subject_ref: "subject".into(),
            fields: vec![ProfileField {
                field_key: "display_name".into(),
                value_type: "string".into(),
                value_ref: "value".into(),
                privacy_class: "public".into(),
                mutable: true,
                ..Default::default()
            }],
            avatar_ref: Some(avatar.clone()),
            version_token_hash: "version".into(),
            freshness: freshness.clone(),
            attribution: ProfileAttribution {
                source_ref: "source".into(),
                provider_class: "mock".into(),
            },
            redaction_class: "field_masked".into(),
            ..Default::default()
        }),
        schema_hash: profile_stable_hash(&ProfileSchemaDescriptor {
            schema_ref: "schema".into(),
            field_definitions: BTreeMap::from([("display_name".into(), "string".into())]),
            compatibility_hash: "compat".into(),
            ..Default::default()
        }),
        avatar_hash: profile_stable_hash(&avatar),
        artifact_hash: profile_stable_hash(&ProfileArtifactHandle {
            artifact_id: "artifact".into(),
            content_class: "profile_export".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            access_policy_ref: "policy".into(),
        }),
    }
}

pub fn profile_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    identity_stable_hash(value)
}
