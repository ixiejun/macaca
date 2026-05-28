//! Turn command handlers for `service.interaction`.
//!
//! Turns are generic units of interactive progress.  They intentionally carry
//! status, reason, thread identity, and audit refs only; application products
//! define their own workflow semantics above this service boundary.

use macaca_proto::workbench::interaction::*;
use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, WorkbenchCommandStatus};
use uuid::Uuid;

use crate::interaction_service_provider::{
    decode, persist_error, InteractionSystemServiceProvider,
};

impl InteractionSystemServiceProvider {
    pub(crate) async fn turn_start(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<TurnStartRequest> = decode(payload)?;
        let store = self.store()?;
        Self::ensure_thread(&store, &typed.scope.session_id, &typed.payload.thread_id).await?;
        let now = chrono::Utc::now();
        let event_ref = self
            .emit(
                "interaction.turn.started",
                &typed.trace,
                &typed.scope.session_id,
                Some(&typed.payload.thread_id),
                None,
                None,
            )
            .await;
        let record = InteractionTurnRecord {
            turn_id: format!("turn-{}", Uuid::new_v4()),
            session_id: typed.scope.session_id.clone(),
            thread_id: typed.payload.thread_id,
            status: InteractionTurnStatus::Active,
            reason: None,
            event_refs: vec![event_ref.clone()],
            started_at: now,
            updated_at: now,
        };
        store.save_turn(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Turn(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn turn_status(
        &self,
        payload: serde_json::Value,
        status: InteractionTurnStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<TurnRefRequest> = decode(payload)?;
        let store = self.store()?;
        let mut record = store
            .get_turn(&typed.scope.session_id, &typed.payload.turn_id)
            .await
            .map_err(persist_error)?
            .ok_or_else(|| {
                ServiceError::InvalidArgument(format!("unknown turn '{}'", typed.payload.turn_id))
            })?;
        record.status = status.clone();
        record.reason = typed.payload.reason;
        record.updated_at = chrono::Utc::now();
        let event_ref = self
            .emit(
                &format!("interaction.turn.{:?}", status).to_lowercase(),
                &typed.trace,
                &typed.scope.session_id,
                Some(&record.thread_id),
                Some(&record.turn_id),
                None,
            )
            .await;
        record.event_refs.push(event_ref.clone());
        store.save_turn(&record).await.map_err(persist_error)?;
        Self::result(
            InteractionResponse::Turn(record),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            Some(event_ref),
        )
    }

    pub(crate) async fn turn_steer(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<ItemAppendRequest> = decode(payload)?;
        if let Some(turn_id) = &typed.payload.turn_id {
            let turn = self
                .store()?
                .get_turn(&typed.scope.session_id, turn_id)
                .await
                .map_err(persist_error)?
                .ok_or_else(|| {
                    ServiceError::InvalidArgument(format!("unknown turn '{turn_id}'"))
                })?;
            if turn.status != InteractionTurnStatus::Active {
                return Err(ServiceError::InvalidArgument(
                    "cannot steer a non-active turn".into(),
                ));
            }
        }
        self.item_append(
            serde_json::to_value(typed)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
        )
        .await
    }

    pub(crate) async fn turn_list(
        &self,
        payload: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: InteractionCommand<TurnListRequest> = decode(payload)?;
        let records = self
            .store()?
            .list_turns(
                &typed.scope.session_id,
                &typed.payload.thread_id,
                typed.payload.limit.clamp(1, 200),
            )
            .await
            .map_err(persist_error)?;
        Self::result(
            InteractionResponse::Turns(records),
            typed.trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }
}
