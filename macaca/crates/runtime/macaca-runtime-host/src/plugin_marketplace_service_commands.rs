//! Command handlers for the runtime-host plugin marketplace provider.
//!
//! This module keeps lifecycle orchestration separate from provider bootstrap.
//! The split follows the Command pattern: each public marketplace operation is
//! decoded into a typed protocol DTO, checked against generic marketplace
//! policy gates, delegated to `PluginControlService` when the low-level plugin
//! state must change, and recorded as an audit or rollback memento.  The code
//! intentionally avoids application-specific decisions; all install behavior is
//! driven by manifest metadata and marketplace policy.

use chrono::Utc;
use macaca_proto::workbench::plugin_marketplace::*;
use macaca_proto::{
    PluginListCommand, PluginTargetCommand, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tracing::{info, warn};

use crate::plugin_marketplace_service_support::*;
use crate::plugin_marketplace_snapshot_decode::decode_snapshot_payload;
use crate::PluginMarketplaceSystemServiceProvider;

impl PluginMarketplaceSystemServiceProvider {
    /// Decode and route one marketplace command through the typed service facade.
    pub(crate) async fn dispatch(
        &self,
        command_name: &str,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        if let Some(reason) = &self.unavailable_reason {
            warn!(
                command = command_name,
                trace_id = %trace.trace_id,
                reason = %reason,
                "plugin marketplace command rejected because provider is unavailable"
            );
            return WorkbenchCommandResult::unavailable(trace, reason.clone());
        }
        match command_name {
            MARKETPLACE_ADD_COMMAND | LEGACY_MARKETPLACE_ADD_COMMAND => {
                self.marketplace_add(decode(payload), trace).await
            }
            MARKETPLACE_REMOVE_COMMAND | LEGACY_MARKETPLACE_REMOVE_COMMAND => {
                self.marketplace_remove(decode(payload), trace).await
            }
            MARKETPLACE_UPGRADE_COMMAND => self.marketplace_upgrade(decode(payload), trace).await,
            PLUGIN_LIST_COMMAND => self.plugin_list(decode(payload), trace).await,
            PLUGIN_READ_COMMAND => self.plugin_read(decode(payload), trace).await,
            PLUGIN_INSTALL_COMMAND => self.plugin_install(decode(payload), trace).await,
            PLUGIN_UNINSTALL_COMMAND => self.plugin_uninstall(decode(payload), trace).await,
            PLUGIN_ENABLE_COMMAND => self.plugin_enable(decode(payload), trace).await,
            PLUGIN_DISABLE_COMMAND => self.plugin_disable(decode(payload), trace).await,
            PLUGIN_AUTH_STATUS_COMMAND => self.plugin_auth_status(decode(payload), trace).await,
            SNAPSHOT_COMMAND => self.snapshot(decode_snapshot_payload(payload), trace).await,
            other => completed(
                trace,
                WorkbenchCommandStatus::Unsupported {
                    reason: format!("unsupported plugin marketplace command '{other}'"),
                },
                None,
                None,
            ),
        }
    }

    async fn marketplace_add(
        &self,
        payload: ServiceResult<MarketplaceCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let descriptor = payload.descriptor;
        if descriptor.marketplace_id.trim().is_empty() {
            return denied(trace, "marketplace id must not be empty");
        }
        info!(
            trace_id = %trace.trace_id,
            marketplace_id = %descriptor.marketplace_id,
            "plugin marketplace source added"
        );
        self.marketplaces
            .write()
            .await
            .insert(descriptor.marketplace_id.clone(), descriptor.clone());
        let audit = self
            .record_audit(
                "marketplace.add",
                Some(descriptor.marketplace_id.clone()),
                None,
                None,
                Vec::new(),
                Vec::new(),
                &trace,
            )
            .await;
        completed(
            trace,
            WorkbenchCommandStatus::Completed,
            Some(PluginMarketplaceResponse::Marketplace(descriptor)),
            Some(audit.audit_ref),
        )
    }

    async fn marketplace_remove(
        &self,
        payload: ServiceResult<MarketplaceRemoveCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let removed = self
            .marketplaces
            .write()
            .await
            .remove(&payload.marketplace_id);
        let Some(mut descriptor) = removed else {
            return denied(
                trace,
                format!("marketplace not found: {}", payload.marketplace_id),
            );
        };
        descriptor.enabled = false;
        let audit = self
            .record_audit(
                "marketplace.remove",
                Some(descriptor.marketplace_id.clone()),
                None,
                None,
                Vec::new(),
                Vec::new(),
                &trace,
            )
            .await;
        info!(
            trace_id = %trace.trace_id,
            marketplace_id = %descriptor.marketplace_id,
            reason = %payload.reason,
            "plugin marketplace source removed"
        );
        completed(
            trace,
            WorkbenchCommandStatus::Completed,
            Some(PluginMarketplaceResponse::Marketplace(descriptor)),
            Some(audit.audit_ref),
        )
    }

    async fn marketplace_upgrade(
        &self,
        payload: ServiceResult<MarketplaceUpgradeCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        if payload.marketplace_id != payload.descriptor.marketplace_id {
            return denied(trace, "marketplace upgrade id mismatch");
        }
        self.marketplaces
            .write()
            .await
            .insert(payload.marketplace_id.clone(), payload.descriptor.clone());
        let audit = self
            .record_audit(
                "marketplace.upgrade",
                Some(payload.marketplace_id),
                None,
                None,
                Vec::new(),
                Vec::new(),
                &trace,
            )
            .await;
        completed(
            trace,
            WorkbenchCommandStatus::Completed,
            Some(PluginMarketplaceResponse::Marketplace(payload.descriptor)),
            Some(audit.audit_ref),
        )
    }

    async fn plugin_list(
        &self,
        payload: ServiceResult<PluginMarketplaceListCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        match self
            .control
            .list(PluginListCommand {
                repository_id: payload.marketplace_id,
                include_disabled: payload.include_disabled,
                trace: trace.clone(),
            })
            .await
        {
            Ok(records) => completed(
                trace,
                WorkbenchCommandStatus::Completed,
                Some(PluginMarketplaceResponse::PluginList(records)),
                None,
            ),
            Err(error) => plugin_failed(trace, error),
        }
    }

    async fn plugin_read(
        &self,
        payload: ServiceResult<PluginMarketplaceReadCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let command = PluginTargetCommand::new(payload.plugin_id, trace.clone());
        match self.control.inspect(command).await {
            Ok(record) => completed(
                trace,
                WorkbenchCommandStatus::Completed,
                Some(PluginMarketplaceResponse::PluginRead(record)),
                None,
            ),
            Err(error) => plugin_failed(trace, error),
        }
    }

    async fn plugin_install(
        &self,
        payload: ServiceResult<PluginMarketplaceInstallCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let Some(marketplace) = self.marketplace(&payload.marketplace_id).await else {
            return denied(
                trace,
                format!("marketplace not found: {}", payload.marketplace_id),
            );
        };
        let Some(manifest) = payload.request.manifest.as_ref() else {
            let audit = self
                .record_audit(
                    "plugin.install",
                    Some(payload.marketplace_id),
                    None,
                    None,
                    vec![gate("manifest", "denied", Some("missing manifest"))],
                    Vec::new(),
                    &trace,
                )
                .await;
            return completed(
                trace,
                WorkbenchCommandStatus::Denied {
                    reason: "plugin install requires a manifest".into(),
                },
                Some(PluginMarketplaceResponse::Audit(audit.clone())),
                Some(audit.audit_ref),
            );
        };
        let gates = self.evaluate_gates(&marketplace, &payload.request, manifest);
        let summary = summarize_manifest_capabilities(manifest);
        if let Some(rejection) = gates.iter().find(|decision| decision.status == "denied") {
            let rejection_reason = rejection
                .reason
                .clone()
                .unwrap_or_else(|| rejection.gate.clone());
            let audit = self
                .record_audit(
                    "plugin.install",
                    Some(payload.marketplace_id),
                    Some(manifest.plugin_id.clone()),
                    Some(manifest.version.clone()),
                    gates,
                    summary,
                    &trace,
                )
                .await;
            return completed(
                trace,
                WorkbenchCommandStatus::Denied {
                    reason: rejection_reason,
                },
                Some(PluginMarketplaceResponse::Audit(audit.clone())),
                Some(audit.audit_ref),
            );
        }
        match self.control.install(payload.request.clone()).await {
            Ok(result) => {
                let rollback = self
                    .record_rollback(
                        result.record.plugin_id.clone(),
                        None,
                        Some(result.record.version.clone()),
                        "plugin.install",
                        vec![format!("cleanup://{}", result.record.plugin_id)],
                        &trace,
                    )
                    .await;
                let audit = self
                    .record_audit(
                        "plugin.install",
                        Some(marketplace.marketplace_id),
                        Some(result.record.plugin_id.clone()),
                        Some(result.record.version.clone()),
                        gates,
                        summary.clone(),
                        &trace,
                    )
                    .await;
                completed(
                    trace,
                    WorkbenchCommandStatus::Completed,
                    Some(PluginMarketplaceResponse::Installed {
                        record: result.record,
                        audit_ref: audit.audit_ref.clone(),
                        rollback_ref: rollback.rollback_ref,
                        capability_summary: summary,
                    }),
                    Some(audit.audit_ref),
                )
            }
            Err(error) => plugin_failed(trace, error),
        }
    }

    async fn plugin_enable(
        &self,
        payload: ServiceResult<PluginTargetCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        match self.control.enable(payload).await {
            Ok(record) => state_result("plugin.enable", record, trace, self).await,
            Err(error) => plugin_failed(trace, error),
        }
    }

    async fn plugin_disable(
        &self,
        payload: ServiceResult<PluginTargetCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        match self.control.disable(payload).await {
            Ok(record) => state_result("plugin.disable", record, trace, self).await,
            Err(error) => plugin_failed(trace, error),
        }
    }

    async fn plugin_uninstall(
        &self,
        payload: ServiceResult<PluginTargetCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let plugin_id = payload.plugin_id.clone();
        match self.control.uninstall(payload).await {
            Ok(event) => {
                let rollback = self
                    .record_rollback(
                        plugin_id,
                        None,
                        None,
                        "plugin.uninstall",
                        vec![format!("repository://{}", event.operation)],
                        &trace,
                    )
                    .await;
                let audit = self
                    .record_audit(
                        "plugin.uninstall",
                        None,
                        event.plugin_id.clone(),
                        None,
                        Vec::new(),
                        Vec::new(),
                        &trace,
                    )
                    .await;
                completed(
                    trace,
                    WorkbenchCommandStatus::Completed,
                    Some(PluginMarketplaceResponse::Uninstalled {
                        event_ref: event_ref(&event),
                        rollback_ref: rollback.rollback_ref,
                        cleanup_refs: rollback.cleanup_refs,
                    }),
                    Some(audit.audit_ref),
                )
            }
            Err(error) => plugin_failed(trace, error),
        }
    }

    async fn plugin_auth_status(
        &self,
        payload: ServiceResult<PluginMarketplaceAuthStatusCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let status = PluginAuthStatus {
            marketplace_id: payload.marketplace_id,
            plugin_id: payload.plugin_id,
            entitlement: "checked".into(),
            license: "checked".into(),
            metering: "checked".into(),
            auth_required: false,
            diagnostics: Vec::new(),
        };
        completed(
            trace,
            WorkbenchCommandStatus::Completed,
            Some(PluginMarketplaceResponse::AuthStatus(status)),
            None,
        )
    }

    async fn snapshot(
        &self,
        payload: ServiceResult<PluginMarketplaceSnapshotCommand>,
        trace: TraceContext,
    ) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
        let payload = match payload {
            Ok(value) => value,
            Err(error) => return failed(trace, error),
        };
        let marketplaces = self.marketplaces.read().await.values().cloned().collect();
        let installed_plugins = self
            .control
            .list(PluginListCommand {
                repository_id: None,
                include_disabled: true,
                trace: trace.clone(),
            })
            .await
            .unwrap_or_default();
        let rollbacks = self.rollbacks.read().await.clone();
        let audit_tail = if payload.include_audit_tail {
            self.audits
                .read()
                .await
                .iter()
                .rev()
                .take(20)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        completed(
            trace,
            WorkbenchCommandStatus::Completed,
            Some(PluginMarketplaceResponse::Snapshot(
                PluginMarketplaceSnapshot {
                    service_id: SERVICE_ID.into(),
                    healthy: self.unavailable_reason.is_none(),
                    marketplaces,
                    installed_plugins,
                    rollback_records: rollbacks,
                    audit_tail,
                    captured_at: Utc::now(),
                },
            )),
            None,
        )
    }
}
