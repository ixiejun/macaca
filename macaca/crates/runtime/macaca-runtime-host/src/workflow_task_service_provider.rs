//! Optional runtime-host provider for workflow-task lifecycle observability.
//!
//! The adapter implements `SystemService` so task commands enter the canonical
//! runtime path. It deliberately maps only descriptor-declared command names to
//! fixed lifecycle event kinds; task payload parsing and concrete task storage
//! remain replaceable provider responsibilities.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::workflow_task::{
    WorkflowTaskState, WORKFLOW_TASK_COMMANDS, WORKFLOW_TASK_PACK_ID, WORKFLOW_TASK_SERVICE_ID,
};
use macaca_proto::domain_pack_contract::workflow_task_lifecycle_event::{
    WorkflowTaskLifecycleEvent, WorkflowTaskLifecycleEventKind,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tokio::sync::broadcast;
use tracing::{info, warn};

/// Task provider that publishes sanitized lifecycle facts through an Observer channel.
pub struct WorkflowTaskLifecycleSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<WorkflowTaskLifecycleEvent>,
    unavailable_reason: Option<String>,
}

impl WorkflowTaskLifecycleSystemServiceProvider {
    /// Build a deterministic provider for composition roots and conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build a fail-closed provider for optional-module absence diagnostics.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: workflow_task_service_descriptor(),
            events,
            unavailable_reason,
        }
    }

    /// Subscribe to reference-only lifecycle events for audit and shell observers.
    pub fn subscribe(&self) -> broadcast::Receiver<WorkflowTaskLifecycleEvent> {
        self.events.subscribe()
    }

    fn events_for(
        &self,
        command: &ServiceCommand,
        trace_id: String,
    ) -> ServiceResult<Vec<WorkflowTaskLifecycleEvent>> {
        event_kinds(command.name.as_str())
            .ok_or_else(|| ServiceError::UnsupportedCommand(command.name.to_string()))
            .map(|kinds| {
                kinds
                    .iter()
                    .map(|kind| WorkflowTaskLifecycleEvent {
                        kind: *kind,
                        trace_id: trace_id.clone(),
                        task_ref: "task:provider-reference".into(),
                        state: state_for(*kind),
                        version_ref: "version:provider-reference".into(),
                        queue_ref_hash: "hash:queue-reference".into(),
                        attempt_index: 0,
                        replay_ref: format!("replay:{trace_id}"),
                    })
                    .collect()
            })
    }
}

#[async_trait]
impl SystemService for WorkflowTaskLifecycleSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "workflow task lifecycle provider started");
        Ok(())
    }

    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "workflow task lifecycle provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        let events = self.events_for(&command, trace.trace_id.clone())?;
        if events.iter().any(|event| !event.is_trace_safe()) {
            return Err(ServiceError::AdapterFailure(
                "unsafe workflow task lifecycle event".into(),
            ));
        }
        for event in &events {
            let _ = self.events.send(event.clone());
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "workflow task lifecycle event published");
        Ok(domain_pack_service_result(
            serde_json::json!({"status": "ok", "event_count": events.len()}),
            trace,
            "mock",
        ))
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "workflow task lifecycle provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "workflow task lifecycle provider cleanup completed");
        Ok(())
    }
}

/// Build a descriptor whose commands are derived from the proto-owned workflow-task contract.
pub fn workflow_task_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(WORKFLOW_TASK_SERVICE_ID),
        ServiceType::new("workflow_task.lifecycle"),
        TraceSchemaRef::new("workflow.task.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), WORKFLOW_TASK_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        WORKFLOW_TASK_COMMANDS.len().to_string(),
    );
    descriptor
}

fn event_kinds(command: &str) -> Option<&'static [WorkflowTaskLifecycleEventKind]> {
    use WorkflowTaskLifecycleEventKind as Kind;
    Some(match command {
        "workflow_task.create" => &[Kind::Created],
        "workflow_task.enqueue" => &[Kind::Enqueued],
        "workflow_task.claim" => &[Kind::Claimed],
        "workflow_task.heartbeat" => &[Kind::HeartbeatRecorded],
        "workflow_task.release" => &[Kind::LeaseRevoked],
        "workflow_task.record_progress" => &[Kind::ProgressRecorded],
        "workflow_task.record_checkpoint" => &[Kind::CheckpointRecorded],
        "workflow_task.attach_artifact" => &[Kind::ArtifactAttached],
        "workflow_task.complete" => &[Kind::Completed],
        "workflow_task.fail" => &[Kind::Failed, Kind::RetryScheduled],
        "workflow_task.cancel" => &[Kind::Cancelled],
        "workflow_task.skip" => &[Kind::Skipped],
        "workflow_task.snapshot" => &[Kind::SnapshotRecorded],
        "workflow_task.inspect_provider" => &[Kind::PackDeclared],
        "workflow_task.update"
        | "workflow_task.patch_metadata"
        | "workflow_task.get"
        | "workflow_task.list"
        | "workflow_task.get_history" => &[Kind::AdmissionValidated],
        _ => return None,
    })
}

fn state_for(kind: WorkflowTaskLifecycleEventKind) -> WorkflowTaskState {
    match kind {
        WorkflowTaskLifecycleEventKind::Completed => WorkflowTaskState::Completed,
        WorkflowTaskLifecycleEventKind::Failed => WorkflowTaskState::Failed,
        WorkflowTaskLifecycleEventKind::Cancelled => WorkflowTaskState::Cancelled,
        WorkflowTaskLifecycleEventKind::Skipped => WorkflowTaskState::Skipped,
        WorkflowTaskLifecycleEventKind::Claimed
        | WorkflowTaskLifecycleEventKind::HeartbeatRecorded => WorkflowTaskState::Claimed,
        WorkflowTaskLifecycleEventKind::Enqueued
        | WorkflowTaskLifecycleEventKind::RetryScheduled => WorkflowTaskState::Queued,
        _ => WorkflowTaskState::Running,
    }
}

/// Retain this import in the module API so composition roots can store the provider behind Arc.
pub type SharedWorkflowTaskLifecycleProvider = Arc<WorkflowTaskLifecycleSystemServiceProvider>;
