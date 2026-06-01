//! Local Workbench proof for the application execution protocol platform.
//!
//! This integration test is intentionally a proof fixture, not production OS
//! logic. It exercises the same durable `ApplicationExecutionEventStore` and
//! provider-neutral event protocol that Web, SDK, hosted providers, external
//! backends, and remote agents use. The fixture creates a tiny frontend/backend
//! Hello World workspace under a session-scoped temporary directory, records
//! only sanitized event summaries, simulates browser subscriber disconnect, and
//! verifies replay/current-state recovery after reconnect for all provider
//! kinds.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionReplayRequest, ApplicationExecutionScope,
    ApplicationId, TraceContext,
};
use macaca_runtime_host::ApplicationExecutionEventStore;
use tempfile::{tempdir, TempDir};

#[tokio::test]
async fn local_workbench_hello_world_proof_replays_after_browser_disconnect_for_all_provider_kinds()
{
    for provider_kind in [
        ApplicationExecutionProviderKind::MacacaHosted,
        ApplicationExecutionProviderKind::ExternalAppBackend,
        ApplicationExecutionProviderKind::RemoteAgent,
    ] {
        let proof = WorkbenchProofFixture::new(provider_kind);
        proof.run_hello_world_task_with_reconnect().await;
    }
}

/// Test-level proof harness that keeps application details out of OS layers.
///
/// The fixture acts like an application-owned Workbench scenario while relying
/// on provider-neutral protocol events. It writes files only under
/// `workspace_root`, then records relative paths and bounded summaries in
/// EventLog so replay evidence remains audit-safe.
struct WorkbenchProofFixture {
    _dir: TempDir,
    workspace_root: PathBuf,
    event_log: Arc<EventLog>,
    provider_kind: ApplicationExecutionProviderKind,
    provider_id: String,
    scope: ApplicationExecutionScope,
}

impl WorkbenchProofFixture {
    /// Build a proof fixture with an isolated durable EventLog and workspace.
    fn new(provider_kind: ApplicationExecutionProviderKind) -> Self {
        let dir = tempdir().expect("proof tempdir should be available");
        let workspace_root = dir.path().join("session-workspace");
        fs::create_dir_all(&workspace_root).expect("session workspace should be created");
        let redb =
            Arc::new(RedbStore::open(dir.path().join("application-execution-proof.redb")).unwrap());
        let event_log = Arc::new(EventLog::new(redb));
        let provider_id = match provider_kind {
            ApplicationExecutionProviderKind::MacacaHosted => "proof-provider-macaca-hosted",
            ApplicationExecutionProviderKind::ExternalAppBackend => {
                "proof-provider-external-backend"
            }
            ApplicationExecutionProviderKind::RemoteAgent => "proof-provider-remote-agent",
            ApplicationExecutionProviderKind::Unavailable => "proof-provider-unavailable",
        }
        .to_string();
        let scope = ApplicationExecutionScope {
            application_id: ApplicationId::from_name("application-execution-workbench-proof"),
            session_id: format!("proof-session-{provider_id}"),
            run_id: format!("proof-run-{provider_id}"),
            tenant_id: None,
            actor: "application-execution-proof".into(),
        };
        Self {
            _dir: dir,
            workspace_root,
            event_log,
            provider_kind,
            provider_id,
            scope,
        }
    }

