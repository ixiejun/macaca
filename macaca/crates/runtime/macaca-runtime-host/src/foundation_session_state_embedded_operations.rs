//! Persistence-backed command operations for the embedded session-state Strategy.
//!
//! Keeping the operation handlers separate from service dispatch makes the
//! provider easier to audit while preserving one runtime-host composition root.

use macaca_proto::{
    ServiceError, ServiceResult, SessionStateCheckpointRef, SessionStateClearSessionCommand,
    SessionStateCompactHistoryCommand, SessionStateCompareCheckpointCommand,
    SessionStateCreateCheckpointCommand, SessionStateDeleteCommand,
    SessionStateExportRedactedCommand, SessionStateGetCommand, SessionStateInspectRecoveryCommand,
    SessionStateKeyRef, SessionStateListCheckpointsCommand, SessionStateListKeysCommand,
    SessionStateMergePatchCommand, SessionStatePutCommand, SessionStateRecoveryMetadata,
    SessionStateRestoreCheckpointCommand, SessionStateSessionRef, SessionStateValueRef,
};

use super::{
    check_revision, hash, json_error, opaque_reference, revision, revision_id, storage_error,
    storage_key, validate_key, validate_session, DurableCheckpoint,
    EmbeddedFoundationSessionStateProvider, MAX_CHECKPOINTS, MAX_PAGE_SIZE,
};

impl EmbeddedFoundationSessionStateProvider {
    pub(super) async fn get(
        &self,
        request: SessionStateGetCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_key(&request.key)?;
        let record = self.load(&request.key.session).await?;
        let value = record.entries.get(&request.key.key);
        Ok(
            serde_json::json!({"status": if value.is_some() {"ok"} else {"not_found"}, "value_present": value.is_some(), "revision": revision(&record)}),
        )
    }

    pub(super) async fn put(
        &self,
        request: SessionStatePutCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_key(&request.key)?;
        if !request.value.is_admissible_reference() {
            return Err(ServiceError::AdapterFailure(
                "value must be an opaque artifact or secret reference".into(),
            ));
        }
        let mut record = self.load(&request.key.session).await?;
        check_revision(&record, request.expected_revision.as_ref())?;
        if record
            .entries
            .get(&request.key.key)
            .is_some_and(|existing| {
                existing.schema_id.is_some()
                    && request.value.schema_id.is_some()
                    && existing.schema_id != request.value.schema_id
            })
        {
            return Err(ServiceError::AdapterFailure("schema mismatch".into()));
        }
        record.entries.insert(request.key.key, request.value);
        record.revision = record.revision.saturating_add(1);
        let revision = revision(&record);
        self.save(&request.key.session, &record).await?;
        Ok(serde_json::json!({"status":"ok","revision":revision}))
    }

    pub(super) async fn delete(
        &self,
        request: SessionStateDeleteCommand,
    ) -> ServiceResult<serde_json::Value> {
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

    pub(super) async fn merge_patch(
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

    pub(super) async fn list_keys(
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

    pub(super) async fn create_checkpoint(
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

    pub(super) async fn list_checkpoints(
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

    pub(super) async fn restore(
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

    pub(super) async fn compare(
        &self,
        request: SessionStateCompareCheckpointCommand,
    ) -> ServiceResult<serde_json::Value> {
        Ok(
            serde_json::json!({"status":"ok","same_revision":request.left.revision_id == request.right.revision_id,"left":hash(&request.left.checkpoint_id),"right":hash(&request.right.checkpoint_id)}),
        )
    }

    pub(super) async fn compact(
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

    pub(super) async fn clear(
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

    pub(super) async fn export_redacted(
        &self,
        request: SessionStateExportRedactedCommand,
    ) -> ServiceResult<serde_json::Value> {
        validate_session(&request.session)?;
        let record = self.load(&request.session).await?;
        Ok(
            serde_json::json!({"status":"ok","redaction_level":request.redaction_level,"session_hash":hash(&request.session.session_id),"key_count":record.entries.len(),"revision":revision(&record)}),
        )
    }

    pub(super) async fn inspect(
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
