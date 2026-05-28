//! Support code for the Codex-class application-neutral proof.
//!
//! The helpers keep the executable proof readable while preserving the same
//! service boundary as production code: every side effect uses a SystemService
//! command with trace context and every large result is represented by bounded
//! summaries, audit refs, or artifacts.

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_persist::RedbStore;
use macaca_proto::workbench::{app_protocol as app, file, hook, interaction, process};
use macaca_proto::{
    ApplicationId, ApplicationManifestV1, BoundedSummary, ServiceCommand, ServiceCommandName,
    TraceContext, WorkbenchCommand, WorkbenchCommandResult, WorkbenchCommandScope,
    WorkbenchCommandStatus,
};
use macaca_runtime_host::{
    AppProtocolSystemServiceProvider, ApprovalSystemServiceProvider,
    CodeIntelligenceSystemServiceProvider, DiagnosticsSystemServiceProvider,
    FileSystemServiceProvider, GitSystemServiceProvider, HookSystemServiceProvider,
    InteractionLedgerStore, InteractionSystemServiceProvider, PersistInteractionLedgerStore,
    ProcessSystemServiceProvider, SandboxSystemServiceProvider,
};
use serde::de::DeserializeOwned;

const APP_SESSION: &str = "proof-session";

mod manifest;
mod steps;

pub use manifest::*;
pub use steps::*;

pub struct ProofProviders {
    pub app_protocol: AppProtocolSystemServiceProvider,
    pub interaction: InteractionSystemServiceProvider,
    pub file: FileSystemServiceProvider,
    pub code: CodeIntelligenceSystemServiceProvider,
    pub git: GitSystemServiceProvider,
    pub sandbox: SandboxSystemServiceProvider,
    pub process: ProcessSystemServiceProvider,
    pub approval: ApprovalSystemServiceProvider,
    pub hook: HookSystemServiceProvider,
    pub review: macaca_runtime_host::ReviewSystemServiceProvider,
    pub diagnostics: DiagnosticsSystemServiceProvider,
    _interaction_store_dir: tempfile::TempDir,
}

impl ProofProviders {
    /// Build provider-backed services with only test-owned fixture state.
    ///
    /// This is an Abstract Factory for the proof. It constructs real local
    /// providers where the proposal requires provider-backed behavior and keeps
    /// those providers out of kernel, SDK, shell, and application production
    /// code.
    pub fn new() -> Self {
        let (interaction, dir) = interaction_provider();
        Self {
            app_protocol: AppProtocolSystemServiceProvider::new(),
            interaction,
            file: FileSystemServiceProvider::local(),
            code: CodeIntelligenceSystemServiceProvider::mock(),
            git: GitSystemServiceProvider::mock(),
            sandbox: SandboxSystemServiceProvider::local(),
            process: ProcessSystemServiceProvider::local(),
            approval: ApprovalSystemServiceProvider::local(),
            hook: HookSystemServiceProvider::local(vec![managed_hook()]),
            review: macaca_runtime_host::ReviewSystemServiceProvider::mock(),
            diagnostics: DiagnosticsSystemServiceProvider::mock(),
            _interaction_store_dir: dir,
        }
    }
}

pub struct ProcessExecOutput {
    pub record: process::ProcessRecord,
}

pub fn proof_scope(manifest: &ApplicationManifestV1) -> WorkbenchCommandScope {
    WorkbenchCommandScope::new(
        ApplicationId::from_name(manifest.package_id.as_str()),
        APP_SESSION,
    )
    .unwrap()
}

pub fn proof_repo() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    run_git(repo.path(), &["init"]);
    run_git(
        repo.path(),
        &["config", "user.email", "proof@example.invalid"],
    );
    run_git(repo.path(), &["config", "user.name", "Macaca Proof"]);
    std::fs::write(repo.path().join("README.md"), "proof_marker: before\n").unwrap();
    run_git(repo.path(), &["add", "README.md"]);
    run_git(repo.path(), &["commit", "-m", "init"]);
    repo
}

pub fn proof_path(root: &std::path::Path, rel: &str) -> file::FilePathRequest {
    file::FilePathRequest {
        workspace_root: root.to_string_lossy().to_string(),
        path: rel.into(),
        follow_symlinks: false,
    }
}

