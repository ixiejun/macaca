//! Runtime sandbox and resource guard for the default WASM provider.
//!
//! The guard is a Decorator-style component used by the provider before guest
//! code can run.  It keeps policy decisions outside the private engine adapter,
//! which preserves a clean Strategy boundary: future engines can reuse the same
//! guard while replacing compile/instantiate/invoke internals.  Every deny path
//! emits a sanitized audit report and logs stable identifiers only.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use macaca_proto::{
    ApplicationHostCommand, PackageRuntimeKind, TraceContext, WasmResourceAuditDecision,
    WasmResourceAuditReport, WasmResourcePolicy, WasmRuntimeErrorKind, WasmRuntimeSessionRequest,
    WasmSandboxPolicy,
};
use tracing::{info, warn};

use super::errors::WasmRuntimeHostError;

const MAX_PAYLOAD_METADATA_KEY: &str = "wasm.resource.max_payload_bytes";
const MAX_CONCURRENCY_METADATA_KEY: &str = "wasm.resource.max_concurrent_sessions";

/// Shared guard state used by all sessions created from one provider instance.
#[derive(Debug, Clone, Default)]
pub(crate) struct WasmSandboxGuard {
    active_sessions: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl WasmSandboxGuard {
    /// Admit a session and return an RAII permit that releases on drop.
    ///
    /// The method is intentionally synchronous because it protects a small
    /// in-memory counter and never performs I/O while holding the mutex.  The
    /// quota key is derived from application id and ability id, avoiding any
    /// business-specific workflow or provider names in the enforcement path.
    pub(crate) fn admit_session(
        &self,
        request: &WasmRuntimeSessionRequest,
    ) -> Result<WasmSessionPermit, WasmRuntimeHostError> {
        let policy = active_resource_policy(request);
        let quota_key = policy
            .quota
            .as_ref()
            .map(|value| value.stable_label())
            .unwrap_or_else(|| {
                format!("{}:{}:session", request.application_id, request.ability_id)
            });
        let limit = policy.max_concurrent_sessions.unwrap_or(u64::MAX);
        let mut active = self.active_sessions.lock().map_err(|_| {
            WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::ResourceExhausted,
                "WASM concurrency guard state is unavailable",
            )
        })?;
        let current = *active.get(&quota_key).unwrap_or(&0);
        if current >= limit {
            let audit = audit_report(
                request,
                WasmResourceAuditDecision::Throttle,
                "concurrency_limit_exceeded",
                "session",
                request.trace.clone(),
                "WASM session concurrency limit was exceeded",
            );
            warn!(
                trace_id = audit.trace_id.as_deref().unwrap_or("none"),
                application_id = %audit.application_id,
                ability_id = %audit.ability_id,
                scope = %audit.scope,
                reason_code = %audit.reason_code,
                active_sessions = current,
                max_concurrent_sessions = limit,
                "WASM sandbox guard denied session admission"
            );
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::ResourceExhausted,
                "WASM session concurrency limit was exceeded",
            ));
        }
        active.insert(quota_key.clone(), current + 1);
        let audit = audit_report(
            request,
            WasmResourceAuditDecision::Allow,
            "session_admitted",
            "session",
            request.trace.clone(),
            "WASM session admitted by sandbox guard",
        );
        info!(
            trace_id = audit.trace_id.as_deref().unwrap_or("none"),
            application_id = %audit.application_id,
            ability_id = %audit.ability_id,
            scope = %audit.scope,
            reason_code = %audit.reason_code,
            active_sessions = current + 1,
            max_concurrent_sessions = limit,
            "WASM sandbox guard admitted session"
        );
        Ok(WasmSessionPermit {
            quota_key,
            active_sessions: Arc::clone(&self.active_sessions),
        })
    }

    /// Validate a command payload before the private engine is invoked.
    ///
    /// The guard serializes the JSON command payload only to measure its byte
    /// length.  It never logs or stores the payload body, which keeps guest
    /// input and secrets out of telemetry even when a request is rejected.
    pub(crate) fn check_dispatch_payload(
        &self,
        request: &WasmRuntimeSessionRequest,
        command: &ApplicationHostCommand,
        trace: TraceContext,
    ) -> Result<(), WasmRuntimeHostError> {
        let policy = active_resource_policy(request);
        if policy.max_wall_time_ms == Some(0) {
            let audit = audit_report(
                request,
                WasmResourceAuditDecision::Timeout,
                "wall_clock_timeout",
                "dispatch",
                Some(trace),
                "WASM dispatch exceeded the sandbox wall-clock policy",
            );
            warn!(
                trace_id = audit.trace_id.as_deref().unwrap_or("none"),
                application_id = %audit.application_id,
                ability_id = %audit.ability_id,
                scope = %audit.scope,
                reason_code = %audit.reason_code,
                max_wall_time_ms = 0,
                "WASM sandbox guard denied dispatch by wall-clock timeout policy"
            );
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::Timeout,
                "WASM dispatch exceeded the sandbox wall-clock policy",
            ));
        }
        let limit = policy.max_payload_bytes.unwrap_or(u64::MAX);
        let payload_bytes = serde_json::to_vec(&command.payload)
            .map(|value| value.len() as u64)
            .unwrap_or(u64::MAX);
        if payload_bytes > limit {
            let audit = audit_report(
                request,
                WasmResourceAuditDecision::Deny,
                "payload_too_large",
                "dispatch",
                Some(trace),
                "WASM command payload exceeded the sandbox policy limit",
            );
            warn!(
                trace_id = audit.trace_id.as_deref().unwrap_or("none"),
                application_id = %audit.application_id,
                ability_id = %audit.ability_id,
                scope = %audit.scope,
                reason_code = %audit.reason_code,
                payload_bytes = payload_bytes,
                max_payload_bytes = limit,
                "WASM sandbox guard denied oversized command payload"
            );
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::ResourceExhausted,
                "WASM command payload exceeded the sandbox policy limit",
            ));
        }
        info!(
            trace_id = %trace.trace_id,
            application_id = %request.application_id,
            ability_id = %request.ability_id,
            scope = "dispatch",
            reason_code = "payload_admitted",
            payload_bytes = payload_bytes,
            max_payload_bytes = limit,
            "WASM sandbox guard admitted command payload"
        );
        Ok(())
    }

    /// Return the default sandbox policy so descriptors can prove deny-by-default.
    pub(crate) fn sandbox_policy(&self) -> WasmSandboxPolicy {
        WasmSandboxPolicy::default()
    }
}

