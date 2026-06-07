//! Runtime execution-control handle wiring pause/resume channels to policy.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use macaca_proto::ExecutionControlPolicy;
use crate::runtime_resume::RuntimeResumeSignal;
/// Runtime-agent pause/resume wiring selected by execution-control policy.
///
/// The web host owns the local session channel, but the pause trigger comes from
/// provider-neutral execution-control policy. This keeps runtime execution
/// generic while preserving the current in-process resume channel until
/// `service.execution_control` owns the state machine.
#[derive(Clone)]
pub struct RuntimeExecutionControl {
    pub(crate) pause_signal: Arc<AtomicBool>,
    pub(crate) resume_rx: Arc<Mutex<mpsc::Receiver<RuntimeResumeSignal>>>,
    pub(crate) policy: ExecutionControlPolicy,
    pub(crate) execution_id: String,
}

impl RuntimeExecutionControl {
    /// Build an in-process execution-control handle from the session-level pause
    /// signal and the receiver completed by the selected resume source.
    pub fn new(
        pause_signal: Arc<AtomicBool>,
        resume_rx: mpsc::Receiver<RuntimeResumeSignal>,
        policy: ExecutionControlPolicy,
        execution_id: impl Into<String>,
    ) -> Self {
        Self {
            pause_signal,
            resume_rx: Arc::new(Mutex::new(resume_rx)),
            policy,
            execution_id: execution_id.into(),
        }
    }
}
