//! Provider-neutral SDK helpers for the foundation key-value state pack.
//!
//! The helpers model bounded client intent and construct only traced generic
//! service calls. They never execute retry loops, subscribe to a provider, or
//! access state directly; the service runtime owns those side effects.

use macaca_proto::{
    DomainPackUnavailableDiagnostic, KeyValueCompareAndSetCommand, KeyValueConflictMode,
    KeyValueKeyRef, KeyValueListKeysCommand, KeyValueNamespaceRef, KeyValuePutCommand,
    KeyValueRestoreNamespaceCommand, KeyValueRevision, KeyValueSnapshotRef, KeyValueTtlPolicy,
    KeyValueTypedValueRef, MacacaError, MacacaResult, TraceContext,
    FOUNDATION_KEY_VALUE_STATE_PACK_ID, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

const MAX_CAS_ATTEMPTS: u8 = 8;
const MAX_SCAN_PAGE_SIZE: u32 = 500;

pub use super::foundation_key_value_state_watch::{
    key_value_watch_subscription, KeyValueWatchCancellation, KeyValueWatchSubscription,
};

/// Provider-neutral builder for one declared key-value state command.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyValueDomainPackCommandBuilder {
    command_name: &'static str,
    payload: serde_json::Value,
    trace: TraceContext,
}

impl KeyValueDomainPackCommandBuilder {
    /// Build the canonical traced call after capability admission.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        DomainPackServiceCallBuilder::new(
            FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            self.command_name,
            self.payload,
            self.trace,
        )?
        .build(resolved)
    }
}

/// A bounded CAS retry plan; callers own result handling and transport retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyValueCasUpdatePlan {
    max_attempts: u8,
}

impl KeyValueCasUpdatePlan {
    /// Limit optimistic retries so a conflict cannot cause an unbounded SDK loop.
    pub fn new(max_attempts: u8) -> MacacaResult<Self> {
        if !(1..=MAX_CAS_ATTEMPTS).contains(&max_attempts) {
            return Err(MacacaError::Config(format!(
                "key-value CAS attempts must be between 1 and {MAX_CAS_ATTEMPTS}"
            )));
        }
        Ok(Self { max_attempts })
    }

    /// Return the caller-visible retry ceiling.
    pub const fn max_attempts(&self) -> u8 {
        self.max_attempts
    }

    /// Construct one compare-and-set attempt from a freshly read revision.
    pub fn build_attempt(
        &self,
        key: KeyValueKeyRef,
        expected_revision: KeyValueRevision,
        value: KeyValueTypedValueRef,
        trace: TraceContext,
    ) -> MacacaResult<KeyValueDomainPackCommandBuilder> {
        command(
            "kv.compare_and_set",
            &KeyValueCompareAndSetCommand {
                key,
                expected_revision,
                value,
            },
            trace,
        )
    }
}

/// A bounded prefix-list request that exposes no provider cursor internals.
pub fn key_value_bounded_prefix_scan_command(
    namespace: KeyValueNamespaceRef,
    prefix: Option<String>,
    page_size: u32,
    cursor: Option<String>,
    trace: TraceContext,
) -> MacacaResult<KeyValueDomainPackCommandBuilder> {
    if !(1..=MAX_SCAN_PAGE_SIZE).contains(&page_size) {
        return Err(MacacaError::Config(format!(
            "key-value scan page_size must be between 1 and {MAX_SCAN_PAGE_SIZE}"
        )));
    }
    command(
        "kv.list_keys",
        &KeyValueListKeysCommand {
            namespace,
            prefix,
            page_size,
            cursor,
        },
        trace,
    )
}

/// Build a TTL-backed cache entry using an opaque value reference.
pub fn key_value_ttl_cache_entry_command(
    key: KeyValueKeyRef,
    value: KeyValueTypedValueRef,
    ttl_seconds: u64,
    trace: TraceContext,
) -> MacacaResult<KeyValueDomainPackCommandBuilder> {
    if ttl_seconds == 0 {
        return Err(MacacaError::Config(
            "key-value cache entry TTL must be greater than zero".into(),
        ));
    }
    command(
        "kv.put",
        &KeyValuePutCommand {
            key,
            value,
            ttl: Some(KeyValueTtlPolicy {
                ttl_seconds: Some(ttl_seconds),
                expire_at_epoch_millis: None,
            }),
            conflict_mode: KeyValueConflictMode::Fail,
        },
        trace,
    )
}

/// Build a restore command that is permanently marked as a dry run.
pub fn key_value_restore_snapshot_dry_run_command(
    snapshot: KeyValueSnapshotRef,
    conflict_mode: KeyValueConflictMode,
    trace: TraceContext,
) -> MacacaResult<KeyValueDomainPackCommandBuilder> {
    command(
        "kv.restore_namespace",
        &KeyValueRestoreNamespaceCommand {
            snapshot,
            conflict_mode,
            dry_run: true,
        },
        trace,
    )
}

/// Return only this pack's already-sanitized unavailable diagnostics.
pub fn key_value_unavailable_diagnostics(
    resolved: &DomainPackResolveResult,
) -> Vec<DomainPackUnavailableDiagnostic> {
    resolved
        .unavailable
        .iter()
        .filter(|diagnostic| diagnostic.pack_id == FOUNDATION_KEY_VALUE_STATE_PACK_ID)
        .cloned()
        .collect()
}

fn command<T: serde::Serialize>(
    command_name: &'static str,
    request: &T,
    trace: TraceContext,
) -> MacacaResult<KeyValueDomainPackCommandBuilder> {
    Ok(KeyValueDomainPackCommandBuilder {
        command_name,
        payload: serde_json::to_value(request)?,
        trace,
    })
}

#[cfg(test)]
#[path = "foundation_key_value_state_client_tests.rs"]
mod tests;
