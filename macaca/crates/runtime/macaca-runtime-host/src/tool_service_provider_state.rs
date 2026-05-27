//! In-memory caches for Tool planning snapshots.
//!
//! These caches are Memento stores for bounded planning evidence.  They never
//! store raw provider payloads, credentials, or tool outputs.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use macaca_proto::{
    IndustrialToolDescriptor, ServiceHealth, ToolArtifactRef, ToolCatalogPlanResult,
    ToolCommandResult, ToolInvocationRef, ToolResultClass, ToolServiceSnapshotResult, TraceContext,
};

use crate::tool_service_result::command_result;

#[derive(Debug, Default)]
pub struct ToolServiceProviderState {
    last_plan: RwLock<Option<ToolCatalogPlanResult>>,
    provider_count: RwLock<usize>,
    invocation_results: RwLock<BTreeMap<String, ToolCommandResult>>,
    artifact_results: RwLock<BTreeMap<String, serde_json::Value>>,
    audit_events: RwLock<Vec<serde_json::Value>>,
    invocation_audits: RwLock<Vec<serde_json::Value>>,
}

impl ToolServiceProviderState {
    /// Seed the registered provider count before the first plan request.
    ///
    /// This avoids the previous fake `provider_count = 0` healthy diagnostic
    /// while still making plan-specific visible/hidden counts come from real
    /// contributor output.
    pub fn set_provider_count(&self, provider_count: usize) {
        *self
            .provider_count
            .write()
            .expect("tool provider cache lock poisoned") = provider_count;
    }

    pub fn provider_count(&self) -> usize {
        *self
            .provider_count
            .read()
            .expect("tool provider cache lock poisoned")
    }

    pub fn record_plan(&self, plan: ToolCatalogPlanResult, provider_count: usize) {
        self.record_plan_audit(&plan, provider_count);
        *self
            .last_plan
            .write()
            .expect("tool plan cache lock poisoned") = Some(plan);
        *self
            .provider_count
            .write()
            .expect("tool provider cache lock poisoned") = provider_count;
    }

    pub fn snapshot(&self, trace: TraceContext) -> ToolServiceSnapshotResult {
        let provider_count = *self
            .provider_count
            .read()
            .expect("tool provider cache lock poisoned");
        let plan = self
            .last_plan
            .read()
            .expect("tool plan cache lock poisoned");
        let plan = plan.as_ref().map(|plan| macaca_proto::ToolPlan {
            visible: plan.visible.clone(),
            hidden: plan.hidden.clone(),
            conflicts: plan.conflicts.clone(),
        });
        ToolServiceSnapshotResult {
            trace,
            health: ServiceHealth::Healthy,
            plan: plan.unwrap_or(macaca_proto::ToolPlan {
                visible: Vec::new(),
                hidden: Vec::new(),
                conflicts: Vec::new(),
            }),
            provider_count,
            captured_at: chrono::Utc::now(),
            metadata: Default::default(),
        }
    }

    /// Resolve a planned descriptor by stable id from the last bounded plan.
    ///
    /// This is a compatibility fallback for callers that send only a stable
    /// `tool_id`.  New framework adapters pass the descriptor selected from the
    /// model-visible plan so invocation routing does not depend on a stale
    /// cache.  The cache stores only sanitized descriptors, never raw tool
    /// input/output or provider payloads.
    pub fn descriptor_by_tool_id(&self, tool_id: &str) -> Option<IndustrialToolDescriptor> {
        let plan = self
            .last_plan
            .read()
            .expect("tool plan cache lock poisoned");
        let plan = plan.as_ref()?;
        plan.visible
            .iter()
            .chain(plan.hidden.iter())
            .find(|entry| entry.descriptor.stable_tool_id == tool_id)
            .map(|entry| entry.descriptor.clone())
    }

    /// Store the normalized command result under its invocation ref.
    ///
    /// The state acts as a small in-memory Memento store for this proposal
    /// slice.  Later artifact providers can replace the backing store without
    /// changing `service.tool` command/result contracts.
    pub fn record_invocation_result(&self, result: ToolCommandResult) {
        let Some(invocation_ref) = result.invocation_ref.as_ref() else {
            return;
        };
        self.invocation_results
            .write()
            .expect("tool invocation result cache lock poisoned")
            .insert(invocation_ref.0.clone(), result);
    }

