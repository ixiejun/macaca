//! Shared support routines for the plugin marketplace service.
//!
//! The helpers in this module implement the Strategy and Memento pieces used by
//! the command handlers.  Gate evaluation is a generic policy strategy derived
//! from marketplace descriptors, while audit and rollback records are mementos
//! that let operators inspect lifecycle decisions without exposing raw package
//! data, secrets, credentials, or application-specific state.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_proto::workbench::plugin_marketplace::*;
use macaca_proto::{
    PluginControlEvent, PluginError, ServiceError, ServiceResult, TraceContext,
    WorkbenchCommandResult, WorkbenchCommandStatus,
};
use uuid::Uuid;

use crate::PluginMarketplaceSystemServiceProvider;

impl PluginMarketplaceSystemServiceProvider {
    /// Read a marketplace descriptor by id without leaking the internal map.
    pub(crate) async fn marketplace(&self, marketplace_id: &str) -> Option<MarketplaceDescriptor> {
        self.marketplaces.read().await.get(marketplace_id).cloned()
    }

    /// Evaluate all install gates before the control plane mutates plugin state.
    pub(crate) fn evaluate_gates(
        &self,
        marketplace: &MarketplaceDescriptor,
        request: &macaca_proto::PluginInstallRequest,
        manifest: &macaca_proto::PluginManifest,
    ) -> Vec<MarketplaceGateDecision> {
        let mut decisions = Vec::new();
        decisions.push(
            if marketplace.enabled && !marketplace.source_policy.disabled_by_admin {
                gate("marketplace", "allowed", None)
            } else {
                gate(
                    "marketplace",
                    "denied",
                    Some("marketplace disabled by policy"),
                )
            },
        );
        decisions.push(
            if marketplace
                .source_policy
                .allowed_source_kinds
                .contains(&request.location.source_kind)
            {
                gate("source_policy", "allowed", None)
            } else {
                gate(
                    "source_policy",
                    "denied",
                    Some("install source is not allowed"),
                )
            },
        );
        decisions.push(signature_gate(
            manifest,
            marketplace.source_policy.require_signature,
        ));
        decisions.push(metadata_gate(
            "entitlement",
            marketplace.source_policy.require_entitlement,
            &request.metadata,
        ));
        decisions.push(metadata_gate(
            "license",
            marketplace.source_policy.require_license,
            &request.metadata,
        ));
        decisions.push(metadata_gate(
            "metering",
            marketplace.source_policy.require_metering,
            &request.metadata,
        ));
        decisions
    }

    /// Store a sanitized lifecycle audit record for replay and diagnostics.
    pub(crate) async fn record_audit(
        &self,
        operation: &str,
        marketplace_id: Option<String>,
        plugin_id: Option<macaca_proto::PluginId>,
        version: Option<macaca_proto::PluginVersion>,
        gate_decisions: Vec<MarketplaceGateDecision>,
        capability_summary: Vec<PluginBundleCapabilitySummary>,
        trace: &TraceContext,
    ) -> PluginMarketplaceAuditRecord {
        let audit = PluginMarketplaceAuditRecord {
            audit_ref: format!("plugin-marketplace-audit-{}", Uuid::new_v4()),
            operation: operation.into(),
            marketplace_id,
            plugin_id,
            version,
            gate_decisions,
            capability_summary,
            trace_id: trace.trace_id.clone(),
            recorded_at: Utc::now(),
        };
        self.audits.write().await.push(audit.clone());
        audit
    }

