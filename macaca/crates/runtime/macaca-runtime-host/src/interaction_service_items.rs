//! Item and snapshot handlers for `service.interaction`.
//!
//! Items are append-only Memento records for user input, agent output, tool
//! events, diagnostics, and other generic interaction facts.  Large or
//! sensitive payloads are represented by bounded summaries and artifact refs so
//! logs, snapshots, and shell streams stay auditable without exposing raw data.

use macaca_proto::workbench::interaction::*;
use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, WorkbenchCommandStatus};
use uuid::Uuid;

use crate::interaction_service_provider::{
    decode, persist_error, InteractionSystemServiceProvider,
};

impl InteractionSystemServiceProvider {
    pub(crate) async fn item_append(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ItemAppendRequest> = decode(payload)?;
        let store = self.store()?;
        Self::ensure_thread(&store, &typed.scope.session_id, &typed.payload.thread_id).await?;
        let summary = Self::artifact_backed_summary(typed.payload.summary, typed.payload.sensitive);
        let artifact_ref = summary.artifact_ref.clone();
        let now = chrono::Utc::now();
        let event_ref = self
            .emit(
                "interaction.item.appended",
                &typed.trace,
                &typed.scope.session_id,
                Some(&typed.payload.thread_id),
                typed.payload.turn_id.as_deref(),
                None,
            )
            .await;
        let record = InteractionItemRecord {
            item_id: format!("item-{}", Uuid::new_v4()),
            session_id: typed.scope.session_id.clone(),
            thread_id: typed.payload.thread_id,
            turn_id: typed.payload.turn_id,
            kind: typed.payload.kind,
            status: InteractionItemStatus::Pending,
            summary,
            artifact_ref,
            event_refs: vec![event_ref.clone()],
            created_at: now,
            updated_at: now,
        };
        store.append_item(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Item(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn item_status(
        &self,
        payload: serde_json::Value,
        status: InteractionItemStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ItemRefRequest> = decode(payload)?;
        let store = self.store()?;
        let mut record = store
            .get_item(&typed.scope.session_id, &typed.payload.item_id)
            .await
            .map_err(persist_error)?
            .ok_or_else(|| {
                ServiceError::InvalidArgument(format!("unknown item '{}'", typed.payload.item_id))
            })?;
        record.status = status.clone();
        record.updated_at = chrono::Utc::now();
        let event_ref = self
            .emit(
                &format!("interaction.item.{:?}", status).to_lowercase(),
                &typed.trace,
                &typed.scope.session_id,
                Some(&record.thread_id),
                record.turn_id.as_deref(),
                Some(&record.item_id),
            )
            .await;
        record.event_refs.push(event_ref.clone());
        store.save_item(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Item(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn item_list(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ItemListRequest> = decode(payload)?;
        let records = self
            .store()?
            .list_items(
                &typed.scope.session_id,
                &typed.payload.thread_id,
                typed.payload.since_index,
                typed.payload.limit.clamp(1, 500),
            )
            .await
            .map_err(persist_error)?;
        Self::result(
            InteractionResponse::Items(records),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn item_watch(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ItemListRequest> = decode(payload)?;
        let latest_index = self
            .store()?
            .list_items(
                &typed.scope.session_id,
                &typed.payload.thread_id,
                0,
                usize::MAX,
            )
            .await
            .map_err(persist_error)?
            .len();
        Self::result(
            InteractionResponse::Watch {
                watch_id: format!("watch-{}", Uuid::new_v4()),
                latest_index,
            },
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn snapshot(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<InteractionSnapshotRequest> = decode(payload)?;
        let store = self.store()?;
        let (threads, turns, items) = store
            .snapshot_counts(&typed.scope.session_id)
            .await
            .map_err(persist_error)?;
        let loaded_threads = store
            .loaded_threads(&typed.scope.session_id)
            .await
            .map_err(persist_error)?;
        Self::result(
            InteractionResponse::Snapshot(InteractionServiceSnapshot {
                threads,
                turns,
                items,
                loaded_threads,
                captured_at: chrono::Utc::now(),
            }),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }
}
