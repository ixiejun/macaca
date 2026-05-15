//! Built-in runtime execution-control capability.
//!
//! This module is the stage-1 in-process implementation behind the future
//! `service.execution_control` provider.  It owns generic execution state,
//! sanitized transition events, observer delivery, and memento-style snapshots;
//! it deliberately does not know application names, agent names, workflow
//! names, provider names, or shell channels.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use macaca_proto::{
    ExecutionControlAwaitResumeCommand, ExecutionControlCancelWaitCommand,
    ExecutionControlCheckpointCommand, ExecutionControlCommandResult, ExecutionControlEvent,
    ExecutionControlPauseCommand, ExecutionControlRegisterExecutionCommand,
    ExecutionControlResolutionStatus, ExecutionControlResumeCommand, ExecutionControlScope,
    ExecutionControlState, ExecutionControlStateCommand, MacacaError, MacacaResult,
};
use serde::{Deserialize, Serialize};

/// Observer boundary for trace, audit, EventLog, RunTrace, or diagnostics.
///
/// Stage 1 uses a synchronous observer so callers can record evidence before
/// emitting live UI updates.  Stage 2 can adapt this trait into service events
/// without changing the state machine.
pub trait ExecutionControlObserver: Send + Sync {
    /// Observe a sanitized execution-control transition.
    fn observe(&self, event: &ExecutionControlEvent);
}

/// Null Object observer used when no trace/audit sink has been attached yet.
#[derive(Default)]
pub struct NoopExecutionControlObserver;

impl ExecutionControlObserver for NoopExecutionControlObserver {
    fn observe(&self, _event: &ExecutionControlEvent) {}
}

/// Memento snapshot for one execution registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlExecutionSnapshot {
    pub scope: ExecutionControlScope,
    pub state: ExecutionControlState,
    pub reason_code: String,
    pub checkpoint_refs: Vec<String>,
    pub events: Vec<ExecutionControlEvent>,
}

/// Bounded memento snapshot for the whole built-in capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlRuntimeSnapshot {
    pub executions: Vec<ExecutionControlExecutionSnapshot>,
}

#[derive(Debug, Clone)]
struct ExecutionControlRecord {
    scope: ExecutionControlScope,
    status: ExecutionControlResolutionStatus,
    state: ExecutionControlState,
    reason_code: String,
    checkpoint_refs: Vec<String>,
    events: Vec<ExecutionControlEvent>,
}

/// Stage-1 runtime capability that executes typed execution-control commands.
///
/// The public methods mirror the future service command surface.  Keeping this
/// boundary command-shaped now makes the later service provider a thin adapter
/// instead of another owner of pause/resume semantics.
pub struct ExecutionControlRuntimeCapability {
    records: RwLock<BTreeMap<String, ExecutionControlRecord>>,
    observer: Arc<dyn ExecutionControlObserver>,
}

impl Default for ExecutionControlRuntimeCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionControlRuntimeCapability {
    /// Create the capability with a Null Object observer.
    pub fn new() -> Self {
        Self::with_observer(Arc::new(NoopExecutionControlObserver))
    }

    /// Create the capability with an observer for durable evidence sinks.
    pub fn with_observer(observer: Arc<dyn ExecutionControlObserver>) -> Self {
        Self {
            records: RwLock::new(BTreeMap::new()),
            observer,
        }
    }

    /// Register an execution after policy resolution and before side effects.
    pub fn register_execution(
        &self,
        command: ExecutionControlRegisterExecutionCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let status = command.policy.status.clone();
        let reason_code = command.policy.reason_code.clone();
        if status != ExecutionControlResolutionStatus::Enabled {
            return Ok(result(
                command.scope,
                status,
                ExecutionControlState::Completed,
                reason_code,
                Vec::new(),
            ));
        }

        let mut record = ExecutionControlRecord {
            scope: command.scope,
            status,
            state: ExecutionControlState::Running,
            reason_code,
            checkpoint_refs: Vec::new(),
            events: Vec::new(),
        };
        let event = event_for(
            &record.scope,
            ExecutionControlState::Running,
            "execution_control.execution_registered",
            &record.scope.metadata,
        );
        self.record_event(&mut record, event);
        let result = record.to_result();
        self.records
            .write()
            .expect("execution-control records lock poisoned")
            .insert(record.scope.execution_id.clone(), record);
        Ok(result)
    }

    /// Move a registered execution through pause-requested and paused states.
    pub fn request_pause(
        &self,
        command: ExecutionControlPauseCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let mut records = self
            .records
            .write()
            .expect("execution-control records lock poisoned");
        let record = find_record_mut(&mut records, &command.scope.execution_id)?;
        record.reason_code = non_empty_reason(command.reason_code)?;
        let mut metadata = command.scope.metadata.clone();
        metadata.insert("trigger".into(), format!("{:?}", command.trigger));
        let requested = event_for(
            &command.scope,
            ExecutionControlState::PauseRequested,
            "execution_control.pause_requested",
            &metadata,
        );
        self.record_event(record, requested);
        let entered = event_for(
            &command.scope,
            ExecutionControlState::Paused,
            "execution_control.pause_entered",
            &metadata,
        );
        self.record_event(record, entered);
        Ok(record.to_result())
    }