pub fn proof_patch() -> String {
    "\
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-proof_marker: before
+proof_marker: after
"
    .into()
}

pub fn summary(text: &str) -> BoundedSummary {
    BoundedSummary {
        text: text.into(),
        truncated: false,
        original_byte_len: Some(text.len() as u64),
        artifact_ref: None,
    }
}

pub async fn call_service<T>(
    provider: &(impl SystemService + Sync),
    command_name: &str,
    payload: impl serde::Serialize,
    trace_id: impl Into<String>,
) -> WorkbenchCommandResult<T>
where
    T: DeserializeOwned,
{
    let output = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(command_name),
            serde_json::to_value(payload).unwrap(),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap()
        .output;
    serde_json::from_value(output).unwrap()
}

pub async fn initialize_app_protocol(
    provider: &AppProtocolSystemServiceProvider,
    scope: &WorkbenchCommandScope,
) {
    let result = call_protocol(
        provider,
        app::INITIALIZE_COMMAND,
        app::AppProtocolInitializeRequest {
            connection_id: Some("proof-connection".into()),
            protocol_version: "app-protocol.v1".into(),
            client: app::AppProtocolClientMetadata {
                client_name: "generic-proof-client".into(),
                client_version: Some("1.0.0".into()),
                transport: app::AppProtocolTransportKind::WebSocket,
                wants_notifications: true,
            },
            requested_capabilities: vec!["capability.app_protocol".into()],
        },
        scope.clone(),
        "trace-proof-app-protocol-init",
    )
    .await;
    assert!(matches!(
        result.output,
        Some(app::AppProtocolResponse::Initialized(_))
    ));
}

pub async fn subscribe_global_app_events(
    provider: &AppProtocolSystemServiceProvider,
    scope: &WorkbenchCommandScope,
) {
    let result = call_protocol(
        provider,
        app::SUBSCRIPTION_CREATE_COMMAND,
        app::AppProtocolSubscriptionCreateRequest {
            connection_id: "proof-connection".into(),
            target: app::AppProtocolSubscriptionTarget::Global,
            event_kinds: Vec::new(),
        },
        scope.clone(),
        "trace-proof-app-protocol-subscribe",
    )
    .await;
    assert!(matches!(
        result.output,
        Some(app::AppProtocolResponse::Subscription(_))
    ));
}

pub async fn stream_service_event(
    provider: &AppProtocolSystemServiceProvider,
    scope: &WorkbenchCommandScope,
    kind: &str,
    service_id: &str,
) {
    stream_event(provider, scope, kind, service_id, None, None).await;
}

pub async fn stream_event(
    provider: &AppProtocolSystemServiceProvider,
    scope: &WorkbenchCommandScope,
    kind: &str,
    service_id: &str,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
) {
    let result = call_protocol(
        provider,
        app::NOTIFICATION_EMIT_COMMAND,
        app::AppProtocolNotificationEmitRequest {
            event: app::AppProtocolEventEnvelope {
                event_kind: kind.into(),
                source_service_id: service_id.into(),
                thread_id: thread_id.map(str::to_string),
                turn_id: turn_id.map(str::to_string),
                item_id: None,
                trace_id: format!("trace-proof-stream-{kind}"),
                audit_ref: Some(format!("audit://proof/{kind}")),
                summary: Some(summary("bounded proof event")),
                emitted_at: chrono::Utc::now(),
            },
        },
        scope.clone(),
        format!("trace-proof-stream-{kind}"),
    )
    .await;
    assert!(matches!(
        result.output,
        Some(app::AppProtocolResponse::Notifications(_))
    ));
}

pub async fn start_thread(
    provider: &InteractionSystemServiceProvider,
    scope: &WorkbenchCommandScope,
    audit_refs: &mut Vec<String>,
) -> interaction::InteractionThreadRecord {
    let result = call_interaction(
        provider,
        interaction::THREAD_START_COMMAND,
        interaction::ThreadStartRequest {
            title: Some("Application-neutral proof".into()),
        },
        scope.clone(),
        "trace-proof-thread-start",
    )
    .await;
    push_audit(&result, audit_refs);
    match result.output {
        Some(interaction::InteractionResponse::Thread(thread)) => thread,
        _ => panic!("expected thread record"),
    }
}

