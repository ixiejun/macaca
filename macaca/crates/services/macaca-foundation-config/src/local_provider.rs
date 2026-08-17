//! Deterministic mock configuration provider for contract and replay tests.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, ConfigProviderCapability, ConfigProviderSnapshot,
    ConfigRedactionSummary, ConfigValueKind, DomainPackProviderCapabilityState, KernelServiceId,
    ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef, FOUNDATION_CONFIG_COMMANDS,
    FOUNDATION_CONFIG_SERVICE_ID,
};

use crate::service_contract::ConfigService;

/// In-memory deterministic Strategy that stores opaque value references only.
#[derive(Debug, Default)]
pub struct MockConfigProvider {
    values: Arc<Mutex<BTreeMap<String, String>>>,
    watches: Arc<Mutex<BTreeSet<String>>>,
    call_count: AtomicUsize,
}

#[async_trait]
impl ConfigService for MockConfigProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor()
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        let operation = command.name.as_str();
        let output = match operation {
            "config.get" | "config.resolve_effective" | "config.explain_provenance" => {
                let key = key_from_payload(&command.payload)?;
                let value_ref = self.values.lock().map_err(lock_error)?.get(&key).cloned();
                serde_json::json!({"status": if value_ref.is_some() { "success" } else { "not_found" }, "key": hash(&key), "value_ref": value_ref})
            }
            "config.get_many"
            | "config.list_keys"
            | "config.describe_schema"
            | "config.validate"
            | "config.reload"
            | "config.snapshot"
            | "config.export_redacted" => {
                serde_json::json!({"status":"success","provider_class":"mock","redacted":true})
            }
            "config.watch" => {
                let checkpoint = hash(trace.trace_id.as_str());
                self.watches
                    .lock()
                    .map_err(lock_error)?
                    .insert(checkpoint.clone());
                serde_json::json!({"status":"watch_checkpoint","checkpoint":checkpoint,"redacted":true})
            }
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        tracing::info!(service_id = FOUNDATION_CONFIG_SERVICE_ID, command = operation,
            trace_id = %trace.trace_id, "foundation config mock command completed");
        Ok(ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::from([
                ("replay.provider_class".into(), "mock".into()),
                ("replay.config_command".into(), operation.into()),
                ("config.audit_event".into(), audit_event(operation).into()),
                ("config.redaction".into(), "opaque_references_only".into()),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> ConfigProviderSnapshot {
        let source_hashes = self
            .values
            .lock()
            .map(|values| {
                values
                    .keys()
                    .map(|key| (format!("key:{}", hash(key)), "mock-source-v1".into()))
                    .collect()
            })
            .unwrap_or_default();
        ConfigProviderSnapshot {
            descriptor_hash: "foundation-config-mock-v1".into(),
            provider_class: "mock".into(),
            source_hashes,
            schema_hashes: Default::default(),
            layer_order: vec!["mock".into()],
            validation_status: "valid".into(),
            replay_ref: "replay:foundation-config:mock".into(),
            redaction_summary: ConfigRedactionSummary {
                redacted_value_count: 0,
                redacted_source_count: 0,
                contains_secret_references: false,
            },
        }
    }
    fn provider_capabilities(&self) -> ConfigProviderCapability {
        ConfigProviderCapability {
            provider_class: "mock".into(),
            supported_commands: FOUNDATION_CONFIG_COMMANDS
                .iter()
                .map(|command| (*command).into())
                .collect(),
            supported_value_kinds: BTreeSet::from([
                ConfigValueKind::String,
                ConfigValueKind::Number,
                ConfigValueKind::Boolean,
                ConfigValueKind::Json,
                ConfigValueKind::SecretReference,
            ]),
            supports_watch: true,
            supports_reload: true,
            supports_redacted_export: true,
            max_keys_per_page: 1_000,
            max_value_bytes: 65_536,
            availability: DomainPackProviderCapabilityState::Available,
        }
    }
    async fn cancel_watch(&self, watch_checkpoint: &str) -> ServiceResult<()> {
        if watch_checkpoint.is_empty() || watch_checkpoint.len() > 128 {
            return Err(ServiceError::InvalidArgument(
                "bounded watch checkpoint required".into(),
            ));
        }
        self.watches
            .lock()
            .map_err(lock_error)?
            .remove(watch_checkpoint);
        tracing::info!(
            service_id = FOUNDATION_CONFIG_SERVICE_ID,
            watch_checkpoint_hash = hash(watch_checkpoint),
            "foundation config mock watch cancelled"
        );
        Ok(())
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        self.values.lock().map_err(lock_error)?.clear();
        self.watches.lock().map_err(lock_error)?.clear();
        tracing::info!(
            service_id = FOUNDATION_CONFIG_SERVICE_ID,
            "foundation config mock cache cleared"
        );
        Ok(())
    }
}

impl MockConfigProvider {
    /// Return invocation evidence used only by contract tests to prove admission preflight.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }
    /// Seed an opaque artifact or secret reference without accepting raw configuration values.
    pub fn insert_reference(
        &self,
        key: impl Into<String>,
        value_ref: impl Into<String>,
    ) -> ServiceResult<()> {
        let value_ref = value_ref.into();
        if !value_ref.starts_with("artifact:") && !value_ref.starts_with("secret:") {
            return Err(ServiceError::InvalidArgument(
                "config values must be opaque references".into(),
            ));
        }
        self.values
            .lock()
            .map_err(lock_error)?
            .insert(key.into(), value_ref);
        Ok(())
    }
}

fn descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(FOUNDATION_CONFIG_SERVICE_ID),
        ServiceType::new("foundation.config"),
        TraceSchemaRef::new("macaca.trace.foundation.config.v1"),
    );
    descriptor.health = ServiceHealth::Healthy;
    descriptor.capabilities = FOUNDATION_CONFIG_COMMANDS
        .iter()
        .map(|name| ServiceCapability::new(CapabilityId::new(*name), "config command"))
        .collect();
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor
}

/// Map declared commands to sanitized observer events without provider-specific data.
fn audit_event(operation: &str) -> &'static str {
    match operation {
        "config.watch" => "config_pack_watch_started",
        "config.reload" => "config_pack_source_reloaded",
        "config.snapshot" => "config_pack_snapshot_recorded",
        "config.validate" => "config_pack_validation_succeeded",
        _ => "config_pack_service_call_succeeded",
    }
}

fn key_from_payload(payload: &serde_json::Value) -> ServiceResult<String> {
    payload
        .get("key")
        .and_then(|key| key.get("key"))
        .and_then(serde_json::Value::as_str)
        .filter(|key| !key.is_empty() && key.len() <= 256)
        .map(str::to_owned)
        .ok_or_else(|| ServiceError::InvalidArgument("bounded config key required".into()))
}

fn hash(value: &str) -> String {
    format!(
        "{:016x}",
        value.bytes().fold(0_u64, |state, byte| state
            .wrapping_mul(1099511628211)
            .wrapping_add(byte as u64))
    )
}
fn lock_error<T>(_: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::AdapterFailure("config provider lock poisoned".into())
}