    /// Store a rollback memento that references cleanup actions without running them.
    pub(crate) async fn record_rollback(
        &self,
        plugin_id: macaca_proto::PluginId,
        previous_version: Option<macaca_proto::PluginVersion>,
        installed_version: Option<macaca_proto::PluginVersion>,
        operation: &str,
        cleanup_refs: Vec<String>,
        trace: &TraceContext,
    ) -> PluginRollbackRecord {
        let rollback = PluginRollbackRecord {
            rollback_ref: format!("plugin-rollback-{}", Uuid::new_v4()),
            plugin_id,
            previous_version,
            installed_version,
            operation: operation.into(),
            cleanup_refs,
            trace_id: trace.trace_id.clone(),
            recorded_at: Utc::now(),
        };
        self.rollbacks.write().await.push(rollback.clone());
        rollback
    }
}

pub(crate) fn decode<T>(value: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
}

pub(crate) fn completed(
    trace: TraceContext,
    status: WorkbenchCommandStatus,
    output: Option<PluginMarketplaceResponse>,
    audit_ref: Option<String>,
) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
    WorkbenchCommandResult {
        trace,
        status,
        output,
        summary: None,
        audit_ref,
        completed_at: Utc::now(),
    }
}

pub(crate) fn failed(
    trace: TraceContext,
    error: ServiceError,
) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
    completed(
        trace,
        WorkbenchCommandStatus::Failed {
            reason: error.to_string(),
        },
        None,
        None,
    )
}

pub(crate) fn denied(
    trace: TraceContext,
    reason: impl Into<String>,
) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
    completed(
        trace,
        WorkbenchCommandStatus::Denied {
            reason: reason.into(),
        },
        None,
        None,
    )
}

pub(crate) fn plugin_failed(
    trace: TraceContext,
    error: PluginError,
) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
    completed(
        trace,
        WorkbenchCommandStatus::Failed {
            reason: error.to_string(),
        },
        None,
        None,
    )
}

pub(crate) async fn state_result(
    operation: &str,
    record: macaca_proto::PluginControlRecord,
    trace: TraceContext,
    provider: &PluginMarketplaceSystemServiceProvider,
) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
    let audit = provider
        .record_audit(
            operation,
            None,
            Some(record.plugin_id.clone()),
            Some(record.version.clone()),
            Vec::new(),
            summarize_manifest_capabilities(&record.manifest),
            &trace,
        )
        .await;
    completed(
        trace,
        WorkbenchCommandStatus::Completed,
        Some(PluginMarketplaceResponse::PluginState(record)),
        Some(audit.audit_ref),
    )
}

pub(crate) fn gate(
    gate: impl Into<String>,
    status: impl Into<String>,
    reason: Option<&str>,
) -> MarketplaceGateDecision {
    MarketplaceGateDecision {
        gate: gate.into(),
        status: status.into(),
        reason: reason.map(str::to_string),
    }
}

fn signature_gate(
    manifest: &macaca_proto::PluginManifest,
    required: bool,
) -> MarketplaceGateDecision {
    if !required {
        return gate("signature", "skipped", None);
    }
    let Some(signature) = &manifest.signature else {
        return gate(
            "signature",
            "denied",
            Some("missing plugin signature metadata"),
        );
    };
    if matches!(
        signature.verification_status.as_str(),
        "verified" | "trusted"
    ) {
        gate("signature", "allowed", None)
    } else {
        gate(
            "signature",
            "denied",
            Some("signature verification is not trusted"),
        )
    }
}

fn metadata_gate(
    gate_name: &'static str,
    required: bool,
    metadata: &BTreeMap<String, String>,
) -> MarketplaceGateDecision {
    if !required {
        return gate(gate_name, "skipped", None);
    }
    let metadata_key = format!("{gate_name}.status");
    let status = metadata
        .get(metadata_key.as_str())
        .map(String::as_str)
        .unwrap_or("allowed");
    if matches!(status, "denied" | "disabled" | "expired") {
        gate(gate_name, "denied", Some("gate policy denied activation"))
    } else {
        gate(gate_name, "allowed", None)
    }
}

pub(crate) fn event_ref(event: &PluginControlEvent) -> String {
    format!(
        "plugin-event:{}:{}",
        event.operation,
        event
            .plugin_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".into())
    )
}
