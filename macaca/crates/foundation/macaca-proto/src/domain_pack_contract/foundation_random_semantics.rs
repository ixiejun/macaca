//! Provider-neutral admission, quota, and redaction rules for random commands.
//!
//! The Specification objects in this module run before an RNG provider is
//! selected. They validate only command metadata and policy evidence, so this
//! protocol layer never handles generated values, seed material, credentials,
//! or provider-native handles. A runtime service owns entropy collection and
//! records the resulting trace/audit event after this gate admits a command.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::foundation_random::{
    RandomAlphabetClass, RandomBytesCommand, RandomIntegerCommand, RandomNonceCommand,
    RandomOutputEncoding, RandomReplayPolicy, RandomResultStatus, RandomStrengthClass,
    RandomTestStreamCreateCommand, RandomTokenCommand, RandomUuidV4Command,
};
const MAX_AUDIT_REFERENCE: usize = 160;

/// Resource units reserved before a random request reaches a provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomResourceReservation {
    pub byte_units: u32,
    pub token_units: u32,
    pub request_units: u32,
    pub deterministic_streams: u32,
}

/// Per-scope resource ceilings configured by policy or a service decorator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomResourceLimits {
    pub max_byte_units: u32,
    pub max_token_units: u32,
    pub max_request_units: u32,
    pub max_deterministic_streams: u32,
}

/// Input facts that the policy decorator passes to this pre-provider gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomPolicyContext {
    pub declared_scopes: BTreeSet<String>,
    pub policy_allowed: bool,
    pub provider_available: bool,
    pub entropy_available: bool,
    pub provider_blocked: bool,
    pub supports_bias_free_integer: bool,
    pub supports_uuid_v4: bool,
    pub supports_deterministic_streams: bool,
    pub replay_context: RandomReplayPolicy,
    pub max_bytes_per_request: u32,
    pub max_blocking_ms: u64,
    pub max_token_length: u32,
    pub limits: RandomResourceLimits,
    pub current: RandomResourceReservation,
}

/// Stable, trace-safe reason for a failed admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomAdmissionFailure {
    PermissionNotDeclared,
    PolicyDenied,
    ProviderUnavailable,
    EntropyUnavailable,
    ProviderBlocked,
    InvalidLength,
    InvalidRange,
    InvalidAlphabet,
    Unsupported,
    DeterministicNotAllowed,
    QuotaExceeded,
}

impl RandomAdmissionFailure {
    /// Convert gate outcomes to the public result status without leaking detail.
    pub fn status(self) -> RandomResultStatus {
        match self {
            Self::PermissionNotDeclared | Self::PolicyDenied => RandomResultStatus::Denied,
            Self::ProviderUnavailable => RandomResultStatus::Unavailable,
            Self::EntropyUnavailable => RandomResultStatus::EntropyUnavailable,
            Self::ProviderBlocked => RandomResultStatus::Blocked,
            Self::InvalidLength => RandomResultStatus::InvalidLength,
            Self::InvalidRange => RandomResultStatus::InvalidRange,
            Self::InvalidAlphabet => RandomResultStatus::InvalidAlphabet,
            Self::Unsupported => RandomResultStatus::Unsupported,
            Self::DeterministicNotAllowed => RandomResultStatus::DeterministicNotAllowed,
            Self::QuotaExceeded => RandomResultStatus::QuotaExceeded,
        }
    }
}

/// Validate a byte command before a CSPRNG provider can be called.
pub fn preflight_bytes(
    command: &RandomBytesCommand,
    context: &RandomPolicyContext,
) -> Result<RandomResourceReservation, RandomAdmissionFailure> {
    require_scope(context, "random.generate")?;
    require_secure_provider(context)?;
    if command.strength == RandomStrengthClass::DeterministicTest {
        return Err(RandomAdmissionFailure::DeterministicNotAllowed);
    }
    if !command.is_bounded_request(context.max_bytes_per_request, context.max_blocking_ms) {
        return Err(RandomAdmissionFailure::InvalidLength);
    }
    reserve(
        context,
        RandomResourceReservation {
            byte_units: command.length,
            request_units: 1,
            ..Default::default()
        },
    )
}

/// Validate a token request and reserve its bounded output cost.
pub fn preflight_token(
    command: &RandomTokenCommand,
    context: &RandomPolicyContext,
) -> Result<RandomResourceReservation, RandomAdmissionFailure> {
    require_scope(context, "random.token")?;
    require_secure_provider(context)?;
    if !command.is_bounded_request(context.max_token_length) {
        return Err(RandomAdmissionFailure::InvalidLength);
    }
    if matches!(command.alphabet, RandomAlphabetClass::CustomPolicyBounded)
        && !context.policy_allowed
    {
        return Err(RandomAdmissionFailure::InvalidAlphabet);
    }
    reserve(
        context,
        RandomResourceReservation {
            token_units: command.char_length,
            request_units: 1,
            ..Default::default()
        },
    )
}

/// Validate bias-free integer capability and its exclusive range.
pub fn preflight_integer(
    command: &RandomIntegerCommand,
    context: &RandomPolicyContext,
) -> Result<RandomResourceReservation, RandomAdmissionFailure> {
    require_scope(context, "random.generate")?;
    require_secure_provider(context)?;
    if !command.has_valid_range() {
        return Err(RandomAdmissionFailure::InvalidRange);
    }
    if command.require_bias_free && !context.supports_bias_free_integer {
        return Err(RandomAdmissionFailure::Unsupported);
    }
    reserve(
        context,
        RandomResourceReservation {
            request_units: 1,
            ..Default::default()
        },
    )
}

