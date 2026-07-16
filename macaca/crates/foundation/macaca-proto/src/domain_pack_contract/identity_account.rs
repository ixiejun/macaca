use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::identity_common::{
    define_identity_command_wrappers, identity_pack_definition, identity_stable_hash,
    IdentityPackCommandEnvelope, IdentityPackDescriptor, IdentityPackError, IdentityPackPage,
    IdentityProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const IDENTITY_ACCOUNT_PACK_ID: &str = "pack.identity.account.v1";
pub const IDENTITY_ACCOUNT_SERVICE_ID: &str = "service.identity.account";

pub const IDENTITY_ACCOUNT_COMMANDS: &[&str] = &[
    "account.inspect_provider",
    "account.describe_schema",
    "account.plan_create",
    "account.create_account",
    "account.read_account",
    "account.search_accounts",
    "account.plan_update",
    "account.update_account",
    "account.plan_lifecycle_transition",
    "account.lifecycle_transition_request",
    "account.link_identity",
    "account.unlink_identity",
    "account.sync_status",
    "account.set_recovery_reference",
    "account.inspect_account_audit",
    "account.plan_audit_export",
    "account.audit_export_request",
    "account.get_artifact_handle",
];

const ACCOUNT_PERMISSION_SCOPES: &[&str] = &[
    "identity.account.read",
    "identity.account.create",
    "identity.account.update",
    "identity.account.lifecycle",
    "identity.account.link_identity",
    "identity.account.audit_export",
];

const ACCOUNT_DIRECTORY_METADATA: &[(&str, &str)] = &[
    ("records", "true"),
    ("identifiers", "minimized"),
    ("custom_schema", "reference_only"),
    ("status_sync", "true"),
];
const ACCOUNT_LIFECYCLE_METADATA: &[(&str, &str)] = &[
    ("transitions", "planned"),
    ("delete_or_suspend", "approval_required"),
    ("version_tokens", "required_when_supported"),
];
const ACCOUNT_LINK_METADATA: &[(&str, &str)] = &[
    ("linked_identities", "reference_only"),
    ("raw_credentials", "false"),
    ("conflict_checks", "true"),
];
const ACCOUNT_MOCK_METADATA: &[(&str, &str)] = &[
    ("accounts", "synthetic"),
    ("lifecycle", "synthetic"),
    ("callable", "false"),
];
const ACCOUNT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("accounts", "false"),
    ("lifecycle", "false"),
    ("reason", "provider_not_installed"),
];

const ACCOUNT_PROVIDER_CLASSES: &[IdentityProviderClass<'_>] = &[
    IdentityProviderClass {
        provider_class: "account-directory",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNT_DIRECTORY_METADATA,
    },
    IdentityProviderClass {
        provider_class: "account-lifecycle",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNT_LIFECYCLE_METADATA,
    },
    IdentityProviderClass {
        provider_class: "linked-identity",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNT_LINK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNT_MOCK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: ACCOUNT_UNAVAILABLE_METADATA,
    },
];

