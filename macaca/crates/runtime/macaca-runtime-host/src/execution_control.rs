//! Runtime-host execution-control policy resolution.
//!
//! This module starts as the in-process Strategy layer for pause/resume policy.
//! It intentionally owns no workflow semantics: applications declare defaults,
//! executions may request bounded overrides, and the resolver returns a typed
//! decision before any runtime pause/resume side effect can happen.

use std::collections::BTreeMap;

use macaca_proto::{
    ExecutionControlCheckpointMode, ExecutionControlMode, ExecutionControlPolicy,
    ExecutionControlPolicyOverride, ExecutionControlResolutionStatus,
    ExecutionControlResolvedPolicy, ExecutionControlResumeSource, ExecutionControlTrigger,
};

/// Deterministic Strategy that merges application defaults with per-run
/// execution-control overrides.
///
/// The resolver owns only admission decisions.  It never subscribes to events,
/// pauses an agent, resumes a task, or interprets product workflows.  Returning
/// a structured decision before side effects keeps later runtime adapters and
/// `service.execution_control` providers traceable and testable.
pub struct ExecutionControlPolicyResolver;

impl ExecutionControlPolicyResolver {
    /// Resolve the effective policy for one execution.
    pub fn resolve(
        app_default: Option<&ExecutionControlPolicy>,
        command_override: Option<&ExecutionControlPolicyOverride>,
    ) -> ExecutionControlResolvedPolicy {
        match (app_default, command_override) {
            (None, None) => disabled("execution_control.disabled.not_declared"),
            (None, Some(_)) => denied("execution_control.denied.override_without_app_policy"),
            (Some(policy), None) => resolve_app_default(policy),
            (Some(policy), Some(override_policy)) => resolve_override(policy, override_policy),
        }
    }
}

fn resolve_app_default(policy: &ExecutionControlPolicy) -> ExecutionControlResolvedPolicy {
    if policy.mode == ExecutionControlMode::Disabled {
        return disabled("execution_control.disabled.app_policy");
    }
    if has_unsupported_trigger(&policy.triggers)
        || has_unsupported_resume_source(&policy.resume_sources)
    {
        return unsupported("execution_control.unsupported.extension_strategy");
    }
    enabled(policy.clone(), "execution_control.enabled.app_policy")
}

fn resolve_override(
    app_policy: &ExecutionControlPolicy,
    command_override: &ExecutionControlPolicyOverride,
) -> ExecutionControlResolvedPolicy {
    if app_policy.mode == ExecutionControlMode::Disabled {
        return denied("execution_control.denied.app_policy_disabled");
    }
    if !app_policy.allow_command_overrides {
        return denied("execution_control.denied.command_overrides_disabled");
    }
    if command_override.mode == ExecutionControlMode::Disabled {
        return disabled("execution_control.disabled.command_override");
    }
    if has_unsupported_trigger(&app_policy.triggers)
        || has_unsupported_trigger(&command_override.triggers)
        || has_unsupported_resume_source(&app_policy.resume_sources)
        || has_unsupported_resume_source(&command_override.resume_sources)
    {
        return unsupported("execution_control.unsupported.extension_strategy");
    }
    if !all_triggers_allowed(&command_override.triggers, &app_policy.triggers) {
        return denied("execution_control.denied.trigger_not_declared");
    }
    if !all_resume_sources_allowed(&command_override.resume_sources, &app_policy.resume_sources) {
        return denied("execution_control.denied.resume_source_not_declared");
    }
    if !checkpoint_mode_allowed(
        &command_override.checkpoint_mode,
        &app_policy.checkpoint_mode,
    ) {
        return denied("execution_control.denied.checkpoint_mode_not_declared");
    }

    let mut policy = ExecutionControlPolicy::enabled(
        command_override.triggers.clone(),
        command_override.resume_sources.clone(),
        command_override.checkpoint_mode.clone(),
    )
    .allow_command_overrides(app_policy.allow_command_overrides);
    policy.timeout_ms = command_override.timeout_ms.or(app_policy.timeout_ms);
    policy.metadata = merge_metadata(&app_policy.metadata, &command_override.metadata);
    enabled(policy, "execution_control.enabled.command_override")
}

fn has_unsupported_trigger(triggers: &[ExecutionControlTrigger]) -> bool {
    triggers
        .iter()
        .any(|trigger| matches!(trigger, ExecutionControlTrigger::Extension { .. }))
}

fn has_unsupported_resume_source(sources: &[ExecutionControlResumeSource]) -> bool {
    sources
        .iter()
        .any(|source| matches!(source, ExecutionControlResumeSource::Extension { .. }))
}

fn all_triggers_allowed(
    requested: &[ExecutionControlTrigger],
    allowed: &[ExecutionControlTrigger],
) -> bool {
    requested.iter().all(|trigger| allowed.contains(trigger))
}

fn all_resume_sources_allowed(
    requested: &[ExecutionControlResumeSource],
    allowed: &[ExecutionControlResumeSource],
) -> bool {
    requested.iter().all(|source| allowed.contains(source))
}