    /// Execute the local proof and verify deterministic replay after reconnect.
    async fn run_hello_world_task_with_reconnect(&self) {
        let store = ApplicationExecutionEventStore::new(Arc::clone(&self.event_log));

        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::ProviderAssigned,
            "provider assigned for local Workbench proof",
            "provider-assigned",
            None,
        )
        .await;
        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::ExecutionAccepted,
            "frontend plus backend Hello World task accepted",
            "execution-accepted",
            Some(serde_json::json!({
                "workspace_root_ref": "session-workspace",
                "task_shape": "frontend_plus_backend_hello_world"
            })),
        )
        .await;
        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::LlmCompleted,
            "LLM planning step completed with sanitized implementation outline",
            "llm-completed",
            Some(serde_json::json!({
                "model_ref": "local-proof-model",
                "prompt_redaction": "summary_only"
            })),
        )
        .await;

        // Dropping the receiver simulates a browser tab or iframe subscriber
        // disconnecting. Provider-owned execution continues because EventLog is
        // the authority, not the browser stream.
        let browser_subscriber = self.event_log.subscribe();
        drop(browser_subscriber);

        let frontend_path =
            self.write_workspace_file("frontend/index.html", "<main><h1>Hello World</h1></main>");
        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::ToolCallCompleted,
            "file service wrote frontend artifact inside session workspace",
            "tool-file-frontend",
            Some(serde_json::json!({
                "tool_family": "file",
                "workspace_relative_path": relative_path(&self.workspace_root, &frontend_path),
                "write_policy": "session_workspace_only"
            })),
        )
        .await;

        let backend_path = self.write_workspace_file(
            "backend/server.js",
            "export function handler() { return 'Hello World'; }\n",
        );
        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::ToolCallCompleted,
            "file service wrote backend artifact inside session workspace",
            "tool-file-backend",
            Some(serde_json::json!({
                "tool_family": "file",
                "workspace_relative_path": relative_path(&self.workspace_root, &backend_path),
                "write_policy": "session_workspace_only"
            })),
        )
        .await;

        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::ToolCallCompleted,
            "process service validated generated frontend/backend artifacts",
            "tool-process-test",
            Some(serde_json::json!({
                "tool_family": "process",
                "exit_code": 0,
                "stdout_ref": "sanitized-process-summary"
            })),
        )
        .await;
        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::CheckpointCreated,
            "checkpoint persisted before reconnect proof",
            "checkpoint",
            Some(serde_json::json!({
                "checkpoint_ref": format!("checkpoint://{}", self.scope.run_id)
            })),
        )
        .await;
        append_proof_event(
            &store,
            &self.scope,
            &self.provider_id,
            self.provider_kind,
            ApplicationExecutionEventType::ExecutionCompleted,
            "Hello World proof completed through generic application execution protocol",
            "completed",
            Some(serde_json::json!({
                "result_ref": "sanitized-hello-world-result",
                "provider_kind": format!("{:?}", self.provider_kind)
            })),
        )
        .await;

        let reconnect_store = ApplicationExecutionEventStore::new(Arc::clone(&self.event_log));
        let replay = reconnect_store
            .replay(ApplicationExecutionReplayRequest {
                application_id: self.scope.application_id,
                session_id: self.scope.session_id.clone(),
                run_id: Some(self.scope.run_id.clone()),
                from_cursor: None,
                page_size: 100,
                event_types: Vec::new(),
                visibility: None,
                trace: TraceContext::new(format!("trace-reconnect-{}", self.provider_id)),
            })
            .await
            .expect("replay after reconnect should succeed");
        let state = replay
            .current_state
            .expect("replay should rebuild current state");
        assert!(state.lifecycle_state.is_terminal());
        assert_eq!(state.provider_kind, self.provider_kind);
        assert_eq!(state.step_summaries.len(), 4);
        assert!(state
            .terminal_result
            .expect("completed proof should carry terminal result")
            .summary
            .contains("Hello World proof completed"));
        assert!(frontend_path.starts_with(&self.workspace_root));
        assert!(backend_path.starts_with(&self.workspace_root));
    }

    /// Write one file under the session workspace and reject escaping paths.
    fn write_workspace_file(&self, relative: &str, contents: &str) -> PathBuf {
        let target = self.workspace_root.join(relative);
        assert!(
            target.starts_with(&self.workspace_root),
            "proof writes must remain session-scoped"
        );
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("proof parent directory should be created");
        }
        fs::write(&target, contents).expect("proof file should be written");
        target
    }
}

/// Append one sanitized protocol event to the durable proof EventLog.
async fn append_proof_event(
    store: &ApplicationExecutionEventStore,
    scope: &ApplicationExecutionScope,
    provider_id: &str,
    provider_kind: ApplicationExecutionProviderKind,
    event_type: ApplicationExecutionEventType,
    summary: &str,
    idempotency_suffix: &str,
    data: Option<serde_json::Value>,
) {
    let event = ApplicationExecutionEventEnvelope {
        application_id: scope.application_id,
        session_id: scope.session_id.clone(),
        run_id: scope.run_id.clone(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type,
        trace: TraceContext::new(format!("trace-{provider_id}-{idempotency_suffix}")),
        actor: scope.actor.clone(),
        provider_id: provider_id.into(),
        provider_kind,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: ApplicationExecutionPayload {
            summary: summary.into(),
            data,
            payload_ref: None,
            truncated: false,
        },
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key: format!("{}-{idempotency_suffix}", scope.run_id),
    };
    store
        .append_idempotent(event)
        .await
        .expect("proof event append should succeed");
}

/// Return an audit-safe workspace relative path for proof evidence.
fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("proof path should be session-relative")
        .to_string_lossy()
        .to_string()
}
