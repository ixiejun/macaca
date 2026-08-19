//! Deterministic metadata-only secret-reference provider for replay tests.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, KernelServiceId,
    SecretReference, SecretVersionState, SecretsCreateReferenceCommand,
    SecretsImportReferenceCommand, SecretsReferenceProviderCapability,
    SecretsReferenceProviderSnapshot, ServiceCallResult, ServiceCapability, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    FOUNDATION_SECRETS_REFERENCE_COMMANDS, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
};

use crate::SecretsReferenceService;

/// In-memory Strategy that stores only safe reference ids and lease metadata.
#[derive(Debug, Default)]
pub struct MockSecretsReferenceProvider {
    references: Arc<Mutex<BTreeMap<String, SecretReference>>>,
    leases: Arc<Mutex<BTreeSet<String>>>,
}

#[async_trait]
impl SecretsReferenceService for MockSecretsReferenceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_SECRETS_REFERENCE_SERVICE_ID),
            ServiceType::new("foundation.secrets_reference"),
            TraceSchemaRef::new("macaca.trace.foundation.secrets_reference.v1"),
        );
        descriptor.health = ServiceHealth::Healthy;
        descriptor.cleanup_policy = CleanupPolicy::OnStop;
        descriptor.capabilities = FOUNDATION_SECRETS_REFERENCE_COMMANDS
            .iter()
            .map(|name| {
                ServiceCapability::new(CapabilityId::new(*name), "secret reference command")
            })
            .collect();
        descriptor
            .metadata
            .insert("provider_class".into(), "mock".into());
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        let operation = command.name.as_str();
        if !FOUNDATION_SECRETS_REFERENCE_COMMANDS.contains(&operation) {
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        let output = match operation {
            "secrets.create_reference" => {
                let request: SecretsCreateReferenceCommand = decode(&command.payload)?;
                validate_reference(&request.reference)?;
                self.references
                    .lock()
                    .map_err(lock_error)?
                    .insert(request.reference.reference_id.clone(), request.reference);
                serde_json::json!({"status":"success","reference_created":true})
            }
            "secrets.import_reference" => {
                let request: SecretsImportReferenceCommand = decode(&command.payload)?;
                let id = stable_hash(&request.locator.redacted_locator_hash);
                let reference = SecretReference {
                    reference_id: format!("ref:{id}"),
                    provider_class: request.locator.provider_class,
                    version_hint: Some("current".into()),
                };
                self.references
                    .lock()
                    .map_err(lock_error)?
                    .insert(reference.reference_id.clone(), reference.clone());
                serde_json::json!({"status":"success","reference":reference,"redacted":true})
            }
            "secrets.create_lease" => {
                let reference: SecretReference = decode(
                    command
                        .payload
                        .get("reference")
                        .unwrap_or(&serde_json::Value::Null),
                )?;
                validate_reference(&reference)?;
                let lease = format!("lease:{}", stable_hash(&trace.trace_id));
                self.leases
                    .lock()
                    .map_err(lock_error)?
                    .insert(lease.clone());
                serde_json::json!({"status":"success","lease_id":lease,"raw_value":null})
            }
            "secrets.resolve_for_provider" => {
                serde_json::json!({"status":"success","resolution_handle":format!("handle:{}", stable_hash(&trace.trace_id)),"raw_value":null})
            }
            "secrets.list_references" => {
                serde_json::json!({"status":"success","reference_count":self.references.lock().map_err(lock_error)?.len(),"redacted":true})
            }
            "secrets.revoke_lease" => {
                self.leases.lock().map_err(lock_error)?.clear();
                serde_json::json!({"status":"success","revoked":true})
            }
            "secrets.rotate_reference" => {
                serde_json::json!({"status":"success","rotation_state":"current","redacted":true})
            }
            "secrets.version_status" => {
                serde_json::json!({"status":"success","version_state":"current"})
            }
            "secrets.audit_access" => {
                serde_json::json!({"status":"success","audit_count":0,"redacted":true})
            }
            _ => serde_json::json!({"status":"success","redacted":true}),
        };
        tracing::info!(service_id = FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
            command = operation, trace_id = %trace.trace_id,
            "secrets reference mock provider command completed");
        Ok(ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::from([
                ("replay.provider_class".into(), "mock".into()),
                ("replay.secrets_reference_command".into(), operation.into()),
                ("service.audit.stage".into(), audit_stage(operation).into()),
                (
                    "secrets_reference.redaction".into(),
                    "raw_values_locators_and_payloads_redacted".into(),
                ),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> SecretsReferenceProviderSnapshot {
        SecretsReferenceProviderSnapshot {
            descriptor_hash: "foundation-secrets-reference-mock-v1".into(),
            provider_class: "mock".into(),
            reference_state_hashes: self
                .references
                .lock()
                .map(|refs| {
                    refs.keys()
                        .map(|id| (id.clone(), stable_hash(id)))
                        .collect()
                })
                .unwrap_or_default(),
            lease_state_hashes: self
                .leases
                .lock()
                .map(|leases| {
                    leases
                        .iter()
                        .map(|id| (id.clone(), stable_hash(id)))
                        .collect()
                })
                .unwrap_or_default(),
            audit_tail_hash: "mock".into(),
        }
    }
    fn provider_capabilities(&self) -> SecretsReferenceProviderCapability {
        SecretsReferenceProviderCapability {
            provider_class: "mock".into(),
            supported_commands: FOUNDATION_SECRETS_REFERENCE_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect(),
            supported_version_states: [
                SecretVersionState::Current,
                SecretVersionState::Previous,
                SecretVersionState::Disabled,
                SecretVersionState::Destroyed,
            ]
            .into_iter()
            .collect(),
            supports_leases: true,
            supports_rotation: true,
            supports_provider_injection: true,
            raw_value_app_results_forbidden: true,
            max_lease_ttl_seconds: 86_400,
            availability: DomainPackProviderCapabilityState::Available,
        }
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        self.leases.lock().map_err(lock_error)?.clear();
        Ok(())
    }
}

fn decode<T: serde::de::DeserializeOwned>(payload: &serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload.clone())
        .map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}
fn validate_reference(reference: &SecretReference) -> ServiceResult<()> {
    if reference.is_safe_reference() {
        Ok(())
    } else {
        Err(ServiceError::InvalidArgument(
            "unsafe secret reference metadata".into(),
        ))
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
    ServiceError::AdapterFailure("secret reference mock lock poisoned".into())
}
fn audit_stage(operation: &str) -> &'static str {
    match operation {
        "secrets.create_reference" | "secrets.import_reference" => {
            "secrets_reference_pack_reference_created"
        }
        "secrets.resolve_for_provider" => "secrets_reference_pack_provider_resolution",
        "secrets.create_lease" => "secrets_reference_pack_lease_created",
        "secrets.revoke_lease" => "secrets_reference_pack_lease_revoked",
        "secrets.rotate_reference" => "secrets_reference_pack_rotation",
        "secrets.audit_access" => "secrets_reference_pack_audit_access",
        _ => "secrets_reference_pack_service_call_succeeded",
    }
}
