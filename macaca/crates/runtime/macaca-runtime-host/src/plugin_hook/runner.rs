//! Plugin Hook Runner.
//!
//! The runner applies the Chain of Responsibility pattern over ordered hook
//! registrations.  Each descriptor is executed through a `PluginHookExecutor`
//! proxy.  v1 ships a descriptor-only executor that produces structured no-op
//! or unavailable outcomes; future WASM/process/remote hosts can replace the
//! executor without changing registry or service code.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use macaca_proto::{
    PluginError, PluginHookDecision, PluginHookEvent, PluginHookInvokeCommand,
    PluginHookInvokeResult, PluginHookKind, PluginHookOutcome, PluginHookRegistration,
};
use tracing::{info, warn};

use crate::plugin_hook::policy::{
    kind_matches, DefaultPluginHookFailureStrategy, DefaultPluginHookTimeoutStrategy,
    PluginHookFailureStrategy, PluginHookTimeoutStrategy,
};
use crate::plugin_hook::registry::PluginHookEventSink;

/// Proxy boundary for concrete hook execution hosts.
#[async_trait]
pub trait PluginHookExecutor: Send + Sync {
    /// Execute one hook descriptor.  Implementations must treat the payload as
    /// bounded metadata and must not request raw secrets through side channels.
    async fn execute(
        &self,
        registration: &PluginHookRegistration,
        command: &PluginHookInvokeCommand,
        timeout_millis: Option<u64>,
    ) -> Result<PluginHookOutcome, PluginError>;
}

/// Descriptor-only Null Object executor.
///
/// This executor makes Hook Bus v1 safe before third-party code execution
/// exists.  Observer hooks become no-op successes.  Mutating, blocking, and
/// approval hooks return a structured unavailable outcome so owning services can
/// continue or escalate according to descriptor failure policy.
pub struct DescriptorOnlyPluginHookExecutor;

#[async_trait]
impl PluginHookExecutor for DescriptorOnlyPluginHookExecutor {
    async fn execute(
        &self,
        registration: &PluginHookRegistration,
        _command: &PluginHookInvokeCommand,
        _timeout_millis: Option<u64>,
    ) -> Result<PluginHookOutcome, PluginError> {
        let started_at = Instant::now();
        let (status, decision, error_code) = match registration.kind {
            PluginHookKind::Observer => ("ok", PluginHookDecision::Noop, None),
            PluginHookKind::Mutating | PluginHookKind::Blocking | PluginHookKind::Approval => (
                "unavailable",
                PluginHookDecision::Noop,
                Some("execution_host_unavailable".into()),
            ),
        };
        Ok(PluginHookOutcome {
            plugin_id: registration.plugin_id.clone(),
            hook_name: registration.hook_name.clone(),
            kind: registration.kind.clone(),
            status: status.into(),
            decision,
            mutation: None,
            approval: None,
            error_code,
            duration_ms: started_at.elapsed().as_millis() as u64,
            metadata: Default::default(),
        })
    }
}

/// Ordered hook runner with replaceable executor and policies.
pub struct PluginHookRunner {
    executor: Arc<dyn PluginHookExecutor>,
    timeout_strategy: Arc<dyn PluginHookTimeoutStrategy>,
    failure_strategy: Arc<dyn PluginHookFailureStrategy>,
    event_sink: Arc<dyn PluginHookEventSink>,
}

impl PluginHookRunner {
    /// Build a runner from explicit strategies.
    pub fn new(
        executor: Arc<dyn PluginHookExecutor>,
        timeout_strategy: Arc<dyn PluginHookTimeoutStrategy>,
        failure_strategy: Arc<dyn PluginHookFailureStrategy>,
        event_sink: Arc<dyn PluginHookEventSink>,
    ) -> Self {
        Self {
            executor,
            timeout_strategy,
            failure_strategy,
            event_sink,
        }
    }

    /// Build the descriptor-only runner used by default hosts.
    pub fn descriptor_only(event_sink: Arc<dyn PluginHookEventSink>) -> Self {
        Self::new(
            Arc::new(DescriptorOnlyPluginHookExecutor),
            Arc::new(DefaultPluginHookTimeoutStrategy::default()),
            Arc::new(DefaultPluginHookFailureStrategy),
            event_sink,
        )
    }

    /// Execute ordered registrations and aggregate their decisions.
    pub async fn run(
        &self,
        command: PluginHookInvokeCommand,
        registrations: Vec<PluginHookRegistration>,
    ) -> Result<PluginHookInvokeResult, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            hook_name = %command.hook_name,
            registrations = registrations.len(),
            "plugin hook runner accepted invocation"
        );
        let mut outcomes = Vec::new();
        let mut aggregate = PluginHookDecision::Noop;
        for registration in registrations {
            if !kind_matches(&registration.descriptor, command.expected_kind.as_ref()) {
                continue;
            }
            let timeout = self
                .timeout_strategy
                .timeout_millis(&registration.descriptor);
            let outcome = match self
                .executor
                .execute(&registration, &command, timeout)
                .await
            {
                Ok(outcome) => outcome,
                Err(error) => self.failure_outcome(&registration, error.to_string()),
            };
            aggregate = merge_decision(aggregate, outcome.decision.clone());
            self.emit_outcome(&command, &outcome);
            let should_stop = matches!(
                aggregate,
                PluginHookDecision::Block | PluginHookDecision::RequireApproval
            ) && matches!(
                registration.kind,
                PluginHookKind::Blocking | PluginHookKind::Approval
            );
            outcomes.push(outcome);
            if should_stop {
                warn!(
                    trace_id = %command.trace.trace_id,
                    hook_name = %command.hook_name,
                    decision = ?aggregate,
                    "plugin hook runner stopped lower-priority hooks"
                );
                break;
            }
        }
        Ok(PluginHookInvokeResult {
            hook_name: command.hook_name,
            decision: aggregate,
            outcomes,
            trace: command.trace,
            metadata: Default::default(),
        })
    }

    fn failure_outcome(
        &self,
        registration: &PluginHookRegistration,
        error: String,
    ) -> PluginHookOutcome {
        PluginHookOutcome {
            plugin_id: registration.plugin_id.clone(),
            hook_name: registration.hook_name.clone(),
            kind: registration.kind.clone(),
            status: "error".into(),
            decision: self
                .failure_strategy
                .decision_for_failure(&registration.descriptor),
            mutation: None,
            approval: None,
            error_code: Some(error),
            duration_ms: 0,
            metadata: Default::default(),
        }
    }

    fn emit_outcome(&self, command: &PluginHookInvokeCommand, outcome: &PluginHookOutcome) {
        let mut event = PluginHookEvent::new(
            Some(outcome.plugin_id.clone()),
            Some(outcome.hook_name.clone()),
            Some(outcome.kind.clone()),
            "invoke",
            outcome.status.clone(),
            command.trace.clone(),
        );
        event.duration_ms = Some(outcome.duration_ms);
        event.error_code = outcome.error_code.clone();
        self.event_sink.emit(event);
    }
}

fn merge_decision(current: PluginHookDecision, next: PluginHookDecision) -> PluginHookDecision {
    match (current, next) {
        (PluginHookDecision::Block, _) | (_, PluginHookDecision::Block) => {
            PluginHookDecision::Block
        }
        (PluginHookDecision::RequireApproval, _) | (_, PluginHookDecision::RequireApproval) => {
            PluginHookDecision::RequireApproval
        }
        (PluginHookDecision::Allow, _) | (_, PluginHookDecision::Allow) => {
            PluginHookDecision::Allow
        }
        _ => PluginHookDecision::Noop,
    }
}
