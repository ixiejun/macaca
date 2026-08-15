//! Deterministic mock configuration provider for contract and replay tests.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, ConfigProviderSnapshot, ConfigRedactionSummary, KernelServiceId,
    ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef, FOUNDATION_CONFIG_COMMANDS,
    FOUNDATION_CONFIG_SERVICE_ID,
};

use crate::service_contract::ConfigService;

/// In-memory deterministic Strategy that stores opaque value references only.
#[derive(Debug, Default)]
pub struct MockConfigProvider {
    values: Arc<Mutex<BTreeMap<String, String>>>,
}

#[async_trait]
impl ConfigService for MockConfigProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor()
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
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
            | "config.watch"
            | "config.reload"
            | "config.snapshot"
            | "config.export_redacted" => {
                serde_json::json!({"status":"success","provider_class":"mock","redacted":true})
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
            redaction_summary: ConfigRedactionSummary {
                redacted_value_count: 0,
                redacted_source_count: 0,
                contains_secret_references: false,
            },
        }
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        self.values.lock().map_err(lock_error)?.clear();
        tracing::info!(
            service_id = FOUNDATION_CONFIG_SERVICE_ID,
            "foundation config mock cache cleared"
        );
        Ok(())
    }
}

impl MockConfigProvider {
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
