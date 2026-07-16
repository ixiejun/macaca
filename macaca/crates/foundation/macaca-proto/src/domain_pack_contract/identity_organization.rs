use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::identity_common::{
    define_identity_command_wrappers, identity_pack_definition, identity_stable_hash,
    IdentityPackCommandEnvelope, IdentityPackDescriptor, IdentityPackError, IdentityPackPage,
    IdentityProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const IDENTITY_ORGANIZATION_PACK_ID: &str = "pack.identity.organization.v1";
pub const IDENTITY_ORGANIZATION_SERVICE_ID: &str = "service.identity.organization";

pub const IDENTITY_ORGANIZATION_COMMANDS: &[&str] = &[
    "organization.inspect_provider",
    "organization.discover_schema",
    "organization.plan_create",
    "organization.create",
    "organization.get",
    "organization.search",
    "organization.plan_update",
    "organization.update",
    "organization.archive",
    "organization.restore",
    "organization.list_members",
    "organization.get_membership",
    "organization.plan_membership_change",
    "organization.request_membership_change",
    "organization.create_invitation",
    "organization.resend_invitation",
    "organization.revoke_invitation",
    "organization.inspect_invitation",
    "organization.plan_role_binding",
    "organization.request_role_binding",
    "organization.list_role_bindings",
    "organization.inspect_directory_links",
    "organization.export_audit",
    "organization.get_artifact",
];

const ORGANIZATION_PERMISSION_SCOPES: &[&str] = &[
    "identity.organization.read",
    "identity.organization.search",
    "identity.organization.write",
    "identity.organization.archive",
    "identity.organization.membership.read",
    "identity.organization.membership.write",
    "identity.organization.invitation.read",
    "identity.organization.invitation.write",
    "identity.organization.role.read",
    "identity.organization.role.write",
    "identity.organization.directory.read",
    "identity.organization.audit.export",
    "identity.organization.artifact.read",
];

const ORG_RECORD_METADATA: &[(&str, &str)] = &[
    ("records", "true"),
    ("domains", "reference_only"),
    ("versioning", "true"),
];
const ORG_MEMBER_METADATA: &[(&str, &str)] = &[
    ("memberships", "true"),
    ("directory_managed_conflict", "true"),
    ("account_profile_mutation", "false"),
];
const ORG_INVITE_ROLE_METADATA: &[(&str, &str)] = &[
    ("invitations", "approval_required"),
    ("role_bindings", "approval_required"),
    ("raw_invite_tokens", "false"),
];
const ORG_MOCK_METADATA: &[(&str, &str)] = &[
    ("organizations", "synthetic"),
    ("memberships", "synthetic"),
    ("callable", "false"),
];
const ORG_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("organizations", "false"),
    ("memberships", "false"),
    ("reason", "provider_not_installed"),
];

const ORGANIZATION_PROVIDER_CLASSES: &[IdentityProviderClass<'_>] = &[
    IdentityProviderClass {
        provider_class: "organization-record",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORG_RECORD_METADATA,
    },
    IdentityProviderClass {
        provider_class: "organization-membership",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORG_MEMBER_METADATA,
    },
    IdentityProviderClass {
        provider_class: "organization-invitation-role",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORG_INVITE_ROLE_METADATA,
    },
    IdentityProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORG_MOCK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: ORG_UNAVAILABLE_METADATA,
    },
];

