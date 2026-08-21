//! Embedded durable Strategy for the foundation session-state pack.
//!
//! The provider owns serialization and retention while the host owns the injected
//! [`PersistStore`]. Logical session ids, keys, and values are never used as
//! observability fields; only bounded hashes and opaque references cross the
//! runtime boundary. This keeps the provider replaceable by a remote store.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_persist::PersistStore;
use macaca_proto::{
    approve_session_state_operation, domain_pack_command_trace, domain_pack_service_result,
    DomainPackProviderCapabilityState, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, SessionStateApprovalFacts,
    SessionStateCheckpointRef, SessionStateClearSessionCommand, SessionStateCompactHistoryCommand,
    SessionStateCompareCheckpointCommand, SessionStateCreateCheckpointCommand,
    SessionStateDeleteCommand, SessionStateExportRedactedCommand, SessionStateGetCommand,
    SessionStateInspectRecoveryCommand, SessionStateKeyRef, SessionStateListCheckpointsCommand,
    SessionStateListKeysCommand, SessionStateMergePatchCommand, SessionStateProviderCapability,
    SessionStateProviderSnapshot, SessionStatePutCommand, SessionStateRecoveryMetadata,
    SessionStateRestoreCheckpointCommand, SessionStateRetentionPolicy, SessionStateRevision,
    SessionStateSessionRef, SessionStateValueRef, FOUNDATION_SESSION_STATE_COMMANDS,
    FOUNDATION_SESSION_STATE_SERVICE_ID,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

use super::foundation_session_state_service_provider::{
    foundation_session_state_service_descriptor, SessionStateRuntimeEvent,
    SessionStateRuntimeEventKind,
};

const STORAGE_PREFIX: &str = "session-state/";
const MAX_CHECKPOINTS: u32 = 128;
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DurableRecord {
    revision: u64,
    entries: BTreeMap<String, SessionStateValueRef>,
    checkpoints: BTreeMap<String, DurableCheckpoint>,
    retention: Option<SessionStateRetentionPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DurableCheckpoint {
    session: SessionStateSessionRef,
    revision: u64,
    entries: BTreeMap<String, SessionStateValueRef>,
}

/// Durable embedded provider composed from the generic persistence port.
pub struct EmbeddedFoundationSessionStateProvider {
    store: Arc<dyn PersistStore>,
    events: broadcast::Sender<SessionStateRuntimeEvent>,
    mutations: Mutex<()>,
}

impl EmbeddedFoundationSessionStateProvider {
    /// Inject persistence rather than opening a database, preserving composition-root ownership.
    pub fn new(store: Arc<dyn PersistStore>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            store,
            events,
            mutations: Mutex::new(()),
        }
    }

    /// Subscribe to bounded lifecycle events suitable for replay and audit consumers.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionStateRuntimeEvent> {
        self.events.subscribe()
    }

    /// Report durable capability without exposing the concrete persistence backend.
    pub fn capability(&self) -> SessionStateProviderCapability {
        SessionStateProviderCapability {
            provider_class: "embedded-durable".into(),
            supported_commands: FOUNDATION_SESSION_STATE_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect::<BTreeSet<_>>(),
            supports_checkpoints: true,
            supports_restore: true,
            supports_compaction: true,
            supports_redacted_export: true,
            max_state_bytes: 1_048_576,
            max_checkpoint_bytes: 4_194_304,
            availability: DomainPackProviderCapabilityState::Available,
        }
    }

    /// Read a bounded Memento. Values are reduced to deterministic hashes.
    pub async fn snapshot(&self) -> ServiceResult<SessionStateProviderSnapshot> {
        let keys = self
            .store
            .list_keys(STORAGE_PREFIX)
            .await
            .map_err(storage_error)?;
        let mut revision_hashes = BTreeMap::new();
        let mut checkpoint_hashes = BTreeMap::new();
        for storage_key in keys.into_iter().take(100) {
            let Some(bytes) = self.store.get(&storage_key).await.map_err(storage_error)? else {
                continue;
            };
            let record: DurableRecord = serde_json::from_slice(&bytes).map_err(json_error)?;
            let session_hash = storage_key.trim_start_matches(STORAGE_PREFIX).to_string();
            revision_hashes.insert(session_hash.clone(), hash(&record.revision.to_string()));
            checkpoint_hashes.insert(
                session_hash,
                hash(
                    &record
                        .checkpoints
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            );
        }
        let _ = self.events.send(event(
            "snapshot",
            "snapshot",
            SessionStateRuntimeEventKind::SnapshotRecorded,
        ));
        Ok(SessionStateProviderSnapshot {
            descriptor_hash: "foundation-session-state:embedded-durable".into(),
            provider_class: "embedded-durable".into(),
            revision_hashes,
            checkpoint_hashes,
            redaction_summary: macaca_proto::SessionStateRedactionSummary {
                redacted_value_count: 0,
                redacted_secret_reference_count: 0,
            },
        })
    }

    async fn load(&self, session: &SessionStateSessionRef) -> ServiceResult<DurableRecord> {
        validate_session(session)?;
        let key = storage_key(session);
        let Some(bytes) = self.store.get(&key).await.map_err(storage_error)? else {
            return Ok(DurableRecord::default());
        };
        serde_json::from_slice(&bytes).map_err(json_error)
    }

    async fn save(
        &self,
        session: &SessionStateSessionRef,
        record: &DurableRecord,
    ) -> ServiceResult<()> {
        let bytes = serde_json::to_vec(record).map_err(json_error)?;
        if bytes.len() > 4_194_304 {
            return Err(ServiceError::AdapterFailure(
                "session state quota exceeded".into(),
            ));
        }
        self.store
            .set(&storage_key(session), &bytes)
            .await
            .map_err(storage_error)
    }

    async fn execute(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !FOUNDATION_SESSION_STATE_COMMANDS.contains(&operation) {
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        // Approval is a provider-neutral Specification and runs before the
        // mutation lock, persistence read, or any provider side effect.
        let facts = approval_facts(operation, &command);
        if approve_session_state_operation(operation, facts).is_err() {
            let _ = self.events.send(event(
                operation,
                &trace.trace_id,
                SessionStateRuntimeEventKind::PolicyDecision,
            ));
            warn!(
                service_id = FOUNDATION_SESSION_STATE_SERVICE_ID,
                command = operation,
                trace_id = %trace.trace_id,
                reason_code = "approval_required",
                "session state command denied before provider side effect"
            );
            return Err(ServiceError::DisabledByPolicy("approval_required".into()));
        }
        let _mutation = self.mutations.lock().await;
        let output = match operation {
            "session_state.get" => self.get(decode(&command.payload)?).await?,
            "session_state.put" => self.put(decode(&command.payload)?).await?,
            "session_state.delete" => self.delete(decode(&command.payload)?).await?,
            "session_state.merge_patch" => self.merge_patch(decode(&command.payload)?).await?,
            "session_state.list_keys" => self.list_keys(decode(&command.payload)?).await?,
            "session_state.create_checkpoint" => {
                self.create_checkpoint(decode(&command.payload)?, &trace.trace_id)
                    .await?
            }
            "session_state.list_checkpoints" => {
                self.list_checkpoints(decode(&command.payload)?).await?
            }
            "session_state.restore_checkpoint" => self.restore(decode(&command.payload)?).await?,
            "session_state.compare_checkpoint" => self.compare(decode(&command.payload)?).await?,
            "session_state.compact_history" => self.compact(decode(&command.payload)?).await?,
            "session_state.clear_session" => self.clear(decode(&command.payload)?).await?,
            "session_state.export_redacted" => {
                self.export_redacted(decode(&command.payload)?).await?
            }
            "session_state.inspect_recovery" => self.inspect(decode(&command.payload)?).await?,
            _ => unreachable!("command list and dispatch must stay in sync"),
        };
        let _ = self.events.send(event(
            operation,
            &trace.trace_id,
            SessionStateRuntimeEventKind::ProviderCallSucceeded,
        ));
        info!(service_id = FOUNDATION_SESSION_STATE_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "embedded session state command completed");
        Ok(domain_pack_service_result(
            output,
            trace,
            "embedded-durable",
        ))
    }

    async fn get(&self, request: SessionStateGetCommand) -> ServiceResult<serde_json::Value> {
        validate_key(&request.key)?;
        let record = self.load(&request.key.session).await?;
        let value = record.entries.get(&request.key.key);
        Ok(
            serde_json::json!({"status": if value.is_some() {"ok"} else {"not_found"}, "value_present": value.is_some(), "revision": revision(&record)}),
        )
    }

    async fn put(&self, request: SessionStatePutCommand) -> ServiceResult<serde_json::Value> {
        validate_key(&request.key)?;
        if !request.value.is_admissible_reference() {
            return Err(ServiceError::AdapterFailure(
                "value must be an opaque artifact or secret reference".into(),
            ));
        }
        let mut record = self.load(&request.key.session).await?;
        check_revision(&record, request.expected_revision.as_ref())?;
        record.entries.insert(request.key.key, request.value);
        record.revision = record.revision.saturating_add(1);
        let revision = revision(&record);
        self.save(&request.key.session, &record).await?;
        Ok(serde_json::json!({"status":"ok","revision":revision}))
    }

    async fn delete(&self, request: SessionStateDeleteCommand) -> ServiceResult<serde_json::Value> {
        validate_key(&request.key)?;
        let mut record = self.load(&request.key.session).await?;
        check_revision(&record, request.expected_revision.as_ref())?;
        let removed = record.entries.remove(&request.key.key).is_some();
        if removed {
            record.revision = record.revision.saturating_add(1);
            self.save(&request.key.session, &record).await?;
        }
        Ok(
            serde_json::json!({"status": if removed {"ok"} else {"not_found"}, "revision":revision(&record)}),
        )
    }

    async fn merge_patch(
        &self,
        request: SessionStateMergePatchCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_key(&request.key)?;
        if !opaque_reference(&request.patch_ref) {
            return Err(ServiceError::AdapterFailure(
                "patch must be an opaque reference".into(),
            ));
        }
        let mut record = self.load(&request.key.session).await?;
        check_revision(&record, request.expected_revision.as_ref())?;
        record.entries.insert(
            request.key.key,
            SessionStateValueRef {
                value_ref: request.patch_ref,
                schema_id: None,
                secret_reference_required: false,
            },
        );
        record.revision = record.revision.saturating_add(1);
        let revision = revision(&record);
        self.save(&request.key.session, &record).await?;
        Ok(serde_json::json!({"status":"ok","revision":revision}))
    }

    async fn list_keys(
        &self,
        request: SessionStateListKeysCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        let record = self.load(&request.session).await?;
        let page_size = request.page_size.clamp(1, MAX_PAGE_SIZE) as usize;
        let prefix = request.prefix.unwrap_or_default();
        let keys = record
            .entries
            .keys()
            .filter(|key| key.starts_with(&prefix))
            .take(page_size)
            .map(|key| hash(key))
            .collect::<Vec<_>>();
        Ok(serde_json::json!({"status":"ok","key_hashes":keys,"revision":revision(&record)}))
    }

    async fn create_checkpoint(
        &self,
        request: SessionStateCreateCheckpointCommand,
        trace_id: &str,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        if !request
            .retention
            .is_bounded(31_536_000, MAX_CHECKPOINTS, 100_000)
        {
            return Err(ServiceError::AdapterFailure(
                "retention policy is out of bounds".into(),
            ));
        }
        let mut record = self.load(&request.session).await?;
        record.retention = Some(request.retention.clone());
        let checkpoint_id = format!(
            "checkpoint:{}",
            hash(&format!("{}:{}", storage_key(&request.session), trace_id))
        );
        record.checkpoints.insert(
            checkpoint_id.clone(),
            DurableCheckpoint {
                session: request.session.clone(),
                revision: record.revision,
                entries: record.entries.clone(),
            },
        );
        while record.checkpoints.len() > request.retention.max_checkpoints as usize {
            let Some(oldest) = record.checkpoints.keys().next().cloned() else {
                break;
            };
            record.checkpoints.remove(&oldest);
        }
        self.save(&request.session, &record).await?;
        let reference = SessionStateCheckpointRef {
            checkpoint_id: checkpoint_id.clone(),
            session: request.session,
            revision_id: revision(&record).revision_id,
        };
        Ok(
            serde_json::json!({"status":"ok","checkpoint_ref":reference,"replay_ref":format!("replay:{}", hash(trace_id))}),
        )
    }

    async fn list_checkpoints(
        &self,
        request: SessionStateListCheckpointsCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        let record = self.load(&request.session).await?;
        let ids = record
            .checkpoints
            .keys()
            .take(request.page_size.clamp(1, MAX_PAGE_SIZE) as usize)
            .map(|id| hash(id))
            .collect::<Vec<_>>();
        Ok(
            serde_json::json!({"status":"ok","checkpoint_hashes":ids,"count":record.checkpoints.len()}),
        )
    }

    async fn restore(
        &self,
        request: SessionStateRestoreCheckpointCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.plan.checkpoint.session)?;
        let mut record = self.load(&request.plan.checkpoint.session).await?;
        let Some(checkpoint) = record
            .checkpoints
            .get(&request.plan.checkpoint.checkpoint_id)
            .cloned()
        else {
            return Ok(serde_json::json!({"status":"not_found"}));
        };
        if request.plan.dry_run {
            return Ok(
                serde_json::json!({"status":"ok","dry_run":true,"would_restore_revision":hash(&checkpoint.revision.to_string())}),
            );
        }
        record.entries = checkpoint.entries;
        record.revision = record.revision.saturating_add(1);
        let revision = revision(&record);
        self.save(&request.plan.checkpoint.session, &record).await?;
        Ok(serde_json::json!({"status":"ok","dry_run":false,"revision":revision}))
    }

    async fn compare(
        &self,
        request: SessionStateCompareCheckpointCommand,
    ) -> ServiceResult<serde_json::Value> {
        Ok(
            serde_json::json!({"status":"ok","same_revision":request.left.revision_id == request.right.revision_id,"left":hash(&request.left.checkpoint_id),"right":hash(&request.right.checkpoint_id)}),
        )
    }

    async fn compact(
        &self,
        request: SessionStateCompactHistoryCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        let mut record = self.load(&request.session).await?;
        let removable = record.checkpoints.keys().cloned().collect::<Vec<_>>();
        if !request.dry_run {
            for id in removable.iter().take(removable.len().saturating_sub(1)) {
                record.checkpoints.remove(id);
            }
            self.save(&request.session, &record).await?;
        }
        Ok(
            serde_json::json!({"status":"ok","dry_run":request.dry_run,"removed_count":if request.dry_run {0} else {removable.len().saturating_sub(1)}}),
        )
    }

    async fn clear(
        &self,
        request: SessionStateClearSessionCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        if request.dry_run {
            return Ok(serde_json::json!({"status":"ok","dry_run":true}));
        }
        self.store
            .delete(&storage_key(&request.session))
            .await
            .map_err(storage_error)?;
        Ok(serde_json::json!({"status":"ok","dry_run":false}))
    }

    async fn export_redacted(
        &self,
        request: SessionStateExportRedactedCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        let record = self.load(&request.session).await?;
        Ok(
            serde_json::json!({"status":"ok","redaction_level":request.redaction_level,"session_hash":hash(&request.session.session_id),"key_count":record.entries.len(),"revision":revision(&record)}),
        )
    }

    async fn inspect(
        &self,
        request: SessionStateInspectRecoveryCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        let record = self.load(&request.session).await?;
        let metadata = SessionStateRecoveryMetadata {
            latest_checkpoint: record
                .checkpoints
                .iter()
                .next_back()
                .map(|(id, checkpoint)| SessionStateCheckpointRef {
                    checkpoint_id: id.clone(),
                    session: checkpoint.session.clone(),
                    revision_id: revision_id(checkpoint.revision).revision_id,
                }),
            latest_revision: Some(revision(&record)),
            recovery_state: "durable".into(),
        };
        Ok(serde_json::to_value(metadata).map_err(json_error)?)
    }
}

/// Convert only bounded command metadata into approval facts. Approval evidence
/// itself is an opaque reference owned by the policy service and is never logged.
fn approval_facts(operation: &str, command: &ServiceCommand) -> SessionStateApprovalFacts {
    let approval_granted = command
        .metadata
        .get("approval_granted")
        .is_some_and(|value| value == "true")
        && command
            .metadata
            .get("approval_source")
            .is_some_and(|value| value == "policy")
        && command
            .metadata
            .get("approval_ref")
            .is_some_and(|value| !value.is_empty() && value.len() <= 128);
    let policy_requires_approval = command
        .metadata
        .get("policy_requires_approval")
        .is_some_and(|value| value == "true");
    let cross_session_restore = operation == "session_state.restore_checkpoint"
        && command.payload["plan"]["cross_session_allowed"] == true;
    let broad_export = operation == "session_state.export_redacted"
        && command.payload["redaction_level"] != "strict";
    let destructive_history_mutation = matches!(
        operation,
        "session_state.restore_checkpoint"
            | "session_state.clear_session"
            | "session_state.compact_history"
    ) && command
        .payload
        .get("dry_run")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
        && command.payload["plan"]["dry_run"] != true;
    SessionStateApprovalFacts {
        cross_session_restore,
        broad_export,
        destructive_history_mutation,
        policy_requires_approval,
        approval_granted,
    }
}

#[async_trait]
impl SystemService for EmbeddedFoundationSessionStateProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = foundation_session_state_service_descriptor();
        descriptor
            .metadata
            .insert("provider_class".into(), "embedded-durable".into());
        descriptor
    }
    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = FOUNDATION_SESSION_STATE_SERVICE_ID,
            "embedded session state provider started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.execute(command).await
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = FOUNDATION_SESSION_STATE_SERVICE_ID,
            "embedded session state provider stopped"
        );
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