    /// Store a sanitized checkpoint reference for replay without raw payloads.
    pub fn record_checkpoint(
        &self,
        command: ExecutionControlCheckpointCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let mut records = self
            .records
            .write()
            .expect("execution-control records lock poisoned");
        let record = find_record_mut(&mut records, &command.scope.execution_id)?;
        let checkpoint_ref = sanitize_value(&command.checkpoint_ref);
        record.checkpoint_refs.push(checkpoint_ref.clone());
        let mut metadata = command.scope.metadata.clone();
        metadata.insert("checkpoint_ref".into(), checkpoint_ref);
        metadata.insert(
            "checkpoint_mode".into(),
            format!("{:?}", command.checkpoint_mode),
        );
        let event = event_for(
            &command.scope,
            record.state.clone(),
            "execution_control.checkpoint_recorded",
            &metadata,
        );
        self.record_event(record, event);
        Ok(record.to_result())
    }

    /// Record that a caller is now waiting for an allowed resume source.
    ///
    /// The built-in provider does not block inside this command. It records the
    /// wait intent as durable evidence and leaves the actual channel wait to the
    /// host adapter that owns the non-serializable receiver.
    pub fn await_resume(
        &self,
        command: ExecutionControlAwaitResumeCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let mut records = self
            .records
            .write()
            .expect("execution-control records lock poisoned");
        let record = find_record_mut(&mut records, &command.scope.execution_id)?;
        let mut metadata = command.scope.metadata.clone();
        metadata.insert(
            "resume_sources".into(),
            format!("{:?}", command.resume_sources),
        );
        if let Some(timeout_ms) = command.timeout_ms {
            metadata.insert("timeout_ms".into(), timeout_ms.to_string());
        }
        let event = event_for(
            &command.scope,
            record.state.clone(),
            "execution_control.resume_wait_registered",
            &metadata,
        );
        self.record_event(record, event);
        Ok(record.to_result())
    }

    /// Accept the first valid resume signal and transition the execution.
    pub fn request_resume(
        &self,
        command: ExecutionControlResumeCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let mut records = self
            .records
            .write()
            .expect("execution-control records lock poisoned");
        let record = find_record_mut(&mut records, &command.scope.execution_id)?;
        if matches!(
            record.state,
            ExecutionControlState::Resuming
                | ExecutionControlState::Completed
                | ExecutionControlState::Cancelled
                | ExecutionControlState::TimedOut
                | ExecutionControlState::Failed
        ) {
            let event = event_for(
                &command.scope,
                record.state.clone(),
                "execution_control.resume_rejected",
                &command.scope.metadata,
            );
            self.record_event(record, event);
            return Ok(record.to_result());
        }

        record.reason_code = non_empty_reason(command.reason_code)?;
        let mut metadata = command.scope.metadata.clone();
        metadata.insert(
            "resume_source".into(),
            format!("{:?}", command.resume_source),
        );
        let requested = event_for(
            &command.scope,
            ExecutionControlState::ResumeRequested,
            "execution_control.resume_requested",
            &metadata,
        );
        self.record_event(record, requested);
        let delivered = event_for(
            &command.scope,
            ExecutionControlState::Resuming,
            "execution_control.resume_delivered",
            &metadata,
        );
        self.record_event(record, delivered);
        Ok(record.to_result())
    }

    /// Cancel or time out an outstanding wait and make the result replayable.
    ///
    /// The public command remains `cancel_wait` because both outcomes end the
    /// local wait contract without delivering a resume signal.  A populated
    /// `timeout_ms` field classifies the terminal state as `TimedOut`, while a
    /// missing timeout keeps the older explicit-cancellation state.  This is a
    /// small State-pattern extension that avoids adding application-specific
    /// timeout branches in shell adapters.
    pub fn cancel_wait(
        &self,
        command: ExecutionControlCancelWaitCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let mut records = self
            .records
            .write()
            .expect("execution-control records lock poisoned");
        let record = find_record_mut(&mut records, &command.scope.execution_id)?;
        record.reason_code = non_empty_reason(command.reason_code)?;
        let mut metadata = command.scope.metadata.clone();
        let (state, reason_code) = if let Some(timeout_ms) = command.timeout_ms {
            metadata.insert("timeout_ms".into(), timeout_ms.to_string());
            (
                ExecutionControlState::TimedOut,
                "execution_control.wait_timed_out",
            )
        } else {
            (
                ExecutionControlState::Cancelled,
                "execution_control.wait_cancelled",
            )
        };
        let event = event_for(&command.scope, state, reason_code, &metadata);
        self.record_event(record, event);
        Ok(record.to_result())
    }

