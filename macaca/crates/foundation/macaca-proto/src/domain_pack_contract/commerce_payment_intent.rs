use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::commerce_common::{
    commerce_pack_definition, commerce_stable_hash, define_commerce_command_wrappers,
    CommercePackCommandEnvelope, CommercePackDescriptor, CommercePackError, CommercePackPage,
    CommerceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const COMMERCE_PAYMENT_INTENT_PACK_ID: &str = "pack.commerce.payment.intent.v1";
pub const COMMERCE_PAYMENT_INTENT_SERVICE_ID: &str = "service.commerce.payment_intent";

pub const COMMERCE_PAYMENT_INTENT_COMMANDS: &[&str] = &[
    "payment_intent.inspect_provider",
    "payment_intent.describe_schema",
    "payment_intent.plan_intent",
    "payment_intent.create_intent",
    "payment_intent.plan_confirmation",
    "payment_intent.confirm",
    "payment_intent.inspect_action",
    "payment_intent.plan_capture",
    "payment_intent.capture",
    "payment_intent.plan_cancellation",
    "payment_intent.cancel",
    "payment_intent.get_status",
    "payment_intent.inspect_idempotency",
    "payment_intent.record_event_reference",
    "payment_intent.plan_audit_export",
    "payment_intent.audit_export_request",
    "payment_intent.get_artifact_handle",
];

pub const COMMERCE_PAYMENT_INTENT_TRACE_EVENTS: &[&str] = &[
    "payment_intent_pack_declared",
    "payment_intent_pack_admission_validated",
    "payment_intent_pack_policy_decision",
    "payment_intent_pack_provider_inspected",
    "payment_intent_pack_service_call_requested",
    "payment_intent_pack_service_call_succeeded",
    "payment_intent_pack_service_call_failed",
    "payment_intent_pack_state_transition_planned",
    "payment_intent_pack_sensitive_input_rejected",
    "payment_intent_pack_unavailable",
    "payment_intent_pack_snapshot_recorded",
];

const PAYMENT_INTENT_PERMISSION_SCOPES: &[&str] = &[
    "commerce.payment.intent.read",
    "commerce.payment.intent.create",
    "commerce.payment.intent.confirm",
    "commerce.payment.intent.capture",
    "commerce.payment.intent.cancel",
    "commerce.payment.intent.audit_export",
];

const PAYMENT_STATE_METADATA: &[(&str, &str)] = &[
    ("intent_state", "true"),
    ("capture_modes", "true"),
    ("cancel_void", "true"),
    ("idempotency", "required"),
];
const PAYMENT_ACTION_METADATA: &[(&str, &str)] = &[
    ("action_required", "handle_only"),
    ("client_secret", "false"),
    ("raw_credentials", "false"),
];
const PAYMENT_EVENT_METADATA: &[(&str, &str)] = &[
    ("event_references", "true"),
    ("webhook_body", "false"),
    ("audit_export", "handle_only"),
];
const PAYMENT_MOCK_METADATA: &[(&str, &str)] = &[
    ("intents", "synthetic"),
    ("actions", "synthetic"),
    ("callable", "false"),
];
const PAYMENT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("intents", "false"),
    ("capture", "false"),
    ("reason", "provider_not_installed"),
];

const PAYMENT_PROVIDER_CLASSES: &[CommerceProviderClass<'_>] = &[
    CommerceProviderClass {
        provider_class: "payment-intent-state",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PAYMENT_STATE_METADATA,
    },
    CommerceProviderClass {
        provider_class: "payment-action",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PAYMENT_ACTION_METADATA,
    },
    CommerceProviderClass {
        provider_class: "payment-event",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PAYMENT_EVENT_METADATA,
    },
    CommerceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PAYMENT_MOCK_METADATA,
    },
    CommerceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: PAYMENT_UNAVAILABLE_METADATA,
    },
];

