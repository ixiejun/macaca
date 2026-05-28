//! Command handlers for `service.hook`.
//!
//! The handlers keep lifecycle semantics in the service provider layer rather
//! than shell code.  Hook selection is an executable Specification, execution is
//! a deterministic Chain of Responsibility, and adapter absence is represented
//! as structured hook outcomes instead of panics or fake success.

use std::time::Instant;

use macaca_proto::workbench::hook::*;
use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use tracing::{info, warn};

use crate::hook_service_provider::{
    audit_ref, event_ref, sanitize_summary, HookSystemServiceProvider, LIST_LIMIT_DEFAULT,
    LIST_LIMIT_MAX,
};

impl HookSystemServiceProvider {
    pub(crate) async fn catalog_list(
        &self,
        request: HookCatalogList,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let limit = request
            .limit
            .unwrap_or(LIST_LIMIT_DEFAULT)
            .clamp(1, LIST_LIMIT_MAX);
        let mut hooks: Vec<_> = self
            .hooks
            .read()
            .await
            .iter()
            .filter(|hook| request.include_disabled || hook.enabled)
            .filter(|hook| {
                request
                    .stage
                    .as_ref()
                    .map_or(true, |stage| &hook.stage == stage)
            })
            .filter(|hook| {
                request
                    .source_class
                    .as_ref()
                    .map_or(true, |source| &hook.source_class == source)
            })
            .cloned()
            .collect();
        sort_hooks(&mut hooks);
        let truncated = hooks.len() > limit;
        hooks.truncate(limit);
        info!(
            trace_id = %trace.trace_id,
            returned = hooks.len(),
            truncated,
            "hook catalog listed descriptors"
        );
        Self::result(
            HookServiceResponse::Catalog { hooks, truncated },
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn policy_resolve(
        &self,
        request: HookPolicyResolve,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let resolution = self.resolve_policy(&request).await;
        info!(
            trace_id = %trace.trace_id,
            stage = ?request.stage,
            active = resolution.active_hooks.len(),
            ignored = resolution.ignored_hooks.len(),
            managed_only = request.managed_only,
            "hook policy resolved lifecycle chain"
        );
        Self::result(
            HookServiceResponse::Policy(resolution),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn pre_tool_run(
        &self,
        request: HookPreToolRun,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let policy = HookPolicyResolve {
            scope: request.scope,
            stage: HookStage::PreTool,
            managed_only: request.managed_only,
            include_unavailable_adapters: true,
            metadata: request.metadata,
        };
        self.run_chain(policy, trace, request.input_summary, None)
            .await
    }

    pub(crate) async fn post_tool_run(
        &self,
        request: HookPostToolRun,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let policy = HookPolicyResolve {
            scope: request.scope,
            stage: HookStage::PostTool,
            managed_only: request.managed_only,
            include_unavailable_adapters: true,
            metadata: request.metadata,
        };
        self.run_chain(policy, trace, None, request.output_summary)
            .await
    }

    pub(crate) async fn lifecycle_run(
        &self,
        request: HookLifecycleRun,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        if !matches!(request.stage, HookStage::Session | HookStage::Turn) {
            return Err(ServiceError::InvalidArgument(
                "lifecycle hook command only accepts session or turn stage".into(),
            ));
        }
        let policy = HookPolicyResolve {
            scope: request.scope,
            stage: request.stage,
            managed_only: request.managed_only,
            include_unavailable_adapters: true,
            metadata: request.metadata,
        };
        self.run_chain(policy, trace, request.lifecycle_summary, None)
            .await
    }

    pub(crate) async fn result_audit(
        &self,
        request: HookResultAuditRead,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let audit = self
            .audits
            .read()
            .await
            .get(&request.audit_ref)
            .cloned()
            .ok_or_else(|| ServiceError::InvalidArgument("unknown hook audit_ref".into()))?;
        Self::result(
            HookServiceResponse::Audit(audit),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
            None,
        )
    }

    async fn run_chain(
        &self,
        policy: HookPolicyResolve,
        trace: TraceContext,
        input_summary: Option<macaca_proto::BoundedSummary>,
        output_summary: Option<macaca_proto::BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let resolution = self.resolve_policy(&policy).await;
        let mut outcomes = Vec::new();
        let mut decision = HookAction::Continue;
        for hook in resolution.active_hooks {
            let outcome = execute_descriptor(&hook, &policy.stage);
            decision = merge_action(decision, outcome.action.clone());
            let should_stop = matches!(decision, HookAction::Block | HookAction::RequestStop);
            outcomes.push(outcome);
            if should_stop {
                warn!(
                    trace_id = %trace.trace_id,
                    stage = ?policy.stage,
                    decision = ?decision,
                    "hook service stopped lower-priority hooks"
                );
                break;
            }
        }
        let audit = self
            .persist_audit(&policy.stage, &decision, &outcomes, &trace)
            .await;
        let result = HookRunResult {
            decision: decision.clone(),
            rewritten_input: select_summary(&outcomes, HookAction::RewriteInput)
                .or_else(|| sanitize_summary(input_summary)),
            additional_context: select_summary(&outcomes, HookAction::AddContext),
            replacement_output: select_summary(&outcomes, HookAction::ReplaceOutput)
                .or_else(|| sanitize_summary(output_summary)),
            outcomes,
            audit_ref: audit.audit_ref.clone(),
        };
        self.emit_event(&audit, &trace).await;
        info!(
            audit_ref = %audit.audit_ref,
            trace_id = %trace.trace_id,
            stage = ?policy.stage,
            decision = ?decision,
            "hook chain completed with audit memento"
        );
        Self::result(
            HookServiceResponse::Run(result),
            trace,
            status_for_action(&decision),
            None,
        )
    }

    async fn resolve_policy(&self, request: &HookPolicyResolve) -> HookPolicyResolution {
        let mut active_hooks = Vec::new();
        let mut ignored_hooks = Vec::new();
        for hook in self.hooks.read().await.iter().cloned() {
            let active = hook.enabled
                && hook.stage == request.stage
                && scope_matches(&hook.scope, &request.scope)
                && (!request.managed_only || hook.source_class == HookSourceClass::Managed)
                && (request.include_unavailable_adapters
                    || !matches!(hook.adapter_kind, HookAdapterKind::Unavailable));
            if active {
                active_hooks.push(hook);
            } else {
                ignored_hooks.push(hook);
            }
        }
        sort_hooks(&mut active_hooks);
        HookPolicyResolution {
            active_hooks,
            ignored_hooks,
            managed_only: request.managed_only,
            reason_code: if request.managed_only {
                "managed_hooks_only".into()
            } else {
                "all_enabled_hooks".into()
            },
        }
    }

    async fn persist_audit(
        &self,
        stage: &HookStage,
        decision: &HookAction,
        outcomes: &[HookOutcome],
        trace: &TraceContext,
    ) -> HookAuditRecord {
        let record = HookAuditRecord {
            audit_ref: audit_ref(),
            stage: stage.clone(),
            decision: decision.clone(),
            hook_ids: outcomes
                .iter()
                .map(|outcome| outcome.hook_id.clone())
                .collect(),
            trace_id: trace.trace_id.clone(),
            recorded_at: chrono::Utc::now(),
        };
        self.audits
            .write()
            .await
            .insert(record.audit_ref.clone(), record.clone());
        record
    }

    async fn emit_event(&self, audit: &HookAuditRecord, trace: &TraceContext) {
        let _ = self.notify_tx.send(HookEvent {
            event_ref: event_ref(),
            audit_ref: audit.audit_ref.clone(),
            stage: audit.stage.clone(),
            decision: audit.decision.clone(),
            trace_id: trace.trace_id.clone(),
            emitted_at: chrono::Utc::now(),
        });
    }
}

fn execute_descriptor(hook: &HookDescriptor, stage: &HookStage) -> HookOutcome {
    let started = Instant::now();
    let adapter_available = matches!(
        hook.adapter_kind,
        HookAdapterKind::Local | HookAdapterKind::Mock
    );
    let action = if adapter_available {
        hook.default_action.clone()
    } else {
        HookAction::Continue
    };
    HookOutcome {
        hook_id: hook.hook_id.clone(),
        source_class: hook.source_class.clone(),
        stage: stage.clone(),
        action,
        status: if adapter_available {
            "ok"
        } else {
            "unavailable"
        }
        .into(),
        duration_ms: started.elapsed().as_millis() as u64,
        feedback: sanitize_summary(hook.feedback.clone()),
        error_code: if adapter_available {
            None
        } else {
            Some("hook_adapter_unavailable".into())
        },
    }
}

fn sort_hooks(hooks: &mut [HookDescriptor]) {
    hooks.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.hook_id.cmp(&right.hook_id))
    });
}

fn scope_matches(filter: &HookScope, actual: &HookScope) -> bool {
    optional_matches(&filter.application_id, &actual.application_id)
        && optional_matches(&filter.session_id, &actual.session_id)
        && optional_matches(&filter.thread_ref, &actual.thread_ref)
        && optional_matches(&filter.turn_ref, &actual.turn_ref)
        && optional_matches(&filter.tool_family, &actual.tool_family)
        && optional_matches(&filter.command_name, &actual.command_name)
}

fn optional_matches(filter: &Option<String>, actual: &Option<String>) -> bool {
    filter.as_ref().map_or(true, |expected| {
        actual.as_ref().is_some_and(|value| value == expected)
    })
}

fn merge_action(current: HookAction, next: HookAction) -> HookAction {
    match (current, next) {
        (HookAction::Block, _) | (_, HookAction::Block) => HookAction::Block,
        (HookAction::RequestStop, _) | (_, HookAction::RequestStop) => HookAction::RequestStop,
        (_, HookAction::ReplaceOutput) => HookAction::ReplaceOutput,
        (_, HookAction::RewriteInput) => HookAction::RewriteInput,
        (_, HookAction::AddContext) => HookAction::AddContext,
        _ => HookAction::Continue,
    }
}

fn select_summary(
    outcomes: &[HookOutcome],
    action: HookAction,
) -> Option<macaca_proto::BoundedSummary> {
    outcomes
        .iter()
        .find(|outcome| outcome.action == action)
        .and_then(|outcome| outcome.feedback.clone())
}

fn status_for_action(action: &HookAction) -> macaca_proto::WorkbenchCommandStatus {
    match action {
        HookAction::Block => macaca_proto::WorkbenchCommandStatus::Denied {
            reason: "pre_tool_hook_blocked".into(),
        },
        _ => macaca_proto::WorkbenchCommandStatus::Completed,
    }
}
