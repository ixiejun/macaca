//! Lifecycle memento operations for component-model sessions.
//!
//! **Pattern:** Memento — checkpoint, restore, upgrade, and rollback reports
//! capture portable lifecycle state without leaking engine internals.

use chrono::Utc;
use macaca_proto::{
    ApplicationAbiError, ApplicationAbiVersion, WasmCheckpointMemento, WasmLifecycleCommand,
    WasmLifecycleOperationStatus, WasmLifecycleReasonCode, WasmRestoreReport, WasmRestoreRequest,
    WasmRollbackReport, WasmRollbackRequest, WasmUpgradeReport, WasmUpgradeRequest,
};

use crate::wasm_runtime_provider::component_model::session::ComponentModelWasmExecutionSession;
use crate::wasm_runtime_provider::traits::WasmExecutionSession;

/// Lifecycle memento helpers for component-model execution sessions.
pub(super) trait ComponentModelWasmExecutionSessionLifecycle {
    async fn checkpoint_memento(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError>;

    async fn restore_from_checkpoint(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError>;

    async fn upgrade_artifact(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError>;

    async fn rollback_to_checkpoint(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError>;
}

impl ComponentModelWasmExecutionSessionLifecycle for ComponentModelWasmExecutionSession {
    async fn checkpoint_memento(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError> {
        let trace = command
            .require_trace()
            .map_err(|_| ApplicationAbiError::MissingTraceContext)?;
        Ok(WasmCheckpointMemento {
            checkpoint_id: format!("component-checkpoint-{}", self.session_id),
            session_id: self.session_id.clone(),
            application_id: self.request.application_id.clone(),
            ability_id: self.request.ability_id.clone(),
            lifecycle_state: self.lifecycle_state(),
            artifact: self.request.artifact.clone(),
            artifact_digest_prefix: self.artifact_digest.chars().take(12).collect(),
            abi_version: ApplicationAbiVersion::v0(),
            created_at: Utc::now(),
            trace: Some(trace),
            metadata: Default::default(),
        })
    }

    async fn restore_from_checkpoint(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError> {
        let compatible = request.checkpoint.artifact == self.request.artifact;
        Ok(WasmRestoreReport {
            status: if compatible {
                WasmLifecycleOperationStatus::Completed
            } else {
                WasmLifecycleOperationStatus::Rejected
            },
            reason_code: if compatible {
                "completed"
            } else {
                "abi_mismatch"
            }
            .into(),
            checkpoint_id: request.checkpoint.checkpoint_id,
            lifecycle_state: self.lifecycle_state(),
            trace: request.trace,
            metadata: Default::default(),
        })
    }

    async fn upgrade_artifact(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError> {
        let compatible = request.target_abi_version == ApplicationAbiVersion::v0();
        Ok(WasmUpgradeReport {
            status: if compatible {
                WasmLifecycleOperationStatus::Completed
            } else {
                WasmLifecycleOperationStatus::Rejected
            },
            reason_code: if compatible {
                "completed"
            } else {
                "abi_mismatch"
            }
            .into(),
            source_artifact: self.request.artifact.clone(),
            source_artifact_digest_prefix: self.artifact_digest.chars().take(12).collect(),
            target_artifact: request.target_artifact,
            target_artifact_digest_prefix: request
                .target_artifact_digest
                .chars()
                .take(12)
                .collect(),
            abi_compatible: compatible,
            trace: request.trace,
            metadata: Default::default(),
        })
    }

    async fn rollback_to_checkpoint(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError> {
        Ok(WasmRollbackReport {
            status: WasmLifecycleOperationStatus::Completed,
            reason_code: WasmLifecycleReasonCode::Completed.as_code().into(),
            checkpoint_id: request.checkpoint.checkpoint_id,
            restored_lifecycle_state: request.checkpoint.lifecycle_state,
            trace: request.trace,
            metadata: Default::default(),
        })
    }
}