/// RAII permit for one active WASM session.
#[derive(Debug)]
pub(crate) struct WasmSessionPermit {
    quota_key: String,
    active_sessions: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl Drop for WasmSessionPermit {
    fn drop(&mut self) {
        let Ok(mut active) = self.active_sessions.lock() else {
            warn!(
                quota_key = %self.quota_key,
                reason_code = "concurrency_guard_release_failed",
                "WASM sandbox guard could not release session permit"
            );
            return;
        };
        let Some(current) = active.get_mut(&self.quota_key) else {
            return;
        };
        *current = current.saturating_sub(1);
        if *current == 0 {
            active.remove(&self.quota_key);
        }
        info!(
            quota_key = %self.quota_key,
            reason_code = "session_released",
            "WASM sandbox guard released session permit"
        );
    }
}

/// Build the active resource policy from defaults, profile resources, and metadata.
///
/// This is the runtime-host half of deterministic policy merging.  Admission
/// can use the same DTOs, while the host also accepts traceable metadata knobs
/// in tests and future deployment profiles without adding app-specific fields.
pub(crate) fn active_resource_policy(request: &WasmRuntimeSessionRequest) -> WasmResourcePolicy {
    let mut profile = WasmResourcePolicy::default_for(&request.application_id, &request.ability_id);
    profile.max_memory_bytes = request.profile.resources.max_memory_bytes;
    profile.max_fuel = request.profile.resources.max_fuel;
    profile.max_wall_time_ms = request.profile.resources.max_wall_time_ms;
    profile.max_host_calls = request.profile.resources.max_host_calls;
    profile.max_payload_bytes = parse_u64_metadata(request, MAX_PAYLOAD_METADATA_KEY);
    profile.max_concurrent_sessions = parse_u64_metadata(request, MAX_CONCURRENCY_METADATA_KEY);
    WasmResourcePolicy::merge_deterministic(&[
        WasmResourcePolicy::default_for(&request.application_id, &request.ability_id),
        profile,
    ])
}

fn parse_u64_metadata(request: &WasmRuntimeSessionRequest, key: &str) -> Option<u64> {
    request
        .metadata
        .get(key)
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn audit_report(
    request: &WasmRuntimeSessionRequest,
    decision: WasmResourceAuditDecision,
    reason_code: &str,
    scope: &str,
    trace: Option<TraceContext>,
    message: &str,
) -> WasmResourceAuditReport {
    WasmResourceAuditReport::new(
        decision,
        reason_code,
        PackageRuntimeKind::WasmComponent,
        request.application_id.clone(),
        request.ability_id.clone(),
        scope,
        trace,
        message,
    )
}
