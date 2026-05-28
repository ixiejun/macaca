//! Command handlers for `service.config`.
//!
//! The handlers keep configuration semantics inside the service layer.  Reads
//! use a Builder-style effective-config resolution over ordered layers.  Writes
//! validate all keys before mutation so batch writes are atomic.  Requirements
//! are returned as executable Specifications for policy decorators, while
//! events and audit records expose only bounded, redacted metadata.

use std::collections::{BTreeMap, HashMap};

use macaca_proto::workbench::config::*;
use macaca_proto::workbench::sandbox::{
    SandboxNetworkMode, SandboxPermissionProfile, FULL_ACCESS_PROFILE, NETWORK_ALLOWED_PROFILE,
    READ_ONLY_PROFILE, REMOTE_ENVIRONMENT_PROFILE, WORKSPACE_WRITE_PROFILE,
};
use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use serde_json::Value;
use tracing::info;

use crate::config_service_provider::{
    audit_ref, event_ref, ConfigSystemServiceProvider, ConfigValueRecord, SECRET_KEY_FRAGMENTS,
    SECRET_REDACTION,
};

impl ConfigSystemServiceProvider {
    pub(crate) async fn read(
        &self,
        request: ConfigReadRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let effective = self
            .effective_config(request.scope, request.key_prefix)
            .await;
        info!(
            trace_id = %trace.trace_id,
            value_count = effective.values.len(),
            "config service resolved effective layered config"
        );
        Self::result(
            ConfigServiceResponse::Read(effective),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn value_write(
        &self,
        request: ConfigValueWrite,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        self.validate_write(&request)?;
        let record = self.apply_write(request.clone()).await;
        let audit = self
            .persist_audit(
                "config.value.write",
                vec![record.key_path.clone()],
                Some(request.writer_ref),
                request.reload,
                &trace,
            )
            .await;
        if request.reload {
            self.emit_change(vec![record.key_path.clone()], true, &trace)
                .await;
        }
        Self::result(
            ConfigServiceResponse::Written {
                values: vec![self.view(&record)],
                audit_ref: audit.audit_ref,
                reload_emitted: request.reload,
            },
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn batch_write(
        &self,
        request: ConfigBatchWrite,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        if request.writes.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "config batch write requires at least one value".into(),
            ));
        }
        for write in &request.writes {
            self.validate_write(write)?;
        }
        let mut records = Vec::new();
        let mut key_paths = Vec::new();
        let mut writer_ref = None;
        for write in request.writes {
            writer_ref.get_or_insert_with(|| write.writer_ref.clone());
            key_paths.push(write.key_path.clone());
            records.push(self.apply_write(write).await);
        }
        let reload = request.reload
            || records.iter().any(|record| {
                record.key_path.starts_with("runtime.") || record.key_path.starts_with("feature.")
            });
        let audit = self
            .persist_audit(
                "config.batch.write",
                key_paths.clone(),
                writer_ref,
                reload,
                &trace,
            )
            .await;
        if reload {
            self.emit_change(key_paths, true, &trace).await;
        }
        Self::result(
            ConfigServiceResponse::Written {
                values: records.iter().map(|record| self.view(record)).collect(),
                audit_ref: audit.audit_ref,
                reload_emitted: reload,
            },
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn schema_read(
        &self,
        _request: ConfigSchemaRead,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Self::result(
            ConfigServiceResponse::Schema(ConfigSchema {
                supported_layers: layer_order(),
                supported_commands: COMMANDS.iter().map(|command| (*command).into()).collect(),
                secret_key_fragments: SECRET_KEY_FRAGMENTS
                    .iter()
                    .map(|fragment| (*fragment).into())
                    .collect(),
                reload_supported: true,
            }),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn reload(
        &self,
        request: ConfigReloadRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let key_paths = self
            .effective_config(request.scope, None)
            .await
            .values
            .keys()
            .cloned()
            .collect();
        let event = self.emit_change(key_paths, true, &trace).await;
        Self::result(
            ConfigServiceResponse::Reloaded(event),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn requirements_read(
        &self,
        _request: ConfigRequirementsRead,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Self::result(
            ConfigServiceResponse::Requirements(default_requirements()),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn permission_profile_list(
        &self,
        _request: PermissionProfileListRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let allowed = default_requirements().allowed_sandbox_profiles;
        let profiles = SandboxPermissionProfile::catalog()
            .into_iter()
            .filter(|profile| allowed.iter().any(|item| item == &profile.profile_ref))
            .collect();
        Self::result(
            ConfigServiceResponse::PermissionProfiles(profiles),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn feature_list(
        &self,
        request: FeatureListRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let mut features = builtin_features();
        features.extend(self.features.read().await.values().cloned());
        let mut deduped = BTreeMap::new();
        for feature in features {
            if request.include_disabled || feature.enabled {
                deduped.insert(feature.feature_key.clone(), feature);
            }
        }
        Self::result(
            ConfigServiceResponse::Features(deduped.into_values().collect()),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn feature_enablement_set(
        &self,
        request: FeatureEnablementSet,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        validate_key_path(&request.feature_key)?;
        let record = FeatureFlagRecord {
            feature_key: request.feature_key.clone(),
            enabled: request.enabled,
            layer: request.layer,
            source_class: ConfigSourceClass::Managed,
            updated_at: chrono::Utc::now(),
        };
        self.features
            .write()
            .await
            .insert(record.feature_key.clone(), record.clone());
        let audit = self
            .persist_audit(
                "feature.enablement.set",
                vec![record.feature_key.clone()],
                Some(request.writer_ref),
                request.reload,
                &trace,
            )
            .await;
        if request.reload {
            self.emit_change(vec![record.feature_key.clone()], true, &trace)
                .await;
        }
        Self::result(
            ConfigServiceResponse::Written {
                values: Vec::new(),
                audit_ref: audit.audit_ref,
                reload_emitted: request.reload,
            },
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn snapshot(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        let count = self.values.read().await.len();
        Self::result(
            ConfigServiceResponse::Features(Vec::new()),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            Some(macaca_proto::BoundedSummary {
                text: format!("config values tracked: {count}"),
                truncated: false,
                original_byte_len: None,
                artifact_ref: None,
            }),
        )
    }

    async fn effective_config(
        &self,
        scope: ConfigScope,
        key_prefix: Option<String>,
    ) -> EffectiveConfig {
        let records = self.values.read().await;
        let mut by_key: HashMap<String, Vec<&ConfigValueRecord>> = HashMap::new();
        for record in records.values() {
            if key_prefix
                .as_ref()
                .map_or(true, |prefix| record.key_path.starts_with(prefix))
            {
                by_key
                    .entry(record.key_path.clone())
                    .or_default()
                    .push(record);
            }
        }
        let mut values = BTreeMap::new();
        for (key, mut candidates) in by_key {
            candidates.sort_by_key(|record| layer_rank(&record.layer));
            if let Some(record) = candidates.last() {
                values.insert(key, self.view(record));
            }
        }
        EffectiveConfig {
            scope,
            values,
            layer_order: layer_order(),
            generated_at: chrono::Utc::now(),
        }
    }

    fn validate_write(&self, write: &ConfigValueWrite) -> ServiceResult<()> {
        validate_key_path(&write.key_path)?;
        if write.writer_ref.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "config write requires non-empty writer_ref".into(),
            ));
        }
        Ok(())
    }

    async fn apply_write(&self, write: ConfigValueWrite) -> ConfigValueRecord {
        let redacted = is_secret_key(&write.key_path);
        let key = (write.layer.clone(), write.key_path.clone());
        let mut values = self.values.write().await;
        let version = values
            .get(&key)
            .map(|record| record.version + 1)
            .unwrap_or(1);
        let record = ConfigValueRecord {
            key_path: write.key_path,
            layer: write.layer,
            value: write.value,
            redacted,
            version,
            writer_ref: write.writer_ref,
            updated_at: chrono::Utc::now(),
        };
        values.insert(key, record.clone());
        info!(
            key_path = %record.key_path,
            layer = ?record.layer,
            version = record.version,
            redacted = record.redacted,
            writer_ref = %record.writer_ref,
            "config value written to service-owned layer"
        );
        record
    }

    fn view(&self, record: &ConfigValueRecord) -> ConfigValueView {
        ConfigValueView {
            key_path: record.key_path.clone(),
            layer: record.layer.clone(),
            value: if record.redacted {
                Value::String(SECRET_REDACTION.into())
            } else {
                record.value.clone()
            },
            redacted: record.redacted,
            version: record.version,
            updated_at: record.updated_at,
        }
    }

    async fn persist_audit(
        &self,
        operation: &str,
        key_paths: Vec<String>,
        writer_ref: Option<String>,
        reload: bool,
        trace: &TraceContext,
    ) -> ConfigAuditRecord {
        let audit = ConfigAuditRecord {
            audit_ref: audit_ref(),
            operation: operation.into(),
            key_paths,
            writer_ref,
            reload,
            trace_id: trace.trace_id.clone(),
            recorded_at: chrono::Utc::now(),
        };
        self.audits
            .write()
            .await
            .insert(audit.audit_ref.clone(), audit.clone());
        audit
    }

    async fn emit_change(
        &self,
        key_paths: Vec<String>,
        reload: bool,
        trace: &TraceContext,
    ) -> ConfigChangedEvent {
        let event = ConfigChangedEvent {
            event_ref: event_ref(),
            key_paths,
            reload,
            trace_id: trace.trace_id.clone(),
            emitted_at: chrono::Utc::now(),
        };
        let _ = self.notify_tx.send(event.clone());
        event
    }
}

fn validate_key_path(key_path: &str) -> ServiceResult<()> {
    let trimmed = key_path.trim();
    if trimmed.is_empty()
        || trimmed.contains(' ')
        || trimmed.contains('\n')
        || trimmed.contains('\t')
        || trimmed.starts_with('.')
        || trimmed.ends_with('.')
    {
        return Err(ServiceError::InvalidArgument(
            "config key_path must be non-empty dot-separated identifier".into(),
        ));
    }
    Ok(())
}

fn is_secret_key(key_path: &str) -> bool {
    let lower = key_path.to_ascii_lowercase();
    SECRET_KEY_FRAGMENTS
        .iter()
        .any(|fragment| lower.contains(fragment))
}

fn layer_rank(layer: &ConfigLayer) -> usize {
    match layer {
        ConfigLayer::Default => 0,
        ConfigLayer::User => 1,
        ConfigLayer::Project => 2,
        ConfigLayer::Application => 3,
        ConfigLayer::Session => 4,
        ConfigLayer::Requirements => 5,
        ConfigLayer::Managed => 6,
    }
}

fn layer_order() -> Vec<ConfigLayer> {
    vec![
        ConfigLayer::Default,
        ConfigLayer::User,
        ConfigLayer::Project,
        ConfigLayer::Application,
        ConfigLayer::Session,
        ConfigLayer::Requirements,
        ConfigLayer::Managed,
    ]
}

fn default_requirements() -> ConfigRequirements {
    ConfigRequirements {
        source_class: ConfigSourceClass::Managed,
        allowed_approval_profiles: vec![
            "approval.manual.guardian".into(),
            "approval.managed".into(),
        ],
        allowed_sandbox_profiles: vec![
            READ_ONLY_PROFILE.into(),
            WORKSPACE_WRITE_PROFILE.into(),
            FULL_ACCESS_PROFILE.into(),
            NETWORK_ALLOWED_PROFILE.into(),
            REMOTE_ENVIRONMENT_PROFILE.into(),
        ],
        allowed_network_modes: vec![
            SandboxNetworkMode::Denied,
            SandboxNetworkMode::Restricted,
            SandboxNetworkMode::Allowed,
            SandboxNetworkMode::RemoteOnly,
        ],
        allowed_permissions: vec![
            "file.read".into(),
            "file.write".into(),
            "process.spawn".into(),
            "network.request".into(),
            "hook.managed".into(),
        ],
        managed_hooks_only: true,
        residency: None,
        feature_requirements: BTreeMap::new(),
        network_constraints: vec!["policy.network.explicit_profile_required".into()],
    }
}

fn builtin_features() -> Vec<FeatureFlagRecord> {
    vec![FeatureFlagRecord {
        feature_key: "service.config.hot_reload".into(),
        enabled: true,
        layer: ConfigLayer::Default,
        source_class: ConfigSourceClass::BuiltIn,
        updated_at: chrono::Utc::now(),
    }]
}
