//! Deterministic key-value Strategy for contract and replay tests.
//!
//! This provider stores no raw namespaces, keys, or values. It accepts every
//! declared command and emits only a stable command replay marker, allowing
//! boundary tests to exercise service routing without a database dependency.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, KernelServiceId,
    KeyValueStateProviderCapability, KeyValueStateProviderSnapshot, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceType, TraceSchemaRef, FOUNDATION_KEY_VALUE_STATE_COMMANDS,
    FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};

use crate::KeyValueStateService;

/// In-memory lifecycle counters with no state values or provider handles.
#[derive(Debug, Default)]
pub struct MockKeyValueStateProvider {
    active_watches: Arc<Mutex<u32>>,
}

#[async_trait]
impl KeyValueStateService for MockKeyValueStateProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor()
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        let operation = command.name.as_str();
        if !FOUNDATION_KEY_VALUE_STATE_COMMANDS.contains(&operation) {
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        let output = if operation == "kv.watch_namespace" {
            let checkpoint = stable_hash(trace.trace_id.as_str());
            *self.active_watches.lock().map_err(lock_error)? += 1;
            serde_json::json!({"status":"success","watch_checkpoint":checkpoint,"redacted":true})
        } else {
            serde_json::json!({"status":"success","provider_class":"mock","redacted":true})
        };
        tracing::info!(service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID, command = operation,
            trace_id = %trace.trace_id, "key-value state mock provider command completed");
        Ok(ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::from([
                ("replay.provider_class".into(), "mock".into()),
                ("replay.key_value_state_command".into(), operation.into()),
                (
                    "key_value_state.audit_event".into(),
                    audit_event(operation).into(),
                ),
                ("service.audit.stage".into(), audit_event(operation).into()),
                (
                    "key_value_state.redaction".into(),
                    "namespaces_keys_and_values_redacted".into(),
                ),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }

    fn snapshot(&self) -> KeyValueStateProviderSnapshot {
        KeyValueStateProviderSnapshot {
            descriptor_hash: "foundation-key-value-state-mock-v1".into(),
            provider_class: "mock".into(),
            namespace_hashes: Default::default(),
            active_watch_count: self.active_watches.lock().map(|count| *count).unwrap_or(0),
        }
    }

    fn provider_capabilities(&self) -> KeyValueStateProviderCapability {
        KeyValueStateProviderCapability {
            provider_class: "mock".into(),
            supported_commands: FOUNDATION_KEY_VALUE_STATE_COMMANDS
                .iter()
                .map(|item| (*item).into())
                .collect(),
            supports_ttl: true,
            supports_watch: true,
            supports_snapshot: true,
            supports_compaction: true,
            max_value_bytes: 1_048_576,
            max_batch_entries: 500,
            availability: DomainPackProviderCapabilityState::Available,
        }
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        *self.active_watches.lock().map_err(lock_error)? = 0;
        tracing::info!(
            service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            "key-value state mock lifecycle state cleared"
        );
        Ok(())
    }
}

fn descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(FOUNDATION_KEY_VALUE_STATE_SERVICE_ID),
        ServiceType::new("foundation.key_value_state"),
        TraceSchemaRef::new("macaca.trace.foundation.key_value_state.v1"),
    );
    descriptor.health = ServiceHealth::Healthy;
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor.capabilities = FOUNDATION_KEY_VALUE_STATE_COMMANDS
        .iter()
        .map(|name| ServiceCapability::new(CapabilityId::new(*name), "key-value state command"))
        .collect();
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor
}

fn audit_event(operation: &str) -> &'static str {
    match operation {
        "kv.watch_namespace" => "key_value_state_pack_watch_started",
        "kv.snapshot_namespace" => "key_value_state_pack_snapshot_recorded",
        "kv.restore_namespace" => "key_value_state_pack_restore_completed",
        "kv.migrate_namespace" => "key_value_state_pack_namespace_migrated",
        "kv.compact_namespace" => "key_value_state_pack_namespace_compacted",
        _ => "key_value_state_pack_service_call_succeeded",
    }
}

fn stable_hash(value: &str) -> String {
    format!(
        "{:016x}",
        value.bytes().fold(0_u64, |state, byte| state
            .wrapping_mul(1099511628211)
            .wrapping_add(byte as u64))
    )
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::AdapterFailure("key-value mock state lock poisoned".into())
}
