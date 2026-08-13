//! Provider-neutral workflow recovery semantics.
//!
//! This module uses State, Specification, Strategy, and Memento patterns. It
//! carries bounded hashes and references only; checkpoint bytes, prompts,
//! manifests, credentials, package bytes, and provider payloads stay external.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Transient,
    Permanent,
    PolicyDenied,
    QuotaExhausted,
    ProviderUnavailable,
    CorruptedCheckpoint,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryLifecycleState {
    Failed,
    Classified,
    Planned,
    Retrying,
    Repairing,
    Compensating,
    Resumed,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureRecordV1 {
    pub schema_version: String,
    pub failure_ref: String,
    pub origin_service_ref: String,
    pub class: FailureClass,
    pub reason_code: String,
    pub trace_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPointV1 {
    pub point_ref: String,
    pub owner_service_ref: String,
    pub checkpoint_ref: String,
    pub integrity_hash: String,
    pub compatibility_version: String,
    pub replay_cursor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicyV1 {
    pub policy_ref: String,
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub terminal_on_exhaustion: bool,
}
impl RetryPolicyV1 {
    pub fn allows_attempt(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
    pub fn backoff_for(&self, attempt: u32) -> u64 {
        self.backoff_ms
            .saturating_mul(u64::from(attempt.saturating_add(1)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPlanV1 {
    pub plan_ref: String,
    pub failure_ref: String,
    pub recovery_point_ref: Option<String>,
    pub action_refs: Vec<String>,
    pub retry_policy: Option<RetryPolicyV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairActionV1 {
    pub action_ref: String,
    pub action_kind: String,
    pub target_ref: String,
    pub policy_ref: String,
    pub compensation_ref: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompensationRefV1 {
    pub compensation_ref: String,
    pub original_action_ref: String,
    pub order_index: u32,
    pub status: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumePlanV1 {
    pub resume_ref: String,
    pub recovery_point_ref: String,
    pub target_service_ref: String,
    pub replay_cursor: String,
    pub compatibility_checked: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayExportV1 {
    pub export_ref: String,
    pub trace_ref: String,
    pub redacted_bundle_ref: String,
    pub event_count: u64,
    pub payloads_redacted: bool,
}

pub struct RecoveryLifecycleSpec;
impl RecoveryLifecycleSpec {
    pub fn allows(from: RecoveryLifecycleState, to: RecoveryLifecycleState) -> bool {
        matches!(
            (from, to),
            (
                RecoveryLifecycleState::Failed,
                RecoveryLifecycleState::Classified
            ) | (
                RecoveryLifecycleState::Classified,
                RecoveryLifecycleState::Planned
            ) | (
                RecoveryLifecycleState::Planned,
                RecoveryLifecycleState::Retrying
            ) | (
                RecoveryLifecycleState::Planned,
                RecoveryLifecycleState::Repairing
            ) | (
                RecoveryLifecycleState::Repairing,
                RecoveryLifecycleState::Compensating
            ) | (
                RecoveryLifecycleState::Retrying,
                RecoveryLifecycleState::Resumed
            ) | (
                RecoveryLifecycleState::Retrying,
                RecoveryLifecycleState::Terminal
            ) | (
                RecoveryLifecycleState::Repairing,
                RecoveryLifecycleState::Resumed
            ) | (
                RecoveryLifecycleState::Compensating,
                RecoveryLifecycleState::Terminal
            )
        )
    }
}

pub struct RecoveryPointSpec;
impl RecoveryPointSpec {
    pub fn is_compatible(
        point: &RecoveryPointV1,
        expected_hash: &str,
        expected_version: &str,
    ) -> bool {
        !point.checkpoint_ref.is_empty()
            && point.integrity_hash == expected_hash
            && point.compatibility_version == expected_version
            && !point.replay_cursor.is_empty()
    }
}

pub struct FailureClassificationSpec;
impl FailureClassificationSpec {
    pub fn classify(reason_code: &str) -> FailureClass {
        match reason_code {
            "timeout" | "temporarily_unavailable" => FailureClass::Transient,
            "policy_denied" => FailureClass::PolicyDenied,
            "quota_exhausted" => FailureClass::QuotaExhausted,
            "provider_unavailable" => FailureClass::ProviderUnavailable,
            "checkpoint_corrupt" => FailureClass::CorruptedCheckpoint,
            "invalid_request" | "unsupported" => FailureClass::Permanent,
            _ => FailureClass::Unknown,
        }
    }
}

impl ResumePlanV1 {
    pub fn can_resume(&self) -> bool {
        self.compatibility_checked
            && !self.recovery_point_ref.is_empty()
            && !self.replay_cursor.is_empty()
    }
}
impl ReplayExportV1 {
    pub fn is_safe(&self) -> bool {
        self.payloads_redacted
            && !self.redacted_bundle_ref.is_empty()
            && self.event_count <= 100_000
    }
}