fn decode<T: for<'de> Deserialize<'de>>(payload: &serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload.clone()).map_err(json_error)
}
fn validate_session(session: &SessionStateSessionRef) -> ServiceResult<()> {
    if session.is_bounded_reference() {
        Ok(())
    } else {
        Err(ServiceError::AdapterFailure(
            "invalid session reference".into(),
        ))
    }
}
fn validate_key(key: &SessionStateKeyRef) -> ServiceResult<()> {
    if key.is_bounded_reference() {
        Ok(())
    } else {
        Err(ServiceError::AdapterFailure(
            "invalid state key reference".into(),
        ))
    }
}
fn check_revision(
    record: &DurableRecord,
    expected: Option<&SessionStateRevision>,
) -> ServiceResult<()> {
    if expected.is_some_and(|value| value.revision_id != revision(record).revision_id) {
        Err(ServiceError::AdapterFailure("revision conflict".into()))
    } else {
        Ok(())
    }
}
fn revision(record: &DurableRecord) -> SessionStateRevision {
    revision_id(record.revision)
}
fn revision_id(value: u64) -> SessionStateRevision {
    SessionStateRevision {
        revision_id: hash(&value.to_string()),
        previous_revision_id: value
            .checked_sub(1)
            .map(|previous| hash(&previous.to_string())),
    }
}
fn storage_key(session: &SessionStateSessionRef) -> String {
    format!(
        "{STORAGE_PREFIX}{}",
        hash(&format!(
            "{}:{}",
            session.session_id,
            session.task_id.as_deref().unwrap_or("")
        ))
    )
}
fn opaque_reference(value: &str) -> bool {
    value.starts_with("artifact:") || value.starts_with("secret:")
}
fn hash(value: &str) -> String {
    let mut state = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        state = (state ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}
fn storage_error(error: impl std::fmt::Display) -> ServiceError {
    warn!(error = %error, "embedded session state persistence operation failed");
    ServiceError::ServiceUnavailable("session state persistence unavailable".into())
}
fn json_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::AdapterFailure(format!("invalid session state payload: {error}"))
}
fn event(
    command: &str,
    trace_id: &str,
    kind: SessionStateRuntimeEventKind,
) -> SessionStateRuntimeEvent {
    SessionStateRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{}", hash(trace_id)),
    }
}
