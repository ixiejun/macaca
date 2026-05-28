use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::workbench::interaction::*;
use macaca_proto::{
    ApplicationId, BoundedSummary, ServiceCommand, ServiceHealth, TraceContext,
    WorkbenchCommandScope, WorkbenchCommandStatus,
};
use tempfile::tempdir;

use crate::{
    interaction_service_command, InteractionLedgerStore, InteractionSystemServiceProvider,
    PersistInteractionLedgerStore,
};

fn store() -> (
    Arc<dyn InteractionLedgerStore>,
    Arc<EventLog>,
    tempfile::TempDir,
) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("interaction.redb")).unwrap());
    let event_log = Arc::new(EventLog::new(Arc::clone(&redb)));
    let store: Arc<dyn InteractionLedgerStore> = Arc::new(PersistInteractionLedgerStore::new(redb));
    (store, event_log, dir)
}

fn scope() -> WorkbenchCommandScope {
    WorkbenchCommandScope::new(
        ApplicationId::from_name("generic-interaction-app"),
        "session-a",
    )
    .unwrap()
}

fn command<T: serde::Serialize>(name: &str, payload: T, trace_id: &str) -> ServiceCommand {
    interaction_service_command(
        name,
        InteractionCommand::new(scope(), TraceContext::new(trace_id), trace_id, payload).unwrap(),
    )
    .unwrap()
}

async fn call<T: serde::de::DeserializeOwned>(
    provider: &InteractionSystemServiceProvider,
    command: ServiceCommand,
) -> T {
    let result = provider.call(command).await.unwrap();
    serde_json::from_value(result.output).unwrap()
}

#[tokio::test]
async fn interaction_provider_replays_thread_turn_item_lifecycle() {
    let (store, event_log, _dir) = store();
    let provider = InteractionSystemServiceProvider::with_event_log(store, Some(event_log.clone()));
    provider.start().await.unwrap();
    let mut events = provider.subscribe();

    let started: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_START_COMMAND,
            ThreadStartRequest {
                title: Some("Generic work".into()),
            },
            "trace-thread-start",
        ),
    )
    .await;
    let thread = match started.output.unwrap() {
        InteractionResponse::Thread(record) => record,
        _ => panic!("expected thread response"),
    };
    assert!(matches!(started.status, WorkbenchCommandStatus::Completed));

    let turn: InteractionCommandResult = call(
        &provider,
        command(
            TURN_START_COMMAND,
            TurnStartRequest {
                thread_id: thread.thread_id.clone(),
            },
            "trace-turn-start",
        ),
    )
    .await;
    let turn = match turn.output.unwrap() {
        InteractionResponse::Turn(record) => record,
        _ => panic!("expected turn response"),
    };

    let item: InteractionCommandResult = call(
        &provider,
        command(
            ITEM_APPEND_COMMAND,
            ItemAppendRequest {
                thread_id: thread.thread_id.clone(),
                turn_id: Some(turn.turn_id.clone()),
                kind: InteractionItemKind::UserInput,
                summary: BoundedSummary {
                    text: "raw sensitive material should not stay inline".into(),
                    truncated: false,
                    original_byte_len: Some(16 * 1024),
                    artifact_ref: None,
                },
                sensitive: true,
            },
            "trace-item-append",
        ),
    )
    .await;
    let item = match item.output.unwrap() {
        InteractionResponse::Item(record) => record,
        _ => panic!("expected item response"),
    };
    assert!(item.summary.truncated);
    assert!(item.artifact_ref.is_some());

    let listed: InteractionCommandResult = call(
        &provider,
        command(
            ITEM_LIST_COMMAND,
            ItemListRequest {
                thread_id: thread.thread_id.clone(),
                since_index: 0,
                limit: 10,
            },
            "trace-item-list",
        ),
    )
    .await;
    assert!(matches!(
        listed.output.unwrap(),
        InteractionResponse::Items(items) if items.len() == 1
    ));

    let resumed: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_RESUME_COMMAND,
            ThreadRefRequest {
                thread_id: thread.thread_id.clone(),
                reason: None,
            },
            "trace-thread-resume",
        ),
    )
    .await;
    assert!(matches!(
        resumed.output.unwrap(),
        InteractionResponse::Thread(record) if record.thread_id == thread.thread_id
    ));

    assert_eq!(
        event_log.query("session-a", 0, 20).await.len(),
        3,
        "mutations should mirror sanitized EventLog evidence"
    );
    assert_eq!(
        events.recv().await.unwrap().event_type,
        "interaction.thread.started"
    );
}

#[tokio::test]
async fn fork_rollback_archive_and_turn_states_are_replayable() {
    let (store, _event_log, _dir) = store();
    let provider = InteractionSystemServiceProvider::new(store);

    let started: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_START_COMMAND,
            ThreadStartRequest { title: None },
            "t1",
        ),
    )
    .await;
    let thread = match started.output.unwrap() {
        InteractionResponse::Thread(record) => record,
        _ => panic!("expected thread response"),
    };

    let forked: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_FORK_COMMAND,
            ThreadForkRequest {
                source_thread_id: thread.thread_id.clone(),
                title: Some("Fork".into()),
            },
            "t2",
        ),
    )
    .await;
    assert!(matches!(
        forked.output.unwrap(),
        InteractionResponse::Thread(record) if record.source_thread_id == Some(thread.thread_id.clone())
    ));

    let rolled_back: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_ROLLBACK_COMMAND,
            ThreadRollbackRequest {
                thread_id: thread.thread_id.clone(),
                boundary_item_id: Some("item-boundary".into()),
                reason: Some("test rollback".into()),
            },
            "t3",
        ),
    )
    .await;
    assert!(matches!(
        rolled_back.output.unwrap(),
        InteractionResponse::Thread(record)
            if record.status == InteractionThreadStatus::RolledBack
                && record.rollback_boundary_item_id == Some("item-boundary".into())
    ));

    let archived: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_ARCHIVE_COMMAND,
            ThreadRefRequest {
                thread_id: thread.thread_id.clone(),
                reason: None,
            },
            "t4",
        ),
    )
    .await;
    assert!(matches!(
        archived.output.unwrap(),
        InteractionResponse::Thread(record) if record.status == InteractionThreadStatus::Archived
    ));
}

#[tokio::test]
async fn unavailable_provider_reports_structured_health_and_error() {
    let provider = InteractionSystemServiceProvider::unavailable();

    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));

    let error = provider
        .call(command(
            THREAD_START_COMMAND,
            ThreadStartRequest { title: None },
            "trace-unavailable",
        ))
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("interaction ledger store"),
        "unavailable providers must fail explicitly instead of pretending success"
    );
}