pub async fn start_turn(
    provider: &InteractionSystemServiceProvider,
    scope: &WorkbenchCommandScope,
    thread_id: &str,
    audit_refs: &mut Vec<String>,
) -> interaction::InteractionTurnRecord {
    let result = call_interaction(
        provider,
        interaction::TURN_START_COMMAND,
        interaction::TurnStartRequest {
            thread_id: thread_id.into(),
        },
        scope.clone(),
        "trace-proof-turn-start",
    )
    .await;
    push_audit(&result, audit_refs);
    match result.output {
        Some(interaction::InteractionResponse::Turn(turn)) => turn,
        _ => panic!("expected turn record"),
    }
}

pub async fn append_interaction_item(
    provider: &InteractionSystemServiceProvider,
    scope: &WorkbenchCommandScope,
    thread_id: &str,
    turn_id: &str,
    kind: interaction::InteractionItemKind,
    text: &str,
    audit_refs: &mut Vec<String>,
) {
    let result = call_interaction(
        provider,
        interaction::ITEM_APPEND_COMMAND,
        interaction::ItemAppendRequest {
            thread_id: thread_id.into(),
            turn_id: Some(turn_id.into()),
            kind,
            summary: summary(text),
            sensitive: false,
        },
        scope.clone(),
        "trace-proof-interaction-item",
    )
    .await;
    push_audit(&result, audit_refs);
    assert!(matches!(
        result.output,
        Some(interaction::InteractionResponse::Item(_))
    ));
}

pub fn push_audit<T>(result: &WorkbenchCommandResult<T>, audit_refs: &mut Vec<String>) {
    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Completed | WorkbenchCommandStatus::PendingApproval { .. }
    ));
    if let Some(audit_ref) = &result.audit_ref {
        audit_refs.push(audit_ref.clone());
    }
}

fn interaction_provider() -> (InteractionSystemServiceProvider, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("interaction.redb")).unwrap());
    let store: Arc<dyn InteractionLedgerStore> = Arc::new(PersistInteractionLedgerStore::new(redb));
    (InteractionSystemServiceProvider::new(store), dir)
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git command failed: {args:?}");
}

fn managed_hook() -> hook::HookDescriptor {
    hook::HookDescriptor {
        hook_id: "hook.generic-proof".into(),
        stage: hook::HookStage::PreTool,
        source_class: hook::HookSourceClass::Managed,
        adapter_kind: hook::HookAdapterKind::Local,
        priority: 0,
        managed: true,
        enabled: true,
        scope: hook::HookScope::default(),
        default_action: hook::HookAction::Continue,
        feedback: Some(summary("managed proof hook continued")),
        trace_schema: "trace.service.hook.proof.v1".into(),
        metadata: BTreeMap::new(),
    }
}

async fn call_interaction(
    provider: &InteractionSystemServiceProvider,
    command_name: &str,
    payload: impl serde::Serialize,
    scope: WorkbenchCommandScope,
    trace_id: impl Into<String>,
) -> WorkbenchCommandResult<interaction::InteractionResponse> {
    let trace_id = trace_id.into();
    let command = WorkbenchCommand::new(
        scope,
        TraceContext::new(trace_id.clone()),
        trace_id.clone(),
        payload,
    )
    .unwrap();
    call_service(provider, command_name, command, trace_id).await
}

async fn call_protocol(
    provider: &AppProtocolSystemServiceProvider,
    command_name: &str,
    payload: impl serde::Serialize,
    scope: WorkbenchCommandScope,
    trace_id: impl Into<String>,
) -> WorkbenchCommandResult<app::AppProtocolResponse> {
    let trace_id = trace_id.into();
    let command = WorkbenchCommand::new(
        scope,
        TraceContext::new(trace_id.clone()),
        trace_id.clone(),
        payload,
    )
    .unwrap();
    call_service(provider, command_name, command, trace_id).await
}
