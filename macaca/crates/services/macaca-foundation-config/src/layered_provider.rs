//! Provider-neutral layered configuration composition for runtime hosts.
//!
//! This module implements the Adapter pattern: host-owned package, workspace,
//! environment, tenant, and remote sources present the same bounded reference
//! interface. The precedence vector is an explicit Strategy, keeping source
//! ordering declarative and preventing application-specific configuration logic.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, ConfigProviderSnapshot, ConfigRedactionSummary, KernelServiceId,
    ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef, FOUNDATION_CONFIG_COMMANDS,
    FOUNDATION_CONFIG_SERVICE_ID,
};

use crate::ConfigService;

/// Stable, provider-neutral kinds that runtime composition may bind to a source.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConfigSourceKind {
    /// Defaults embedded in a package descriptor.
    PackageDescriptor,
    /// Workspace-scoped declarative configuration.
    Workspace,
    /// Bounded environment adapter output; never an environment dump.
    Environment,
    /// Tenant-scoped configuration selected by the runtime policy layer.
    Tenant,
    /// A remote configuration bridge with no SDK-visible provider API.
    Remote,
}

impl ConfigSourceKind {
    fn name(self) -> &'static str {
        match self {
            Self::PackageDescriptor => "package_descriptor",
            Self::Workspace => "workspace",
            Self::Environment => "environment",
            Self::Tenant => "tenant",
            Self::Remote => "remote",
        }
    }
}

/// A bounded source adapter that stores only opaque artifact or secret references.
#[derive(Clone, Debug)]
pub struct ReferenceMapConfigSource {
    kind: ConfigSourceKind,
    source_id: String,
    values: Arc<Mutex<BTreeMap<String, String>>>,
}

impl ReferenceMapConfigSource {
    /// Create an empty adapter with a bounded opaque source identifier.
    pub fn new(kind: ConfigSourceKind, source_id: impl Into<String>) -> ServiceResult<Self> {
        let source_id = source_id.into();
        if source_id.is_empty() || source_id.len() > 128 {
            return Err(ServiceError::InvalidArgument(
                "bounded source id required".into(),
            ));
        }
        Ok(Self {
            kind,
            source_id,
            values: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    /// Store an opaque reference after host admission has validated the source.
    pub fn insert_reference(
        &self,
        key: impl Into<String>,
        reference: impl Into<String>,
    ) -> ServiceResult<()> {
        let key = key.into();
        let reference = reference.into();
        if key.is_empty() || key.len() > 256 {
            return Err(ServiceError::InvalidArgument(
                "bounded config key required".into(),
            ));
        }
        if !reference.starts_with("artifact:") && !reference.starts_with("secret:") {
            return Err(ServiceError::InvalidArgument(
                "config values must be opaque references".into(),
            ));
        }
        self.values
            .lock()
            .map_err(lock_error)?
            .insert(key, reference);
        Ok(())
    }

    fn get(&self, key: &str) -> ServiceResult<Option<String>> {
        Ok(self.values.lock().map_err(lock_error)?.get(key).cloned())
    }

    fn source_hash(&self) -> String {
        hash(&format!("{}:{}", self.kind.name(), self.source_id))
    }
}

/// Layered provider with deterministic highest-priority-last resolution.
pub struct LayeredConfigProvider {
    sources: Vec<ReferenceMapConfigSource>,
}

impl LayeredConfigProvider {
    /// Compose source adapters in ascending precedence; later entries override earlier ones.
    pub fn new(sources: Vec<ReferenceMapConfigSource>) -> ServiceResult<Self> {
        if sources.len() > 16 {
            return Err(ServiceError::InvalidArgument(
                "too many config sources".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        for source in &sources {
            if !identities.insert((source.kind, source.source_id.clone())) {
                return Err(ServiceError::InvalidArgument(
                    "duplicate config source".into(),
                ));
            }
        }
        Ok(Self { sources })
    }

    fn effective_reference(&self, key: &str) -> ServiceResult<Option<(String, String)>> {
        for source in self.sources.iter().rev() {
            if let Some(reference) = source.get(key)? {
                return Ok(Some((reference, source.source_hash())));
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl ConfigService for LayeredConfigProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_CONFIG_SERVICE_ID),
            ServiceType::new("foundation.config"),
            TraceSchemaRef::new("macaca.trace.foundation.config.v1"),
        );
        descriptor.health = ServiceHealth::Healthy;
        descriptor.cleanup_policy = CleanupPolicy::OnStop;
        descriptor.capabilities = FOUNDATION_CONFIG_COMMANDS
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "config command"))
            .collect();
        descriptor
            .metadata
            .insert("provider_class".into(), "layered_adapter".into());
        descriptor
            .metadata
            .insert("source_count".into(), self.sources.len().to_string());
        descriptor
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
                let resolved = self.effective_reference(&key)?;
                serde_json::json!({"status": if resolved.is_some() { "success" } else { "not_found" }, "key": hash(&key), "value_ref": resolved.as_ref().map(|item| &item.0), "source_hash": resolved.as_ref().map(|item| &item.1), "redacted": true})
            }
            "config.get_many"
            | "config.list_keys"
            | "config.describe_schema"
            | "config.validate"
            | "config.watch"
            | "config.reload"
            | "config.snapshot"
            | "config.export_redacted" => {
                serde_json::json!({"status":"success","provider_class":"layered_adapter","source_count":self.sources.len(),"redacted":true})
            }
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        tracing::info!(service_id = FOUNDATION_CONFIG_SERVICE_ID, command = operation, trace_id = %trace.trace_id, source_count = self.sources.len(), "foundation layered config command completed");
        Ok(ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::from([
                ("replay.provider_class".into(), "layered_adapter".into()),
                ("replay.config_command".into(), operation.into()),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }

    fn snapshot(&self) -> ConfigProviderSnapshot {
        ConfigProviderSnapshot {
            descriptor_hash: "foundation-config-layered-v1".into(),
            provider_class: "layered_adapter".into(),
            source_hashes: self
                .sources
                .iter()
                .map(|source| (source.kind.name().into(), source.source_hash()))
                .collect(),
            schema_hashes: BTreeMap::new(),
            redaction_summary: ConfigRedactionSummary {
                redacted_value_count: 0,
                redacted_source_count: self.sources.len() as u32,
                contains_secret_references: true,
            },
        }
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        for source in &self.sources {
            source.values.lock().map_err(lock_error)?.clear();
        }
        tracing::info!(
            service_id = FOUNDATION_CONFIG_SERVICE_ID,
            "foundation layered config sources cleared during shutdown"
        );
        Ok(())
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
    ServiceError::AdapterFailure("config source lock poisoned".into())
}