pub fn commerce_payment_intent_pack_definition() -> DomainPackDefinition {
    commerce_pack_definition(CommercePackDescriptor {
        pack_id: COMMERCE_PAYMENT_INTENT_PACK_ID,
        child_change_id: "openspec:add-pack-commerce-payment-intent",
        docs_slug: "payment-intent",
        sdk_slug: "payment.intent",
        service_id: COMMERCE_PAYMENT_INTENT_SERVICE_ID,
        commands: COMMERCE_PAYMENT_INTENT_COMMANDS,
        permission_scopes: PAYMENT_INTENT_PERMISSION_SCOPES,
        provider_classes: PAYMENT_PROVIDER_CLASSES,
        health_probe: "payment_intent.inspect_provider",
        unavailable_reason: "commerce_payment_intent_provider_not_installed",
        replay_schema: "commerce.payment_intent.replay.v1",
        data_classification: "regulated_payment_intent_reference_metadata",
        retention_policy: "payment_intent_state_action_event_idempotency_and_audit_artifact_metadata_by_reference",
        redaction_policy: "raw_payment_credentials_client_secrets_provider_payloads_sca_payloads_wallet_cryptograms_webhooks_private_keys_signatures_and_unbounded_output_redacted",
        timeout_ms: 120_000,
        budget_units: 5,
        examples: &[
            "Declare `pack.commerce.payment.intent.v1` as optional until a payment-intent provider is installed.",
            "Use tokenized payment method references, action handles, event references, and artifact handles instead of raw gateway payloads.",
        ],
        migration_notes: &[
            "Payment-intent commands become callable only after an approved payment service provider registers matching schemas.",
            "Refunds, receipts, disputes, settlement, payouts, fraud decisions, and checkout UI remain separate capabilities.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentScope {
    pub tenant_scope: String,
    pub merchant_account_ref: String,
    pub order_or_cart_ref: Option<String>,
    pub payment_intent_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentProviderCapability {
    pub provider_class: String,
    pub payment_method_types: BTreeSet<String>,
    pub capture_modes: BTreeSet<String>,
    pub feature_flags: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub event_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentPlan {
    pub plan_ref: String,
    pub amount_micros: i64,
    pub currency: String,
    pub capture_mode: String,
    pub merchant_account_ref: String,
    pub order_or_cart_ref: Option<String>,
    pub payment_method: PaymentMethodReference,
    pub idempotency_key_hash: String,
}

impl PaymentIntentPlan {
    pub fn has_valid_amount(&self) -> bool {
        self.amount_micros > 0 && !self.currency.trim().is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentRecord {
    pub payment_intent_ref: String,
    pub amount_micros: i64,
    pub currency: String,
    pub capture_mode: String,
    pub state: String,
    pub action_requirements: Vec<PaymentActionRequirement>,
    pub authorization_refs: Vec<String>,
    pub capture_refs: Vec<String>,
    pub cancellation_refs: Vec<String>,
    pub freshness: PaymentIntentFreshness,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentMethodReference {
    pub token_ref: String,
    pub method_type: String,
    pub region_support: BTreeSet<String>,
    pub reusable: bool,
    pub risk_metadata_ref: Option<String>,
    pub raw_credential_rejected: bool,
}

impl PaymentMethodReference {
    /// Raw payment credentials must be denied before a provider call can be planned.
    pub fn is_tokenized_only(&self) -> bool {
        !self.token_ref.trim().is_empty() && !self.raw_credential_rejected
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentActionRequirement {
    pub action_ref: String,
    pub action_type: String,
    pub redirect_handle: Option<String>,
    pub expires_at_epoch_ms: u64,
    pub return_reference: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentAuthorization {
    pub authorization_ref: String,
    pub amount_micros: i64,
    pub currency: String,
    pub expires_at_epoch_ms: u64,
    pub provider_reference_hash: String,
    pub side_effect_evidence_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentCapture {
    pub capture_ref: String,
    pub amount_micros: i64,
    pub currency: String,
    pub partial_capture: bool,
    pub provider_reference_hash: String,
    pub side_effect_evidence_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentCancellation {
    pub cancellation_ref: String,
    pub reason_ref: String,
    pub provider_reference_hash: String,
    pub side_effect_evidence_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentEventReference {
    pub event_ref: String,
    pub provider_class: String,
    pub event_type: String,
    pub event_timestamp_epoch_ms: u64,
    pub delivery_id_hash: String,
    pub webhook_freshness: PaymentIntentFreshness,
    pub replay_pointer: String,
    pub bounded_result_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentAuditExportPlan {
    pub export_ref: String,
    pub scope_ref: String,
    pub format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentArtifactHandle {
    pub artifact_id: String,
    pub export_format: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub redaction_profile: String,
    pub access_policy_ref: String,
}

define_commerce_command_wrappers!(
    PaymentIntentInspectProviderCommand,
    PaymentIntentDescribeSchemaCommand,
    PaymentIntentPlanIntentCommand,
    PaymentIntentCreateIntentCommand,
    PaymentIntentPlanConfirmationCommand,
    PaymentIntentConfirmCommand,
    PaymentIntentInspectActionCommand,
    PaymentIntentPlanCaptureCommand,
    PaymentIntentCaptureCommand,
    PaymentIntentPlanCancellationCommand,
    PaymentIntentCancelCommand,
    PaymentIntentGetStatusCommand,
    PaymentIntentInspectIdempotencyCommand,
    PaymentIntentRecordEventReferenceCommand,
    PaymentIntentPlanAuditExportCommand,
    PaymentIntentAuditExportRequestCommand,
    PaymentIntentGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentIntentResultStatus {
    Success,
    Partial,
    ActionRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    ApprovalRequired,
    RawCredentialRejected,
    StateInvalid,
    ExportAccepted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentResultEnvelope<T> {
    pub status: PaymentIntentResultStatus,
    pub data: Option<T>,
    pub page: Option<CommercePackPage<T>>,
    pub error: Option<CommercePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub plan_hash: String,
    pub record_hash: String,
    pub method_hash: String,
    pub action_hash: String,
    pub capture_hash: String,
    pub event_hash: String,
    pub artifact_hash: String,
}

pub fn commerce_payment_intent_descriptor_hashes() -> PaymentIntentDescriptorHashes {
    let method = PaymentMethodReference {
        token_ref: "token".into(),
        method_type: "card".into(),
        region_support: BTreeSet::from(["US".into()]),
        reusable: false,
        raw_credential_rejected: false,
        ..Default::default()
    };
    let freshness = PaymentIntentFreshness {
        source_timestamp_epoch_ms: 1,
        event_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    PaymentIntentDescriptorHashes {
        command_schema_hash: payment_intent_stable_hash(&COMMERCE_PAYMENT_INTENT_COMMANDS),
        result_schema_hash: payment_intent_stable_hash(&PaymentIntentResultStatus::Success),
        descriptor_hash: payment_intent_stable_hash(&commerce_payment_intent_pack_definition()),
        provider_capability_hash: payment_intent_stable_hash(&PaymentIntentProviderCapability {
            provider_class: "mock".into(),
            payment_method_types: BTreeSet::from(["card".into()]),
            capture_modes: BTreeSet::from(["manual".into(), "automatic".into()]),
            feature_flags: BTreeSet::from(["action_required".into(), "events".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        plan_hash: payment_intent_stable_hash(&PaymentIntentPlan {
            plan_ref: "plan".into(),
            amount_micros: 1_000_000,
            currency: "USD".into(),
            capture_mode: "manual".into(),
            merchant_account_ref: "merchant".into(),
            payment_method: method.clone(),
            idempotency_key_hash: "idem".into(),
            ..Default::default()
        }),
        record_hash: payment_intent_stable_hash(&PaymentIntentRecord {
            payment_intent_ref: "intent".into(),
            amount_micros: 1_000_000,
            currency: "USD".into(),
            capture_mode: "manual".into(),
            state: "requires_confirmation".into(),
            freshness,
            redaction_class: "payment_reference_only".into(),
            ..Default::default()
        }),
        method_hash: payment_intent_stable_hash(&method),
        action_hash: payment_intent_stable_hash(&PaymentActionRequirement {
            action_ref: "action".into(),
            action_type: "redirect".into(),
            redirect_handle: Some("redirect".into()),
            expires_at_epoch_ms: 10,
            ..Default::default()
        }),
        capture_hash: payment_intent_stable_hash(&PaymentCapture {
            capture_ref: "capture".into(),
            amount_micros: 1_000_000,
            currency: "USD".into(),
            partial_capture: false,
            provider_reference_hash: "provider".into(),
            side_effect_evidence_ref: "evidence".into(),
        }),
        event_hash: payment_intent_stable_hash(&PaymentIntentEventReference {
            event_ref: "event".into(),
            provider_class: "mock".into(),
            event_type: "updated".into(),
            event_timestamp_epoch_ms: 1,
            delivery_id_hash: "delivery".into(),
            webhook_freshness: PaymentIntentFreshness {
                source_timestamp_epoch_ms: 1,
                event_timestamp_epoch_ms: Some(1),
                freshness_class: "fresh".into(),
            },
            replay_pointer: "replay".into(),
            bounded_result_code: "accepted".into(),
        }),
        artifact_hash: payment_intent_stable_hash(&PaymentIntentArtifactHandle {
            artifact_id: "artifact".into(),
            export_format: "json".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            redaction_profile: "payment".into(),
            access_policy_ref: "policy".into(),
        }),
    }
}

pub fn payment_intent_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    commerce_stable_hash(value)
}
