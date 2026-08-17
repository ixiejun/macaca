//! Deterministic filesystem Strategy for contract, replay, and boundary tests.
//!
//! The provider stores only opaque content references keyed by hashed logical
//! paths. It intentionally does not touch a host filesystem; a local scoped
//! provider is composed separately by the runtime host. This makes it safe to
//! exercise every declared command without leaking file bytes or host paths.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, FilesystemProviderCapability,
    FilesystemProviderSnapshot, KernelServiceId, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef, FOUNDATION_FILESYSTEM_COMMANDS, FOUNDATION_FILESYSTEM_SERVICE_ID,
};

use crate::service_contract::FilesystemService;

/// In-memory provider state containing only safe identifiers and opaque references.
#[derive(Debug, Default)]
pub struct MockFilesystemProvider {
    content_refs: Arc<Mutex<BTreeMap<String, String>>>,
    handles: Arc<Mutex<BTreeSet<String>>>,
    watches: Arc<Mutex<BTreeSet<String>>>,
}

#[async_trait]
impl FilesystemService for MockFilesystemProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor()
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        let operation = command.name.as_str();
        if !FOUNDATION_FILESYSTEM_COMMANDS.contains(&operation) {
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        let output = match operation {
            "filesystem.open_handle" => {
                let handle = stable_hash(trace.trace_id.as_str());
                self.handles
                    .lock()
                    .map_err(lock_error)?
                    .insert(handle.clone());
                serde_json::json!({"status":"success","handle_checkpoint":handle,"redacted":true})
            }
            "filesystem.close_handle" => {
                let handle = command
                    .payload
                    .get("handle")
                    .and_then(|value| value.get("handle_id"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                self.handles.lock().map_err(lock_error)?.remove(handle);
                serde_json::json!({"status":"success","redacted":true})
            }
            "filesystem.read_file" => {
                let path_key = path_key(&command.payload)?;
                let content_ref = self
                    .content_refs
                    .lock()
                    .map_err(lock_error)?
                    .get(&path_key)
                    .cloned();
                serde_json::json!({"status":if content_ref.is_some(){"success"}else{"not_found"},"content_ref":content_ref,"path_hash":path_key,"redacted":true})
            }
            "filesystem.write_file" | "filesystem.append_file" => {
                let path_key = path_key(&command.payload)?;
                let content_ref = command
                    .payload
                    .get("content")
                    .and_then(|value| value.get("content_ref"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| value.starts_with("artifact:"))
                    .ok_or_else(|| {
                        ServiceError::InvalidArgument(
                            "opaque artifact content reference required".into(),
                        )
                    })?;
                self.content_refs
                    .lock()
                    .map_err(lock_error)?
                    .insert(path_key.clone(), content_ref.into());
                serde_json::json!({"status":"success","path_hash":path_key,"redacted":true})
            }
            "filesystem.watch_path" => {
                let checkpoint = stable_hash(trace.trace_id.as_str());
                self.watches
                    .lock()
                    .map_err(lock_error)?
                    .insert(checkpoint.clone());
                serde_json::json!({"status":"success","watch_checkpoint":checkpoint,"redacted":true})
            }
            "filesystem.list_directory"
            | "filesystem.stat_path"
            | "filesystem.create_directory"
            | "filesystem.copy_path"
            | "filesystem.move_path"
            | "filesystem.delete_path"
            | "filesystem.create_temp"
            | "filesystem.snapshot_tree"
            | "filesystem.restore_snapshot" => {
                serde_json::json!({"status":"success","provider_class":"mock","redacted":true})
            }
            _ => unreachable!("declared command set was checked before dispatch"),
        };
        tracing::info!(service_id = FOUNDATION_FILESYSTEM_SERVICE_ID, command = operation,
            trace_id = %trace.trace_id, "filesystem mock provider command completed");
        Ok(ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::from([
                ("replay.provider_class".into(), "mock".into()),
                ("replay.filesystem_command".into(), operation.into()),
                (
                    "filesystem.audit_event".into(),
                    audit_event(operation).into(),
                ),
                ("service.audit.stage".into(), audit_event(operation).into()),
                (
                    "filesystem.redaction".into(),
                    "paths_and_content_references_only".into(),
                ),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }

    fn snapshot(&self) -> FilesystemProviderSnapshot {
        FilesystemProviderSnapshot {
            descriptor_hash: "foundation-filesystem-mock-v1".into(),
            provider_class: "mock".into(),
            open_handle_count: self
                .handles
                .lock()
                .map(|items| items.len() as u32)
                .unwrap_or(0),
            active_watch_count: self
                .watches
                .lock()
                .map(|items| items.len() as u32)
                .unwrap_or(0),
            root_hashes: self
                .content_refs
                .lock()
                .map(|items| {
                    items
                        .keys()
                        .map(|key| (key.clone(), "mock-root-v1".into()))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn provider_capabilities(&self) -> FilesystemProviderCapability {
        FilesystemProviderCapability {
            provider_class: "mock".into(),
            supported_commands: FOUNDATION_FILESYSTEM_COMMANDS
                .iter()
                .map(|item| (*item).into())
                .collect(),
            supported_root_kinds: ["app_workspace", "session_workspace", "temporary"]
                .into_iter()
                .map(String::from)
                .collect(),
            supports_recursive_operations: true,
            supports_watch: true,
            supports_snapshot: true,
            supports_atomic_write: true,
            max_file_bytes: 16_777_216,
            max_directory_entries: 10_000,
            availability: DomainPackProviderCapabilityState::Available,
            unavailable_reason: None,
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
            service_id = FOUNDATION_FILESYSTEM_SERVICE_ID,
            watch_checkpoint_hash = stable_hash(watch_checkpoint),
            "filesystem mock watch cancelled"
        );
        Ok(())
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        self.content_refs.lock().map_err(lock_error)?.clear();
        self.handles.lock().map_err(lock_error)?.clear();
        self.watches.lock().map_err(lock_error)?.clear();
        tracing::info!(
            service_id = FOUNDATION_FILESYSTEM_SERVICE_ID,
            "filesystem mock provider lifecycle state cleared"
        );
        Ok(())
    }
}

fn descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(FOUNDATION_FILESYSTEM_SERVICE_ID),
        ServiceType::new("foundation.filesystem"),
        TraceSchemaRef::new("macaca.trace.foundation.filesystem.v1"),
    );
    descriptor.health = ServiceHealth::Healthy;
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor.capabilities = FOUNDATION_FILESYSTEM_COMMANDS
        .iter()
        .map(|name| ServiceCapability::new(CapabilityId::new(*name), "filesystem command"))
        .collect();
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor
}

fn path_key(payload: &serde_json::Value) -> ServiceResult<String> {
    let path = payload
        .get("path")
        .or_else(|| payload.get("source"))
        .and_then(|value| value.get("relative_path"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 512)
        .ok_or_else(|| ServiceError::InvalidArgument("bounded logical path required".into()))?;
    Ok(stable_hash(path))
}

fn audit_event(operation: &str) -> &'static str {
    match operation {
        "filesystem.open_handle" => "filesystem_pack_handle_opened",
        "filesystem.close_handle" => "filesystem_pack_handle_closed",
        "filesystem.watch_path" => "filesystem_pack_watch_started",
        "filesystem.snapshot_tree" => "filesystem_pack_snapshot_recorded",
        "filesystem.restore_snapshot" => "filesystem_pack_restore_completed",
        _ => "filesystem_pack_service_call_succeeded",
    }
}

fn stable_hash(value: &str) -> String {
    format!(
        "{:016x}",
        value.bytes().fold(0_u64, |state, byte| {
            state.wrapping_mul(1099511628211).wrapping_add(byte as u64)
        })
    )
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::AdapterFailure("filesystem mock state lock poisoned".into())
}