    /// Record a sanitized invocation audit memento for replay.
    ///
    /// The caller supplies only bounded metadata and hashes. Raw input, raw
    /// output, prompts, headers, environment values, credentials, and provider
    /// payloads are deliberately not accepted by this method, which makes
    /// accidental leakage harder at the service boundary.
    pub fn record_invocation_audit(
        &self,
        result: &ToolCommandResult,
        tool_id: &str,
        provider_id: &str,
        service_id: &str,
        input_hash: String,
    ) {
        let output_hash = result
            .inline_output
            .as_ref()
            .map(stable_json_hash)
            .unwrap_or_else(|| "hash:none".into());
        let invocation_ref = result
            .invocation_ref
            .as_ref()
            .map(|value| value.0.clone())
            .unwrap_or_default();
        let audit = serde_json::json!({
            "trace_id": result.trace.trace_id,
            "application_id": null,
            "session_id": result.trace.session_id,
            "agent_name": result.trace.agent,
            "service_id": service_id,
            "provider_id": provider_id,
            "tool_id": tool_id,
            "status": result.status,
            "reason_code": result.error_summary.as_deref().unwrap_or(&result.status),
            "input_hash": input_hash,
            "output_hash": output_hash,
            "artifact_refs": result.artifact_refs.iter().map(|item| item.0.clone()).collect::<Vec<_>>(),
            "invocation_ref": invocation_ref,
            "latency_ms": result.metadata.get("latency_ms").cloned().unwrap_or_else(|| "0".into()),
            "captured_at": result.captured_at,
        });
        self.invocation_audits
            .write()
            .expect("tool invocation audit lock poisoned")
            .push(audit);
        if result.status == "denied" {
            self.push_event(serde_json::json!({
                "event": "tool.policy.decision",
                "trace_id": result.trace.trace_id,
                "tool_id": tool_id,
                "status": "denied",
                "reason_code": result.error_summary.as_deref().unwrap_or("policy_denied"),
                "captured_at": result.captured_at,
            }));
        }
        if result.status == "approval_required" {
            self.push_event(serde_json::json!({
                "event": "tool.approval.requested",
                "trace_id": result.trace.trace_id,
                "tool_id": tool_id,
                "status": result.status,
                "captured_at": result.captured_at,
            }));
        }
        if result.invocation_ref.is_some() {
            self.push_event(serde_json::json!({
                "event": "tool.result.persisted",
                "trace_id": result.trace.trace_id,
                "tool_id": tool_id,
                "artifact_count": result.artifact_refs.len(),
                "captured_at": result.captured_at,
            }));
        }
        for artifact_ref in &result.artifact_refs {
            self.push_event(serde_json::json!({
                "event": "tool.artifact.opened",
                "trace_id": result.trace.trace_id,
                "tool_id": tool_id,
                "artifact_ref": artifact_ref.0,
                "captured_at": result.captured_at,
            }));
        }
        self.push_event(serde_json::json!({
            "event": invocation_event_name(&result.status),
            "trace_id": result.trace.trace_id,
            "tool_id": tool_id,
            "status": result.status,
            "invocation_ref": invocation_ref,
            "captured_at": result.captured_at,
        }));
        self.push_event(serde_json::json!({
            "event": "tool.resource_lease.released",
            "trace_id": result.trace.trace_id,
            "tool_id": tool_id,
            "status": result.status,
            "captured_at": result.captured_at,
        }));
    }

    /// Append a sanitized lifecycle event for live/replay projections.
    pub fn record_invocation_event(
        &self,
        trace: &TraceContext,
        tool_id: &str,
        event: &str,
        status: &str,
    ) {
        self.push_event(serde_json::json!({
            "event": event,
            "trace_id": trace.trace_id,
            "tool_id": tool_id,
            "status": status,
            "captured_at": chrono::Utc::now(),
        }));
    }

    pub fn invocation_result(
        &self,
        invocation_ref: &ToolInvocationRef,
    ) -> Option<ToolCommandResult> {
        self.invocation_results
            .read()
            .expect("tool invocation result cache lock poisoned")
            .get(&invocation_ref.0)
            .cloned()
    }

    pub fn record_artifact(&self, artifact_ref: &ToolArtifactRef, payload: serde_json::Value) {
        self.artifact_results
            .write()
            .expect("tool artifact cache lock poisoned")
            .insert(artifact_ref.0.clone(), payload);
    }

    pub fn artifact(&self, artifact_ref: &ToolArtifactRef) -> Option<serde_json::Value> {
        self.artifact_results
            .read()
            .expect("tool artifact cache lock poisoned")
            .get(&artifact_ref.0)
            .cloned()
    }