/// Validate UUID and nonce feature use without exposing generated identifiers.
pub fn preflight_identifier(
    uuid: Option<&RandomUuidV4Command>,
    nonce: Option<&RandomNonceCommand>,
    context: &RandomPolicyContext,
) -> Result<RandomResourceReservation, RandomAdmissionFailure> {
    let scope = if nonce.is_some() {
        "random.nonce"
    } else {
        "random.identifier"
    };
    require_scope(context, scope)?;
    require_secure_provider(context)?;
    if uuid.is_some_and(|command| command.count == 0 || !context.supports_uuid_v4) {
        return Err(RandomAdmissionFailure::Unsupported);
    }
    if let Some(command) = nonce {
        if command.byte_length == 0 || command.byte_length > context.max_bytes_per_request {
            return Err(RandomAdmissionFailure::InvalidLength);
        }
        if !matches!(
            command.encoding,
            RandomOutputEncoding::RawBytes
                | RandomOutputEncoding::Hex
                | RandomOutputEncoding::Base64Url
        ) {
            return Err(RandomAdmissionFailure::InvalidAlphabet);
        }
    }
    let byte_units = nonce.map_or(0, |command| command.byte_length);
    reserve(
        context,
        RandomResourceReservation {
            byte_units,
            request_units: 1,
            ..Default::default()
        },
    )
}

/// Validate creation of a deterministic stream exclusively for test/replay use.
pub fn preflight_test_stream(
    command: &RandomTestStreamCreateCommand,
    context: &RandomPolicyContext,
) -> Result<RandomResourceReservation, RandomAdmissionFailure> {
    require_scope(context, "random.test_seed")?;
    if !context.policy_allowed {
        return Err(RandomAdmissionFailure::PolicyDenied);
    }
    if !context.provider_available {
        return Err(RandomAdmissionFailure::ProviderUnavailable);
    }
    if !context.supports_deterministic_streams
        || !command.is_allowed_in_context(context.replay_context)
    {
        return Err(RandomAdmissionFailure::DeterministicNotAllowed);
    }
    reserve(
        context,
        RandomResourceReservation {
            request_units: 1,
            deterministic_streams: 1,
            ..Default::default()
        },
    )
}

/// Run a provider closure only when a preflight decision has admitted it.
///
/// This helper is intentionally generic so tests and service decorators can
/// prove that denied or quota-exceeded commands never reach a provider.
pub fn dispatch_after_preflight<T>(
    decision: Result<RandomResourceReservation, RandomAdmissionFailure>,
    provider: impl FnOnce() -> T,
) -> Result<T, RandomAdmissionFailure> {
    decision.map(|_| provider())
}

/// Reserve units atomically in value form; persistence belongs to a provider decorator.
pub fn reserve(
    context: &RandomPolicyContext,
    requested: RandomResourceReservation,
) -> Result<RandomResourceReservation, RandomAdmissionFailure> {
    let next = RandomResourceReservation {
        byte_units: context
            .current
            .byte_units
            .saturating_add(requested.byte_units),
        token_units: context
            .current
            .token_units
            .saturating_add(requested.token_units),
        request_units: context
            .current
            .request_units
            .saturating_add(requested.request_units),
        deterministic_streams: context
            .current
            .deterministic_streams
            .saturating_add(requested.deterministic_streams),
    };
    if next.byte_units > context.limits.max_byte_units
        || next.token_units > context.limits.max_token_units
        || next.request_units > context.limits.max_request_units
        || next.deterministic_streams > context.limits.max_deterministic_streams
    {
        return Err(RandomAdmissionFailure::QuotaExceeded);
    }
    Ok(next)
}

/// Project only safe command facts into trace, audit, or snapshot metadata.
pub fn redacted_random_audit_fields(
    command_name: &str,
    length_or_count: u32,
    purpose: &str,
    trace_id: &str,
) -> Option<RandomAuditFields> {
    (safe(command_name) && safe(purpose) && safe(trace_id)).then(|| RandomAuditFields {
        command_name: command_name.into(),
        length_or_count,
        purpose: purpose.into(),
        trace_id: trace_id.into(),
    })
}

/// Sanitized event payload deliberately lacking generated material and seeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomAuditFields {
    pub command_name: String,
    pub length_or_count: u32,
    pub purpose: String,
    pub trace_id: String,
}

fn require_scope(context: &RandomPolicyContext, scope: &str) -> Result<(), RandomAdmissionFailure> {
    if !context.declared_scopes.contains(scope) {
        return Err(RandomAdmissionFailure::PermissionNotDeclared);
    }
    if !context.policy_allowed {
        return Err(RandomAdmissionFailure::PolicyDenied);
    }
    Ok(())
}

fn require_secure_provider(context: &RandomPolicyContext) -> Result<(), RandomAdmissionFailure> {
    if !context.provider_available {
        return Err(RandomAdmissionFailure::ProviderUnavailable);
    }
    if !context.entropy_available {
        return Err(RandomAdmissionFailure::EntropyUnavailable);
    }
    if context.provider_blocked {
        return Err(RandomAdmissionFailure::ProviderBlocked);
    }
    Ok(())
}

fn safe(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_AUDIT_REFERENCE
        && !value.chars().any(char::is_control)
        && !["seed", "secret", "token", "payload", "credential"]
            .iter()
            .any(|term| value.to_ascii_lowercase().contains(term))
}
