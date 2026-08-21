//! Restart, conflict, checkpoint, and redaction tests for the durable session-state Strategy.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::PersistStore;
use macaca_proto::{
    MacacaResult, ServiceCommand, ServiceCommandName, SessionStateKeyRef, SessionStatePutCommand,
    SessionStateRetentionPolicy, SessionStateSessionRef, SessionStateValueRef, TraceContext,
};
use std::collections::BTreeMap;
use tokio::sync::Mutex;

use super::foundation_session_state_embedded_provider::EmbeddedFoundationSessionStateProvider;
use macaca_kernel::SystemService;

#[derive(Default)]
struct MemoryPersistStore(Mutex<BTreeMap<String, Vec<u8>>>);

#[async_trait]
impl PersistStore for MemoryPersistStore {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        Ok(self.0.lock().await.get(key).cloned())
    }
    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()> {
        self.0.lock().await.insert(key.into(), value.into());
        Ok(())
    }
    async fn delete(&self, key: &str) -> MacacaResult<()> {
        self.0.lock().await.remove(key);
        Ok(())
    }
    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
        Ok(self
            .0
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect())
    }
}

fn session() -> SessionStateSessionRef {
    SessionStateSessionRef {
        session_id: "session-test".into(),
        task_id: Some("task-test".into()),
    }
}

fn put_command(trace_id: &str, value_ref: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new("session_state.put"),
        serde_json::to_value(SessionStatePutCommand {
            key: SessionStateKeyRef {
                session: session(),
                key: "form.field".into(),
            },
            value: SessionStateValueRef {
                value_ref: value_ref.into(),
                schema_id: Some("form.v1".into()),
                secret_reference_required: false,
            },
            expected_revision: None,
        })
        .unwrap(),
        TraceContext::new(trace_id),
    )
}

fn command_with_metadata(
    name: &str,
    payload: serde_json::Value,
    trace_id: &str,
    metadata: &[(&str, &str)],
) -> ServiceCommand {
    let mut command = ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(trace_id),
    );
    command.metadata = metadata
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect::<BTreeMap<_, _>>();
    command
}

#[tokio::test]
async fn embedded_provider_replays_state_after_provider_restart() {
    let store = Arc::new(MemoryPersistStore::default());
    let first = EmbeddedFoundationSessionStateProvider::new(store.clone());
    first
        .call(put_command("durable-put", "artifact:opaque-form-state"))
        .await
        .unwrap();

    // A new provider instance represents a process restart; only the injected store survives.
    let restarted = EmbeddedFoundationSessionStateProvider::new(store);
    let reply = restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.get"),
            serde_json::json!({
                "key": {"session": session(), "key": "form.field"}
            }),
            TraceContext::new("durable-get"),
        ))
        .await
        .unwrap();
    assert_eq!(reply.output["status"], "ok");
    assert_eq!(reply.output["value_present"], true);
    assert!(!serde_json::to_string(&reply)
        .unwrap()
        .contains("opaque-form-state"));

    let recovery = restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.inspect_recovery"),
            serde_json::json!({"session": session()}),
            TraceContext::new("durable-recovery"),
        ))
        .await
        .unwrap();
    assert_eq!(recovery.output["recovery_state"], "durable");
}

#[tokio::test]
async fn checkpoint_restore_supports_dry_run_and_revision_conflicts() {
    let provider =
        EmbeddedFoundationSessionStateProvider::new(Arc::new(MemoryPersistStore::default()));
    let initial = provider
        .call(put_command("checkpoint-put", "artifact:before"))
        .await
        .unwrap();
    let checkpoint = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.create_checkpoint"),
            serde_json::json!({
                "session": session(),
                "retention": SessionStateRetentionPolicy {
                    ttl_seconds: None,
                    max_checkpoints: 4,
                    compact_after_revisions: 20,
                }
            }),
            TraceContext::new("checkpoint-create"),
        ))
        .await
        .unwrap();
    let checkpoint_ref = checkpoint.output["checkpoint_ref"].clone();
    provider
        .call(put_command("checkpoint-after", "artifact:after"))
        .await
        .unwrap();

    let dry_run = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.restore_checkpoint"),
            serde_json::json!({
                "plan": {"checkpoint": checkpoint_ref, "dry_run": true, "cross_session_allowed": false}
            }),
            TraceContext::new("restore-dry-run"),
        ))
        .await
        .unwrap();
    assert_eq!(dry_run.output["dry_run"], true);

    let conflict = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.put"),
            serde_json::json!({
                "key": {"session": session(), "key": "form.field"},
                "value": {"value_ref": "artifact:conflict", "schema_id": null, "secret_reference_required": false},
                "expected_revision": {"revision_id": "wrong", "previous_revision_id": null}
            }),
            TraceContext::new("revision-conflict"),
        ))
        .await
        .unwrap_err();
    assert!(conflict.to_string().contains("revision conflict"));
    assert_eq!(initial.output["status"], "ok");
}

#[tokio::test]
async fn denied_sensitive_operations_do_not_mutate_durable_state() {
    let provider =
        EmbeddedFoundationSessionStateProvider::new(Arc::new(MemoryPersistStore::default()));
    provider
        .call(put_command("approval-seed", "artifact:seed"))
        .await
        .unwrap();
    let checkpoint = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.create_checkpoint"),
            serde_json::json!({
                "session": session(),
                "retention": SessionStateRetentionPolicy {
                    ttl_seconds: None,
                    max_checkpoints: 4,
                    compact_after_revisions: 20,
                }
            }),
            TraceContext::new("approval-checkpoint"),
        ))
        .await
        .unwrap();
    let checkpoint_ref = checkpoint.output["checkpoint_ref"].clone();
    let before = provider.snapshot().await.unwrap();

    let denied = [
        command_with_metadata(
            "session_state.restore_checkpoint",
            serde_json::json!({
                "plan": {"checkpoint": checkpoint_ref, "dry_run": false, "cross_session_allowed": true}
            }),
            "denied-cross-session",
            &[],
        ),
        command_with_metadata(
            "session_state.compact_history",
            serde_json::json!({
                "session": session(),
                "before_revision": {"revision_id": "revision:before", "previous_revision_id": null},
                "dry_run": false
            }),
            "denied-compact",
            &[],
        ),
        command_with_metadata(
            "session_state.clear_session",
            serde_json::json!({"session": session(), "dry_run": false}),
            "denied-clear",
            &[],
        ),
        command_with_metadata(
            "session_state.export_redacted",
            serde_json::json!({"session": session(), "redaction_level": "diagnostic"}),
            "denied-export",
            &[],
        ),
    ];
    for command in denied {
        let error = provider.call(command).await.unwrap_err();
        assert!(error.to_string().contains("approval_required"));
        assert_eq!(provider.snapshot().await.unwrap(), before);
    }
}

#[tokio::test]
async fn approved_clear_is_allowed_after_policy_evidence() {
    let provider =
        EmbeddedFoundationSessionStateProvider::new(Arc::new(MemoryPersistStore::default()));
    provider
        .call(put_command("approved-seed", "artifact:seed"))
        .await
        .unwrap();
    let result = provider
        .call(command_with_metadata(
            "session_state.clear_session",
            serde_json::json!({"session": session(), "dry_run": false}),
            "approved-clear",
            &[
                ("approval_granted", "true"),
                ("approval_source", "policy"),
                ("approval_ref", "approval:clear-session"),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(result.output["status"], "ok");
    assert!(provider
        .snapshot()
        .await
        .unwrap()
        .revision_hashes
        .is_empty());
}