    /// Build the provider-neutral replay payload returned by `tool.audit.query`.
    ///
    /// This is the runtime-host Memento read side. It returns aggregate counts,
    /// stable refs, hashes, timestamps, and sanitized event records. The method
    /// never reconstructs raw provider data and therefore remains safe for Web,
    /// CLI, frontend, and EventLog/SSE diagnostic surfaces.
    pub fn audit_query(&self, trace: TraceContext) -> ToolCommandResult {
        let plan = self
            .last_plan
            .read()
            .expect("tool plan cache lock poisoned")
            .clone();
        let plan_payload = plan.as_ref().map(|plan| {
            let mut reason_counts = serde_json::Map::new();
            for (key, value) in &plan.metadata {
                if let Some(reason) = key.strip_prefix("tool.reason.") {
                    reason_counts.insert(
                        reason.to_string(),
                        value.parse::<usize>().unwrap_or_default().into(),
                    );
                }
            }
            serde_json::json!({
                "trace_id": plan.trace.trace_id,
                "visible_count": plan.visible.len(),
                "hidden_count": plan.hidden.len(),
                "conflict_count": plan.conflicts.len(),
                "policy_refs": plan.audit_refs.iter().filter_map(audit_ref_value).collect::<Vec<_>>(),
                "reason_counts": reason_counts,
                "captured_at": plan.captured_at,
            })
        });
        let events = self
            .audit_events
            .read()
            .expect("tool audit event lock poisoned")
            .clone();
        let invocations = self
            .invocation_audits
            .read()
            .expect("tool invocation audit lock poisoned")
            .clone();

        tracing::info!(
            trace_id = %trace.trace_id,
            event_count = events.len(),
            invocation_count = invocations.len(),
            "tool service replaying sanitized audit memento"
        );
        command_result(
            trace,
            "ok",
            ToolResultClass::StructuredJson,
            Some(serde_json::json!({
                "plan": plan_payload,
                "events": events,
                "invocations": invocations,
            })),
            Vec::new(),
            None,
            None,
        )
    }

    /// Return a bounded explanation of the currently observed policy outcomes.
    ///
    /// `service.tool` does not make final policy semantics visible here. It
    /// exposes reason-code counts and audit refs already produced by planning
    /// and invocation decorators, allowing shells to explain "why hidden or
    /// denied" without duplicating policy engines in presentation code.
    pub fn policy_explain(&self, trace: TraceContext) -> ToolCommandResult {
        let audit = self.audit_query(trace.clone());
        let policy = audit.inline_output.as_ref().map(|payload| {
            serde_json::json!({
                "plan": payload.get("plan"),
                "invocation_count": payload
                    .get("invocations")
                    .and_then(|value| value.as_array())
                    .map(Vec::len)
                    .unwrap_or_default(),
            })
        });
        command_result(
            trace,
            "ok",
            ToolResultClass::StructuredJson,
            policy,
            Vec::new(),
            None,
            None,
        )
    }

    /// Record and return the provider health snapshot used by Web/CLI adapters.
    ///
    /// The provider count and health enum are sanitized status facts. Concrete
    /// provider configuration, credentials, and environment values never enter
    /// this event or result payload.
    pub fn provider_health(
        &self,
        trace: TraceContext,
        health: ServiceHealth,
        provider_count: usize,
    ) -> macaca_proto::ToolServiceHealthResult {
        let captured_at = chrono::Utc::now();
        self.push_event(serde_json::json!({
            "event": "tool.provider.health_changed",
            "trace_id": trace.trace_id,
            "provider_count": provider_count,
            "status": format!("{health:?}"),
            "captured_at": captured_at,
        }));
        macaca_proto::ToolServiceHealthResult {
            trace,
            health,
            provider_count,
            captured_at,
            metadata: BTreeMap::new(),
        }
    }

    fn record_plan_audit(&self, plan: &ToolCatalogPlanResult, provider_count: usize) {
        self.push_event(serde_json::json!({
            "event": "tool.planning.started",
            "trace_id": plan.trace.trace_id,
            "provider_count": provider_count,
            "captured_at": plan.captured_at,
        }));
        self.push_event(serde_json::json!({
            "event": "tool.planning.completed",
            "trace_id": plan.trace.trace_id,
            "visible_count": plan.visible.len(),
            "hidden_count": plan.hidden.len(),
            "conflict_count": plan.conflicts.len(),
            "captured_at": plan.captured_at,
        }));
        for entry in &plan.hidden {
            self.push_event(serde_json::json!({
                "event": "tool.hidden.diagnostic",
                "trace_id": plan.trace.trace_id,
                "tool_id": entry.descriptor.stable_tool_id,
                "reason_code": entry.hidden_reason.as_ref().map(reason_code),
                "captured_at": plan.captured_at,
            }));
        }
    }

    fn push_event(&self, event: serde_json::Value) {
        self.audit_events
            .write()
            .expect("tool audit event lock poisoned")
            .push(event);
    }
}

pub(crate) fn stable_json_hash(value: &serde_json::Value) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("hash:{:016x}", hasher.finish())
}

fn reason_code(reason: &macaca_proto::ToolHiddenReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".into())
}

fn invocation_event_name(status: &str) -> &'static str {
    match status {
        "ok" => "tool.invocation.completed",
        "cancelled" => "tool.invocation.cancelled",
        "approval_required" => "tool.invocation.progress",
        _ => "tool.invocation.failed",
    }
}

fn audit_ref_value(ref_value: &macaca_proto::ToolAuditRef) -> Option<String> {
    serde_json::to_value(ref_value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
}