/// Build the account descriptor without binding IdP, directory, credential, or workflow code.
pub fn identity_account_pack_definition() -> DomainPackDefinition {
    identity_pack_definition(IdentityPackDescriptor {
        pack_id: IDENTITY_ACCOUNT_PACK_ID,
        child_change_id: "openspec:add-pack-identity-account",
        docs_slug: "account",
        sdk_slug: "account",
        service_id: IDENTITY_ACCOUNT_SERVICE_ID,
        commands: IDENTITY_ACCOUNT_COMMANDS,
        permission_scopes: ACCOUNT_PERMISSION_SCOPES,
        provider_classes: ACCOUNT_PROVIDER_CLASSES,
        health_probe: "account.inspect_provider",
        unavailable_reason: "identity_account_provider_not_installed",
        replay_schema: "identity.account.replay.v1",
        data_classification: "identity_account_reference_metadata",
        retention_policy: "account_identifier_lifecycle_link_recovery_and_audit_metadata_by_reference",
        redaction_policy: "raw_credentials_password_hashes_reset_tokens_mfa_secrets_provider_payloads_identity_documents_and_unbounded_audit_exports_redacted",
        timeout_ms: 90_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.identity.account.v1` as optional until an account provider is installed.",
            "Use account handles, minimized identifiers, lifecycle plans, and audit artifact handles instead of provider-native user payloads.",
        ],
        migration_notes: &[
            "Account commands become callable only after an approved account service provider registers matching schemas.",
            "Auth handoff, sessions, credentials, profile preferences, organization membership, and tenant policy remain separate packs.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountScope {
    pub tenant_scope: String,
    pub provider_scope: String,
    pub account_ref: Option<String>,
    pub subject_ref: Option<String>,
    pub identity_provider_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountProviderCapability {
    pub provider_class: String,
    pub feature_flags: BTreeSet<String>,
    pub supported_lifecycle_states: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub account_ref: String,
    pub stable_subject_ref: String,
    pub identifiers: Vec<AccountIdentifier>,
    pub minimized_attributes: BTreeMap<String, String>,
    pub lifecycle_state: AccountLifecycleState,
    pub linked_identities: Vec<LinkedIdentityReference>,
    pub organization_refs: Vec<String>,
    pub tenant_refs: Vec<String>,
    pub recovery_refs: Vec<AccountRecoveryReference>,
    pub audit_refs: Vec<AccountAuditReference>,
    pub version_token_hash: String,
    pub freshness: AccountFreshness,
    pub redaction_class: String,
}

impl AccountRecord {
    /// Bound account projections before they enter trace, snapshot, or SDK evidence.
    pub fn is_bounded(&self, max_identifiers: usize, max_links: usize) -> bool {
        self.identifiers.len() <= max_identifiers && self.linked_identities.len() <= max_links
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIdentifier {
    pub identifier_ref: String,
    pub identifier_kind: String,
    pub normalized_value_hash: String,
    pub verification_state: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAttributePatch {
    pub patch_ref: String,
    pub attributes: BTreeMap<String, String>,
    pub custom_schema_refs: BTreeSet<String>,
    pub profile_preference_boundary: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLifecycleState {
    pub state: String,
    pub provider_state_ref: Option<String>,
    pub custom_state_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLifecycleTransitionPlan {
    pub plan_ref: String,
    pub from_state: String,
    pub to_state: String,
    pub requires_approval: bool,
    pub validation_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLifecycleTransitionRequest {
    pub plan_ref: String,
    pub idempotency_key_hash: String,
    pub approval_ref: Option<String>,
    pub version_token_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLifecycleTransitionResult {
    pub transition_ref: String,
    pub state: AccountLifecycleState,
    pub side_effect_evidence_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedIdentityReference {
    pub link_ref: String,
    pub provider_class: String,
    pub issuer_ref: String,
    pub external_subject_ref: String,
    pub assurance_level: String,
    pub link_state: String,
    pub freshness: AccountFreshness,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecoveryReference {
    pub recovery_ref: String,
    pub recovery_kind: String,
    pub contact_ref: Option<String>,
    pub reset_flow_ref: Option<String>,
    pub support_case_ref: Option<String>,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAuditReference {
    pub event_ref: String,
    pub actor_ref: String,
    pub event_type: String,
    pub event_timestamp_epoch_ms: u64,
    pub bounded_reason_code: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAuditExportPlan {
    pub export_ref: String,
    pub scope_ref: String,
    pub format: String,
    pub redaction_profile: String,
    pub retention_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_identity_command_wrappers!(
    AccountInspectProviderCommand,
    AccountDescribeSchemaCommand,
    AccountPlanCreateCommand,
    AccountCreateAccountCommand,
    AccountReadAccountCommand,
    AccountSearchAccountsCommand,
    AccountPlanUpdateCommand,
    AccountUpdateAccountCommand,
    AccountPlanLifecycleTransitionCommand,
    AccountLifecycleTransitionRequestCommand,
    AccountLinkIdentityCommand,
    AccountUnlinkIdentityCommand,
    AccountSyncStatusCommand,
    AccountSetRecoveryReferenceCommand,
    AccountInspectAccountAuditCommand,
    AccountPlanAuditExportCommand,
    AccountAuditExportRequestCommand,
    AccountGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountResultStatus {
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
    LifecycleInvalid,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountResultEnvelope<T> {
    pub status: AccountResultStatus,
    pub data: Option<T>,
    pub page: Option<IdentityPackPage<T>>,
    pub error: Option<IdentityPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub account_hash: String,
    pub transition_hash: String,
    pub link_hash: String,
    pub artifact_hash: String,
}

pub fn identity_account_descriptor_hashes() -> AccountDescriptorHashes {
    let freshness = AccountFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    let link = LinkedIdentityReference {
        link_ref: "link".into(),
        provider_class: "mock".into(),
        issuer_ref: "issuer".into(),
        external_subject_ref: "external-subject".into(),
        assurance_level: "aal2".into(),
        link_state: "linked".into(),
        freshness: freshness.clone(),
        replay_pointer: "replay".into(),
    };
    let account = AccountRecord {
        account_ref: "account".into(),
        stable_subject_ref: "subject".into(),
        identifiers: vec![AccountIdentifier {
            identifier_ref: "email".into(),
            identifier_kind: "email".into(),
            normalized_value_hash: "hash".into(),
            verification_state: "verified".into(),
            redaction_class: "hash_only".into(),
        }],
        lifecycle_state: AccountLifecycleState {
            state: "active".into(),
            ..Default::default()
        },
        linked_identities: vec![link.clone()],
        version_token_hash: "version".into(),
        freshness,
        redaction_class: "identity_reference_only".into(),
        ..Default::default()
    };
    AccountDescriptorHashes {
        command_schema_hash: account_stable_hash(&IDENTITY_ACCOUNT_COMMANDS),
        result_schema_hash: account_stable_hash(&AccountResultStatus::Success),
        descriptor_hash: account_stable_hash(&identity_account_pack_definition()),
        provider_capability_hash: account_stable_hash(&AccountProviderCapability {
            provider_class: "mock".into(),
            feature_flags: BTreeSet::from(["lifecycle".into(), "linked_identity".into()]),
            supported_lifecycle_states: BTreeSet::from(["active".into(), "suspended".into()]),
            limits: BTreeMap::from([("max_page_size".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        account_hash: account_stable_hash(&account),
        transition_hash: account_stable_hash(&AccountLifecycleTransitionPlan {
            plan_ref: "transition".into(),
            from_state: "active".into(),
            to_state: "suspended".into(),
            requires_approval: true,
            ..Default::default()
        }),
        link_hash: account_stable_hash(&link),
        artifact_hash: account_stable_hash(&AccountArtifactHandle {
            artifact_id: "artifact".into(),
            content_class: "audit_export".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            access_policy_ref: "policy".into(),
        }),
    }
}

pub fn account_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    identity_stable_hash(value)
}