pub fn identity_organization_pack_definition() -> DomainPackDefinition {
    identity_pack_definition(IdentityPackDescriptor {
        pack_id: IDENTITY_ORGANIZATION_PACK_ID,
        child_change_id: "openspec:add-pack-identity-organization",
        docs_slug: "organization",
        sdk_slug: "organization",
        service_id: IDENTITY_ORGANIZATION_SERVICE_ID,
        commands: IDENTITY_ORGANIZATION_COMMANDS,
        permission_scopes: ORGANIZATION_PERMISSION_SCOPES,
        provider_classes: ORGANIZATION_PROVIDER_CLASSES,
        health_probe: "organization.inspect_provider",
        unavailable_reason: "identity_organization_provider_not_installed",
        replay_schema: "identity.organization.replay.v1",
        data_classification: "identity_organization_reference_metadata",
        retention_policy: "organization_membership_invitation_role_directory_and_audit_metadata_by_reference",
        redaction_policy: "raw_credentials_invite_tokens_tokens_directory_sync_secrets_provider_payloads_member_dumps_private_profile_fields_and_unbounded_audit_exports_redacted",
        timeout_ms: 120_000,
        budget_units: 5,
        examples: &[
            "Declare `pack.identity.organization.v1` as optional until an organization provider is installed.",
            "Use organization handles, membership references, role bindings, invitation references, and audit artifact handles instead of provider-native directory payloads.",
        ],
        migration_notes: &[
            "Organization commands become callable only after an approved organization service provider registers matching schemas.",
            "Account lifecycle, rich profile fields, auth handoff, tenant policy, billing, communication delivery, and application RBAC remain outside this pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationScope {
    pub tenant_scope: String,
    pub organization_ref: Option<String>,
    pub provider_scope: String,
    pub directory_ref: Option<String>,
    pub caller_subject_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationProviderCapability {
    pub provider_class: String,
    pub feature_flags: BTreeSet<String>,
    pub supported_states: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationRecord {
    pub organization_ref: String,
    pub display_name_ref: String,
    pub identifiers: Vec<OrganizationIdentifier>,
    pub verified_domain_refs: Vec<String>,
    pub lifecycle_state: OrganizationLifecycleState,
    pub metadata_namespace_refs: BTreeSet<String>,
    pub policy_refs: Vec<OrganizationPolicyReference>,
    pub version_token_hash: String,
    pub freshness: OrganizationFreshness,
    pub audit_refs: Vec<OrganizationAuditReference>,
}

impl OrganizationRecord {
    /// Bound organization evidence before exposing it to SDK or trace consumers.
    pub fn is_bounded(&self, max_identifiers: usize, max_audit_refs: usize) -> bool {
        self.identifiers.len() <= max_identifiers && self.audit_refs.len() <= max_audit_refs
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationIdentifier {
    pub identifier_ref: String,
    pub identifier_kind: String,
    pub uniqueness_scope: String,
    pub verification_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationLifecycleState {
    pub state: String,
    pub provider_state_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationMembership {
    pub membership_ref: String,
    pub organization_ref: String,
    pub account_ref: String,
    pub profile_ref: Option<String>,
    pub membership_state: OrganizationMembershipState,
    pub role_bindings: Vec<OrganizationRoleBinding>,
    pub source_ref: String,
    pub invitation_ref: Option<String>,
    pub directory_group_ref: Option<String>,
    pub version_token_hash: String,
    pub freshness: OrganizationFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationMembershipState {
    pub state: String,
    pub directory_managed: bool,
    pub conflict_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationInvitation {
    pub invitation_ref: String,
    pub organization_ref: String,
    pub redacted_recipient_ref: String,
    pub requested_role_refs: BTreeSet<String>,
    pub expires_at_epoch_ms: u64,
    pub delivery_ref: Option<String>,
    pub state: String,
    pub audit_refs: Vec<OrganizationAuditReference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationRoleReference {
    pub role_ref: String,
    pub display_label_ref: String,
    pub privilege_class: String,
    pub source_ref: String,
    pub inherited: bool,
    pub directory_managed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationRoleBinding {
    pub binding_ref: String,
    pub role_ref: OrganizationRoleReference,
    pub subject_ref: String,
    pub organization_ref: String,
    pub effective_state: String,
    pub version_token_hash: String,
    pub freshness: OrganizationFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryGroupReference {
    pub directory_provider_class: String,
    pub group_ref: String,
    pub nested_group_hint: bool,
    pub dynamic_group_hint: bool,
    pub schema_version: String,
    pub sync_freshness: OrganizationFreshness,
    pub conflict_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationPolicyReference {
    pub policy_ref: String,
    pub policy_kind: String,
    pub decision_freshness_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationAuditReference {
    pub event_ref: String,
    pub event_type: String,
    pub bounded_reason_code: String,
    pub provider_cursor_ref: Option<String>,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_identity_command_wrappers!(
    OrganizationInspectProviderCommand,
    OrganizationDiscoverSchemaCommand,
    OrganizationPlanCreateCommand,
    OrganizationCreateCommand,
    OrganizationGetCommand,
    OrganizationSearchCommand,
    OrganizationPlanUpdateCommand,
    OrganizationUpdateCommand,
    OrganizationArchiveCommand,
    OrganizationRestoreCommand,
    OrganizationListMembersCommand,
    OrganizationGetMembershipCommand,
    OrganizationPlanMembershipChangeCommand,
    OrganizationRequestMembershipChangeCommand,
    OrganizationCreateInvitationCommand,
    OrganizationResendInvitationCommand,
    OrganizationRevokeInvitationCommand,
    OrganizationInspectInvitationCommand,
    OrganizationPlanRoleBindingCommand,
    OrganizationRequestRoleBindingCommand,
    OrganizationListRoleBindingsCommand,
    OrganizationInspectDirectoryLinksCommand,
    OrganizationExportAuditCommand,
    OrganizationGetArtifactCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationResultStatus {
    Success,
    Paged,
    Partial,
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
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationResultEnvelope<T> {
    pub status: OrganizationResultStatus,
    pub data: Option<T>,
    pub page: Option<IdentityPackPage<T>>,
    pub error: Option<IdentityPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub schema_version_compatibility_hash: String,
    pub command_availability_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub organization_hash: String,
    pub membership_hash: String,
    pub role_schema_hash: String,
    pub invitation_hash: String,
    pub artifact_hash: String,
    pub policy_template_hash: String,
    pub redaction_profile_hash: String,
}

pub fn identity_organization_descriptor_hashes() -> OrganizationDescriptorHashes {
    let freshness = OrganizationFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    let audit = OrganizationAuditReference {
        event_ref: "event".into(),
        event_type: "created".into(),
        bounded_reason_code: "ok".into(),
        replay_pointer: "replay".into(),
        ..Default::default()
    };
    let role = OrganizationRoleReference {
        role_ref: "role".into(),
        display_label_ref: "admin".into(),
        privilege_class: "elevated".into(),
        source_ref: "provider".into(),
        inherited: false,
        directory_managed: false,
    };
    OrganizationDescriptorHashes {
        command_schema_hash: organization_stable_hash(&IDENTITY_ORGANIZATION_COMMANDS),
        result_schema_hash: organization_stable_hash(&OrganizationResultStatus::Success),
        schema_version_compatibility_hash: organization_stable_hash(&(
            &identity_organization_pack_definition().metadata.version,
            &identity_organization_pack_definition()
                .metadata
                .compatibility,
        )),
        command_availability_hash: organization_stable_hash(&(
            IDENTITY_ORGANIZATION_SERVICE_ID,
            IDENTITY_ORGANIZATION_COMMANDS,
            "preview_unavailable_until_provider_registered",
        )),
        descriptor_hash: organization_stable_hash(&identity_organization_pack_definition()),
        provider_capability_hash: organization_stable_hash(&OrganizationProviderCapability {
            provider_class: "mock".into(),
            feature_flags: BTreeSet::from(["membership".into(), "role_binding".into()]),
            supported_states: BTreeSet::from(["active".into(), "archived".into()]),
            limits: BTreeMap::from([("max_members".into(), 1000)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        role_schema_hash: organization_stable_hash(&role),
        organization_hash: organization_stable_hash(&OrganizationRecord {
            organization_ref: "org".into(),
            display_name_ref: "display".into(),
            identifiers: vec![OrganizationIdentifier {
                identifier_ref: "slug".into(),
                identifier_kind: "slug".into(),
                uniqueness_scope: "tenant".into(),
                verification_state: "verified".into(),
            }],
            lifecycle_state: OrganizationLifecycleState {
                state: "active".into(),
                ..Default::default()
            },
            version_token_hash: "version".into(),
            freshness: freshness.clone(),
            audit_refs: vec![audit.clone()],
            ..Default::default()
        }),
        membership_hash: organization_stable_hash(&OrganizationMembership {
            membership_ref: "membership".into(),
            organization_ref: "org".into(),
            account_ref: "account".into(),
            membership_state: OrganizationMembershipState {
                state: "active".into(),
                ..Default::default()
            },
            role_bindings: vec![OrganizationRoleBinding {
                binding_ref: "binding".into(),
                role_ref: role,
                subject_ref: "subject".into(),
                organization_ref: "org".into(),
                effective_state: "active".into(),
                version_token_hash: "version".into(),
                freshness: freshness.clone(),
            }],
            version_token_hash: "version".into(),
            freshness,
            ..Default::default()
        }),
        invitation_hash: organization_stable_hash(&OrganizationInvitation {
            invitation_ref: "invite".into(),
            organization_ref: "org".into(),
            redacted_recipient_ref: "recipient".into(),
            expires_at_epoch_ms: 10,
            state: "pending".into(),
            audit_refs: vec![audit],
            ..Default::default()
        }),
        artifact_hash: organization_stable_hash(&OrganizationArtifactHandle {
            artifact_id: "artifact".into(),
            content_class: "audit_export".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            access_policy_ref: "policy".into(),
        }),
        policy_template_hash: organization_stable_hash(
            &identity_organization_pack_definition()
                .metadata
                .policy_template,
        ),
        redaction_profile_hash: organization_stable_hash(
            &identity_organization_pack_definition()
                .metadata
                .data_governance
                .redaction_policy,
        ),
    }
}

pub fn organization_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    identity_stable_hash(value)
}
