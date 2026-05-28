use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_persist::RedbStore;
use macaca_proto::workbench::interaction::*;
use macaca_proto::{
    ApplicationId, BoundedSummary, ServiceCommand, TraceContext, WorkbenchCommandScope,
};
use tempfile::tempdir;

use crate::{
    interaction_service_command, InteractionLedgerStore, InteractionSystemServiceProvider,
    PersistInteractionLedgerStore,
};

fn store() -> (Arc<dyn InteractionLedgerStore>, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("interaction.redb")).unwrap());
    let store: Arc<dyn InteractionLedgerStore> = Arc::new(PersistInteractionLedgerStore::new(redb));
    (store, dir)
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
async fn turn_and_item_state_transitions_are_persisted() {
    let (store, _dir) = store();
    let provider = InteractionSystemServiceProvider::new(store);

    let started: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_START_COMMAND,
            ThreadStartRequest { title: None },
            "trace-thread",
        ),
    )
    .await;
    let thread = match started.output.unwrap() {
        InteractionResponse::Thread(record) => record,
        _ => panic!("expected thread response"),
    };

    let turn: InteractionCommandResult = call(
        &provider,
        command(
            TURN_START_COMMAND,
            TurnStartRequest {
                thread_id: thread.thread_id.clone(),
            },
            "trace-turn",
        ),
    )
    .await;
    let turn = match turn.output.unwrap() {
        InteractionResponse::Turn(record) => record,
        _ => panic!("expected turn response"),
    };

    let interrupted: InteractionCommandResult = call(
        &provider,
        command(
            TURN_INTERRUPT_COMMAND,
            TurnRefRequest {
                thread_id: thread.thread_id.clone(),
                turn_id: turn.turn_id.clone(),
                reason: Some("operator interrupt".into()),
            },
            "trace-interrupt",
        ),
    )
    .await;
    assert!(matches!(
        interrupted.output.unwrap(),
        InteractionResponse::Turn(record)
            if record.status == InteractionTurnStatus::Interrupted
                && record.reason == Some("operator interrupt".into())
    ));

    let second_turn: InteractionCommandResult = call(
        &provider,
        command(
            TURN_START_COMMAND,
            TurnStartRequest {
                thread_id: thread.thread_id.clone(),
            },
            "trace-second-turn",
        ),
    )
    .await;
    let second_turn = match second_turn.output.unwrap() {
        InteractionResponse::Turn(record) => record,
        _ => panic!("expected turn response"),
    };

    let item: InteractionCommandResult = call(
        &provider,
        command(
            ITEM_APPEND_COMMAND,
            ItemAppendRequest {
                thread_id: thread.thread_id.clone(),
                turn_id: Some(second_turn.turn_id.clone()),
                kind: InteractionItemKind::AgentOutput,
                summary: BoundedSummary {
                    text: "bounded output".into(),
                    truncated: false,
                    original_byte_len: Some(14),
                    artifact_ref: None,
                },
                sensitive: false,
            },
            "trace-item",
        ),
    )
    .await;
    let item = match item.output.unwrap() {
        InteractionResponse::Item(record) => record,
        _ => panic!("expected item response"),
    };

    let completed_item: InteractionCommandResult = call(
        &provider,
        command(
            ITEM_COMPLETE_COMMAND,
            ItemRefRequest {
                thread_id: thread.thread_id.clone(),
                item_id: item.item_id.clone(),
                reason: None,
            },
            "trace-item-complete",
        ),
    )
    .await;
    assert!(matches!(
        completed_item.output.unwrap(),
        InteractionResponse::Item(record) if record.status == InteractionItemStatus::Completed
    ));

    let completed_turn: InteractionCommandResult = call(
        &provider,
        command(
            TURN_COMPLETE_COMMAND,
            TurnRefRequest {
                thread_id: thread.thread_id.clone(),
                turn_id: second_turn.turn_id.clone(),
                reason: Some("done".into()),
            },
            "trace-turn-complete",
        ),
    )
    .await;
    assert!(matches!(
        completed_turn.output.unwrap(),
        InteractionResponse::Turn(record)
            if record.status == InteractionTurnStatus::Completed
                && record.reason == Some("done".into())
    ));

    let failed_turn: InteractionCommandResult = call(
        &provider,
        command(
            TURN_FAIL_COMMAND,
            TurnRefRequest {
                thread_id: thread.thread_id.clone(),
                turn_id: turn.turn_id.clone(),
                reason: Some("failure evidence".into()),
            },
            "trace-turn-fail",
        ),
    )
    .await;
    assert!(matches!(
        failed_turn.output.unwrap(),
        InteractionResponse::Turn(record) if record.status == InteractionTurnStatus::Failed
    ));
}

#[tokio::test]
async fn watch_snapshot_and_loaded_threads_are_replayable() {
    let (store, _dir) = store();
    let provider = InteractionSystemServiceProvider::new(store);

    let started: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_START_COMMAND,
            ThreadStartRequest {
                title: Some("Replayable thread".into()),
            },
            "trace-watch-thread",
        ),
    )
    .await;
    let thread = match started.output.unwrap() {
        InteractionResponse::Thread(record) => record,
        _ => panic!("expected thread response"),
    };

    for index in 0..3 {
        let _: InteractionCommandResult = call(
            &provider,
            command(
                ITEM_APPEND_COMMAND,
                ItemAppendRequest {
                    thread_id: thread.thread_id.clone(),
                    turn_id: None,
                    kind: InteractionItemKind::Other("generic-test".into()),
                    summary: BoundedSummary {
                        text: format!("item-{index}"),
                        truncated: false,
                        original_byte_len: Some(6),
                        artifact_ref: None,
                    },
                    sensitive: false,
                },
                &format!("trace-item-{index}"),
            ),
        )
        .await;
    }

    let watch: InteractionCommandResult = call(
        &provider,
        command(
            ITEM_WATCH_COMMAND,
            ItemListRequest {
                thread_id: thread.thread_id.clone(),
                since_index: 0,
                limit: 10,
            },
            "trace-watch",
        ),
    )
    .await;
    assert!(matches!(
        watch.output.unwrap(),
        InteractionResponse::Watch { latest_index, .. } if latest_index == 3
    ));

    let snapshot: InteractionCommandResult = call(
        &provider,
        command(
            SNAPSHOT_COMMAND,
            InteractionSnapshotRequest { limit: 100 },
            "trace-snapshot",
        ),
    )
    .await;
    assert!(matches!(
        snapshot.output.unwrap(),
        InteractionResponse::Snapshot(state)
            if state.threads == 1 && state.items == 3
                && state.loaded_threads == vec![thread.thread_id.clone()]
    ));

    let loaded: InteractionCommandResult = call(
        &provider,
        command(
            THREAD_LOADED_LIST_COMMAND,
            ThreadListRequest {
                include_archived: false,
                limit: 10,
            },
            "trace-loaded",
        ),
    )
    .await;
    assert!(matches!(
        loaded.output.unwrap(),
        InteractionResponse::Threads(records)
            if records.iter().any(|record| record.thread_id == thread.thread_id)
    ));
}
