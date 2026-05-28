//! Thread command handlers for `service.interaction`.
//!
//! The provider keeps thread lifecycle transitions in this focused module so
//! the runtime-host Adapter remains small.  These handlers implement the State
//! pattern over provider-neutral records and persist every transition before a
//! shell or SDK can observe the result.

use macaca_proto::workbench::interaction::*;
use macaca_proto::{ServiceCallResult, ServiceResult, WorkbenchCommandStatus};
use tracing::info;
use uuid::Uuid;

use crate::interaction_service_provider::{
    decode, persist_error, InteractionSystemServiceProvider,
};

impl InteractionSystemServiceProvider {
    pub(crate) async fn thread_start(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadStartRequest> = decode(payload)?;
        let store = self.store()?;
        let now = chrono::Utc::now();
        let event_ref = self
            .emit(
                "interaction.thread.started",
                &typed.trace,
                &typed.scope.session_id,
                None,
                None,
                None,
            )
            .await;
        let record = InteractionThreadRecord {
            thread_id: format!("thread-{}", Uuid::new_v4()),
            scope: typed.scope.clone(),
            status: InteractionThreadStatus::Active,
            title: typed.payload.title,
            source_thread_id: None,
            rollback_boundary_item_id: None,
            event_refs: vec![event_ref.clone()],
            created_at: now,
            updated_at: now,
        };
        store.save_thread(&record).await.map_err(persist_error)?;
        info!(
            thread_id = %record.thread_id,
            trace_id = %typed.trace.trace_id,
            "interaction thread persisted"
        );
        Self::result(
            InteractionResponse::Thread(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn thread_read(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadRefRequest> = decode(payload)?;
        let store = self.store()?;
        let record =
            Self::ensure_thread(&store, &typed.scope.session_id, &typed.payload.thread_id).await?;
        Self::result(
            InteractionResponse::Thread(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn thread_fork(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadForkRequest> = decode(payload)?;
        let store = self.store()?;
        let source = Self::ensure_thread(
            &store,
            &typed.scope.session_id,
            &typed.payload.source_thread_id,
        )
        .await?;
        let now = chrono::Utc::now();
        let event_ref = self
            .emit(
                "interaction.thread.forked",
                &typed.trace,
                &typed.scope.session_id,
                Some(&source.thread_id),
                None,
                None,
            )
            .await;
        let record = InteractionThreadRecord {
            thread_id: format!("thread-{}", Uuid::new_v4()),
            scope: typed.scope.clone(),
            status: InteractionThreadStatus::Active,
            title: typed.payload.title.or(source.title),
            source_thread_id: Some(source.thread_id),
            rollback_boundary_item_id: None,
            event_refs: vec![event_ref.clone()],
            created_at: now,
            updated_at: now,
        };
        store.save_thread(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Thread(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn thread_status(
        &self,
        payload: serde_json::Value,
        archive: bool,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadRefRequest> = decode(payload)?;
        let store = self.store()?;
        let mut record =
            Self::ensure_thread(&store, &typed.scope.session_id, &typed.payload.thread_id).await?;
        record.status = if archive {
            InteractionThreadStatus::Archived
        } else {
            InteractionThreadStatus::Active
        };
        record.updated_at = chrono::Utc::now();
        let event_name = if archive {
            "interaction.thread.archived"
        } else {
            "interaction.thread.unarchived"
        };
        let event_ref = self
            .emit(
                event_name,
                &typed.trace,
                &typed.scope.session_id,
                Some(&record.thread_id),
                None,
                None,
            )
            .await;
        record.event_refs.push(event_ref.clone());
        store.save_thread(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Thread(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn thread_rollback(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadRollbackRequest> = decode(payload)?;
        let store = self.store()?;
        let mut record =
            Self::ensure_thread(&store, &typed.scope.session_id, &typed.payload.thread_id).await?;
        record.status = InteractionThreadStatus::RolledBack;
        record.rollback_boundary_item_id = typed.payload.boundary_item_id;
        record.updated_at = chrono::Utc::now();
        let event_ref = self
            .emit(
                "interaction.thread.rolled_back",
                &typed.trace,
                &typed.scope.session_id,
                Some(&record.thread_id),
                None,
                None,
            )
            .await;
        record.event_refs.push(event_ref.clone());
        store.save_thread(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Thread(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn thread_list(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadListRequest> = decode(payload)?;
        let records = self
            .store()?
            .list_threads(
                &typed.scope.session_id,
                typed.payload.include_archived,
                typed.payload.limit.clamp(1, 200),
            )
            .await
            .map_err(persist_error)?;
        Self::result(
            InteractionResponse::Threads(records),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn loaded_threads(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ThreadListRequest> = decode(payload)?;
        let ids = self
            .store()?
            .loaded_threads(&typed.scope.session_id)
            .await
            .map_err(persist_error)?;
        let records = ids
            .into_iter()
            .map(|thread_id| InteractionThreadRecord {
                thread_id,
                scope: typed.scope.clone(),
                status: InteractionThreadStatus::Active,
                title: None,
                source_thread_id: None,
                rollback_boundary_item_id: None,
                event_refs: Vec::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .collect();
        Self::result(
            InteractionResponse::Threads(records),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }
}