fn checkpoint_mode_allowed(
    requested: &ExecutionControlCheckpointMode,
    allowed: &ExecutionControlCheckpointMode,
) -> bool {
    checkpoint_rank(requested) <= checkpoint_rank(allowed)
}

fn checkpoint_rank(mode: &ExecutionControlCheckpointMode) -> u8 {
    match mode {
        ExecutionControlCheckpointMode::Disabled => 0,
        ExecutionControlCheckpointMode::ReferenceOnly => 1,
        ExecutionControlCheckpointMode::SnapshotReference => 2,
    }
}

fn merge_metadata(
    app_metadata: &BTreeMap<String, String>,
    override_metadata: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = app_metadata.clone();
    merged.extend(override_metadata.clone());
    merged
}

fn enabled(policy: ExecutionControlPolicy, reason_code: &str) -> ExecutionControlResolvedPolicy {
    ExecutionControlResolvedPolicy {
        status: ExecutionControlResolutionStatus::Enabled,
        policy: Some(policy),
        reason_code: reason_code.into(),
        metadata: BTreeMap::new(),
        events: Vec::new(),
    }
}

fn disabled(reason_code: &str) -> ExecutionControlResolvedPolicy {
    ExecutionControlResolvedPolicy {
        status: ExecutionControlResolutionStatus::Disabled,
        policy: None,
        reason_code: reason_code.into(),
        metadata: BTreeMap::new(),
        events: Vec::new(),
    }
}

fn denied(reason_code: &str) -> ExecutionControlResolvedPolicy {
    ExecutionControlResolvedPolicy {
        status: ExecutionControlResolutionStatus::Denied,
        policy: None,
        reason_code: reason_code.into(),
        metadata: BTreeMap::new(),
        events: Vec::new(),
    }
}

fn unsupported(reason_code: &str) -> ExecutionControlResolvedPolicy {
    ExecutionControlResolvedPolicy {
        status: ExecutionControlResolutionStatus::Unsupported,
        policy: None,
        reason_code: reason_code.into(),
        metadata: BTreeMap::new(),
        events: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        ExecutionControlCheckpointMode, ExecutionControlMode, ExecutionControlPolicy,
        ExecutionControlPolicyOverride, ExecutionControlResolutionStatus,
        ExecutionControlResumeSource, ExecutionControlTrigger,
    };

    use super::ExecutionControlPolicyResolver;

    #[test]
    fn resolver_disables_execution_control_when_no_policy_is_declared() {
        let resolution = ExecutionControlPolicyResolver::resolve(None, None);

        assert_eq!(
            resolution.status,
            ExecutionControlResolutionStatus::Disabled
        );
        assert_eq!(
            resolution.reason_code,
            "execution_control.disabled.not_declared"
        );
    }

    #[test]
    fn resolver_allows_command_override_to_narrow_manifest_defaults() {
        let app_policy = ExecutionControlPolicy::enabled(
            vec![
                ExecutionControlTrigger::tool_call_barrier("create_goal"),
                ExecutionControlTrigger::GoalLifecycle,
            ],
            vec![
                ExecutionControlResumeSource::goal_lifecycle(),
                ExecutionControlResumeSource::ApprovalDecision,
            ],
            ExecutionControlCheckpointMode::SnapshotReference,
        )
        .allow_command_overrides(true);
        let command_override = ExecutionControlPolicyOverride::enable_for_run(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        );

        let resolution =
            ExecutionControlPolicyResolver::resolve(Some(&app_policy), Some(&command_override));

        assert_eq!(resolution.status, ExecutionControlResolutionStatus::Enabled);
        let policy = resolution.policy.unwrap();
        assert_eq!(policy.mode, ExecutionControlMode::Enabled);
        assert_eq!(policy.triggers.len(), 1);
        assert_eq!(policy.resume_sources.len(), 1);
        assert_eq!(
            policy.checkpoint_mode,
            ExecutionControlCheckpointMode::ReferenceOnly
        );
    }

    #[test]
    fn resolver_denies_command_override_when_app_disallows_overrides() {
        let app_policy = ExecutionControlPolicy::enabled(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        );
        let command_override = ExecutionControlPolicyOverride::enable_for_run(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        );

        let resolution =
            ExecutionControlPolicyResolver::resolve(Some(&app_policy), Some(&command_override));

        assert_eq!(resolution.status, ExecutionControlResolutionStatus::Denied);
        assert_eq!(resolution.policy, None);
    }

    #[test]
    fn resolver_rejects_unsupported_extension_trigger() {
        let app_policy = ExecutionControlPolicy::enabled(
            vec![ExecutionControlTrigger::Extension {
                trigger_id: "custom.pause".into(),
            }],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        );

        let resolution = ExecutionControlPolicyResolver::resolve(Some(&app_policy), None);

        assert_eq!(
            resolution.status,
            ExecutionControlResolutionStatus::Unsupported
        );
        assert_eq!(resolution.policy, None);
    }
}