    /// Query the current state for diagnostics and record the read access.
    ///
    /// State queries are operationally important because shells and diagnostics
    /// can use them to explain why execution is paused, cancelled, or resuming.
    /// The query therefore emits a sanitized observer event even though it does
    /// not transition the business state.  This keeps audit replay complete
    /// without exposing raw command payloads or application-specific details.
    pub fn query_state(
        &self,
        command: ExecutionControlStateCommand,
    ) -> MacacaResult<ExecutionControlCommandResult> {
        let mut records = self
            .records
            .write()
            .expect("execution-control records lock poisoned");
        let record = find_record_mut(&mut records, &command.scope.execution_id)?;
        let event = event_for(
            &command.scope,
            record.state.clone(),
            "execution_control.state_queried",
            &command.scope.metadata,
        );
        self.record_event(record, event);
        Ok(record.to_result())
    }

    /// Return a bounded memento snapshot of all registered executions.
    pub fn snapshot(&self) -> ExecutionControlRuntimeSnapshot {
        let records = self
            .records
            .read()
            .expect("execution-control records lock poisoned");
        ExecutionControlRuntimeSnapshot {
            executions: records
                .values()
                .map(ExecutionControlRecord::to_snapshot)
                .collect(),
        }
    }

    fn record_event(&self, record: &mut ExecutionControlRecord, event: ExecutionControlEvent) {
        tracing::info!(
            execution_id = %event.execution_id,
            state = ?event.state,
            reason_code = %event.reason_code,
            trace_id = %event.trace.trace_id,
            "execution control event recorded"
        );
        record.state = event.state.clone();
        record.events.push(event.clone());
        self.observer.observe(&event);
    }
}

impl ExecutionControlRecord {
    fn to_result(&self) -> ExecutionControlCommandResult {
        result(
            self.scope.clone(),
            self.status.clone(),
            self.state.clone(),
            self.reason_code.clone(),
            self.events.clone(),
        )
    }

    fn to_snapshot(&self) -> ExecutionControlExecutionSnapshot {
        ExecutionControlExecutionSnapshot {
            scope: self.scope.clone(),
            state: self.state.clone(),
            reason_code: self.reason_code.clone(),
            checkpoint_refs: self.checkpoint_refs.clone(),
            events: self.events.clone(),
        }
    }
}

fn find_record_mut<'a>(
    records: &'a mut BTreeMap<String, ExecutionControlRecord>,
    execution_id: &str,
) -> MacacaResult<&'a mut ExecutionControlRecord> {
    records.get_mut(execution_id).ok_or_else(|| {
        MacacaError::Agent(format!(
            "execution control record '{execution_id}' was not registered"
        ))
    })
}

fn result(
    scope: ExecutionControlScope,
    status: ExecutionControlResolutionStatus,
    state: ExecutionControlState,
    reason_code: String,
    events: Vec<ExecutionControlEvent>,
) -> ExecutionControlCommandResult {
    ExecutionControlCommandResult {
        scope,
        status,
        state,
        reason_code,
        events,
        metadata: BTreeMap::new(),
    }
}

fn event_for(
    scope: &ExecutionControlScope,
    state: ExecutionControlState,
    reason_code: &str,
    metadata: &BTreeMap<String, String>,
) -> ExecutionControlEvent {
    ExecutionControlEvent {
        execution_id: scope.execution_id.clone(),
        state,
        reason_code: reason_code.into(),
        trace: scope.trace.clone(),
        metadata: sanitize_metadata(metadata),
    }
}

fn non_empty_reason(reason_code: String) -> MacacaResult<String> {
    let trimmed = reason_code.trim().to_string();
    if trimmed.is_empty() {
        Err(MacacaError::Agent(
            "execution control reason_code is required".into(),
        ))
    } else {
        Ok(trimmed)
    }
}

fn sanitize_metadata(metadata: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .iter()
        .filter(|(key, _)| !is_sensitive_key(key))
        .map(|(key, value)| (key.clone(), sanitize_value(value)))
        .collect()
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    [
        "secret",
        "token",
        "credential",
        "private_key",
        "prompt",
        "manifest",
        "payload",
        "wasm",
        "package_bytes",
        "signature",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn sanitize_value(value: &str) -> String {
    const MAX_VALUE_LEN: usize = 256;
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX_VALUE_LEN {
        return trimmed.to_string();
    }
    trimmed.chars().take(MAX_VALUE_LEN).collect()
}

#[cfg(test)]
#[path = "execution_control_runtime_tests.rs"]
mod execution_control_runtime_tests;
