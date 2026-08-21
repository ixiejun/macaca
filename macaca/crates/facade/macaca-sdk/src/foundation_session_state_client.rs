//! Provider-neutral SDK helpers for the foundation session-state pack.
//!
//! The helpers are a Facade over typed command DTOs. They create only traced
//! `ServiceCallCommand` values after admission; provider selection, persistence,
//! policy, approval, and side effects remain owned by the service runtime.

use macaca_proto::{
    DomainPackUnavailableDiagnostic, MacacaResult, SessionStateCheckpointRef,
    SessionStateClearSessionCommand, SessionStateCompactHistoryCommand,
    SessionStateCreateCheckpointCommand, SessionStateGetCommand, SessionStateKeyRef,
    SessionStateMergePatchCommand, SessionStatePutCommand, SessionStateRestoreCheckpointCommand,
    SessionStateRestorePlan, SessionStateRetentionPolicy, SessionStateRevision,
    SessionStateSessionRef, SessionStateValueRef, TraceContext, FOUNDATION_SESSION_STATE_PACK_ID,
    FOUNDATION_SESSION_STATE_SERVICE_ID,
};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

const MAX_LIST_PAGE_SIZE: u32 = 500;

/// Provider-neutral builder for one declared session-state command.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionStateDomainPackCommandBuilder {
    command_name: &'static str,
    payload: serde_json::Value,
    trace: TraceContext,
}

impl SessionStateDomainPackCommandBuilder {
    /// Build the canonical traced service call after effective capability admission.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        DomainPackServiceCallBuilder::new(
            FOUNDATION_SESSION_STATE_SERVICE_ID,
            self.command_name,
            self.payload,
            self.trace,
        )?
        .build(resolved)
    }
}

/// Build a read-only get request for an opaque session-state key.
pub fn session_state_get_command(
    key: SessionStateKeyRef,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command("session_state.get", &SessionStateGetCommand { key }, trace)
}

/// Build a revision-aware put request; values must remain opaque references.
pub fn session_state_put_command(
    key: SessionStateKeyRef,
    value: SessionStateValueRef,
    expected_revision: Option<SessionStateRevision>,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command(
        "session_state.put",
        &SessionStatePutCommand {
            key,
            value,
            expected_revision,
        },
        trace,
    )
}

/// Build a revision-aware merge request using an opaque patch artifact reference.
pub fn session_state_merge_patch_command(
    key: SessionStateKeyRef,
    patch_ref: String,
    expected_revision: Option<SessionStateRevision>,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command(
        "session_state.merge_patch",
        &SessionStateMergePatchCommand {
            key,
            patch_ref,
            expected_revision,
        },
        trace,
    )
}

/// Build a bounded checkpoint request with an explicit retention Memento.
pub fn session_state_create_checkpoint_command(
    session: SessionStateSessionRef,
    retention: SessionStateRetentionPolicy,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command(
        "session_state.create_checkpoint",
        &SessionStateCreateCheckpointCommand { session, retention },
        trace,
    )
}

/// Build a restore request that is permanently constrained to dry-run mode.
pub fn session_state_restore_dry_run_command(
    checkpoint: SessionStateCheckpointRef,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command(
        "session_state.restore_checkpoint",
        &SessionStateRestoreCheckpointCommand {
            plan: SessionStateRestorePlan {
                checkpoint,
                dry_run: true,
                cross_session_allowed: false,
            },
        },
        trace,
    )
}

/// Build a dry-run history compaction request so callers can inspect impact first.
pub fn session_state_compact_dry_run_command(
    session: SessionStateSessionRef,
    before_revision: SessionStateRevision,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command(
        "session_state.compact_history",
        &SessionStateCompactHistoryCommand {
            session,
            before_revision,
            dry_run: true,
        },
        trace,
    )
}

/// Build a dry-run clear request; actual destructive clear remains runtime-governed.
pub fn session_state_clear_dry_run_command(
    session: SessionStateSessionRef,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    command(
        "session_state.clear_session",
        &SessionStateClearSessionCommand {
            session,
            dry_run: true,
        },
        trace,
    )
}

/// Validate a bounded page size without exposing provider cursors through helpers.
pub fn session_state_validate_page_size(page_size: u32) -> MacacaResult<()> {
    if (1..=MAX_LIST_PAGE_SIZE).contains(&page_size) {
        Ok(())
    } else {
        Err(macaca_proto::MacacaError::Config(format!(
            "session-state page_size must be between 1 and {MAX_LIST_PAGE_SIZE}"
        )))
    }
}

/// Return only this pack's sanitized unavailable diagnostics.
pub fn session_state_unavailable_diagnostics(
    resolved: &DomainPackResolveResult,
) -> Vec<DomainPackUnavailableDiagnostic> {
    resolved
        .unavailable
        .iter()
        .filter(|diagnostic| diagnostic.pack_id == FOUNDATION_SESSION_STATE_PACK_ID)
        .cloned()
        .collect()
}

fn command<T: serde::Serialize>(
    command_name: &'static str,
    request: &T,
    trace: TraceContext,
) -> MacacaResult<SessionStateDomainPackCommandBuilder> {
    Ok(SessionStateDomainPackCommandBuilder {
        command_name,
        payload: serde_json::to_value(request)?,
        trace,
    })
}

#[cfg(test)]
#[path = "foundation_session_state_client_tests.rs"]
mod tests;
