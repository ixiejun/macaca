//! Focused tests for `service.hook`.
//!
//! These tests prove the service owns hook cataloging, managed-only policy,
//! deterministic ordering, block-before-side-effect decisions, bounded
//! feedback, audit replay, events, and unavailable provider behavior.

use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::workbench::hook::*;
use macaca_proto::{
    BoundedSummary, ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::HookSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn scope() -> HookScope {
    HookScope {
        application_id: Some("app.generic".into()),
        session_id: Some("session-1".into()),
        thread_ref: Some("thread-1".into()),
        turn_ref: Some("turn-1".into()),
        tool_family: Some("file".into()),
        command_name: Some("file.write".into()),
    }
}

fn hook(id: &str, priority: i32, source: HookSourceClass, action: HookAction) -> HookDescriptor {
    HookDescriptor {
        hook_id: id.into(),
        stage: HookStage::PreTool,
        source_class: source.clone(),
        adapter_kind: HookAdapterKind::Local,
        priority,
        managed: source == HookSourceClass::Managed,
        enabled: true,
        scope: HookScope {
            tool_family: Some("file".into()),
            ..HookScope::default()
        },
        default_action: action,
        feedback: Some(BoundedSummary {
            text: format!("{id} feedback"),
            truncated: false,
            original_byte_len: None,
            artifact_ref: None,
        }),
        trace_schema: "trace.service.hook.fixture.v1".into(),
        metadata: BTreeMap::new(),
    }
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<HookServiceResponse> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn catalog_and_policy_order_hooks_deterministically() {
    let provider = HookSystemServiceProvider::local(vec![
        hook("hook.b", 10, HookSourceClass::Managed, HookAction::Continue),
        hook("hook.a", 0, HookSourceClass::Managed, HookAction::Continue),
    ]);

    let catalog = decode(
        provider
            .call(command(CATALOG_LIST_COMMAND, HookCatalogList::default()))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        catalog.output,
        Some(HookServiceResponse::Catalog { hooks, truncated: false })
            if hooks[0].hook_id == "hook.a" && hooks[1].hook_id == "hook.b"
    ));

    let policy = decode(
        provider
            .call(command(
                POLICY_RESOLVE_COMMAND,
                HookPolicyResolve {
                    scope: scope(),
                    stage: HookStage::PreTool,
                    managed_only: false,
                    include_unavailable_adapters: true,
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        policy.output,
        Some(HookServiceResponse::Policy(resolution))
            if resolution.active_hooks.len() == 2
                && resolution.active_hooks[0].hook_id == "hook.a"
    ));
}

#[tokio::test]
async fn pre_tool_block_stops_lower_priority_hooks_and_emits_event() {
    let provider = HookSystemServiceProvider::local(vec![
        hook("hook.block", 0, HookSourceClass::Managed, HookAction::Block),
        hook(
            "hook.later",
            1,
            HookSourceClass::Managed,
            HookAction::Continue,
        ),
    ]);
    let mut events = provider.subscribe();

    let result = decode(
        provider
            .call(command(
                PRE_TOOL_RUN_COMMAND,
                HookPreToolRun {
                    scope: scope(),
                    managed_only: true,
                    input_summary: None,
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Denied { .. }
    ));
    let Some(HookServiceResponse::Run(run)) = result.output else {
        panic!("expected hook run result");
    };
    assert_eq!(run.decision, HookAction::Block);
    assert_eq!(run.outcomes.len(), 1);
    assert_eq!(events.recv().await.unwrap().decision, HookAction::Block);
}

#[tokio::test]
async fn managed_only_ignores_user_project_and_session_hooks() {
    let provider = HookSystemServiceProvider::local(vec![
        hook("hook.user", 0, HookSourceClass::User, HookAction::Block),
        hook(
            "hook.managed",
            1,
            HookSourceClass::Managed,
            HookAction::Continue,
        ),
    ]);

    let policy = decode(
        provider
            .call(command(
                POLICY_RESOLVE_COMMAND,
                HookPolicyResolve {
                    scope: scope(),
                    stage: HookStage::PreTool,
                    managed_only: true,
                    include_unavailable_adapters: true,
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        policy.output,
        Some(HookServiceResponse::Policy(resolution))
            if resolution.active_hooks.len() == 1
                && resolution.active_hooks[0].hook_id == "hook.managed"
                && resolution.ignored_hooks.iter().any(|hook| hook.hook_id == "hook.user")
    ));
}

#[tokio::test]
async fn feedback_is_bounded_and_audit_is_replayable() {
    let mut rewrite = hook(
        "hook.rewrite",
        0,
        HookSourceClass::Managed,
        HookAction::RewriteInput,
    );
    rewrite.feedback = Some(BoundedSummary {
        text: "x".repeat(5000),
        truncated: false,
        original_byte_len: None,
        artifact_ref: None,
    });
    let provider = HookSystemServiceProvider::local(vec![rewrite]);

    let result = decode(
        provider
            .call(command(
                PRE_TOOL_RUN_COMMAND,
                HookPreToolRun {
                    scope: scope(),
                    managed_only: false,
                    input_summary: None,
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(HookServiceResponse::Run(run)) = result.output else {
        panic!("expected hook run result");
    };
    assert!(run.rewritten_input.as_ref().unwrap().truncated);

    let audit = decode(
        provider
            .call(command(
                RESULT_AUDIT_COMMAND,
                HookResultAuditRead {
                    audit_ref: run.audit_ref,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        audit.output,
        Some(HookServiceResponse::Audit(record))
            if record.decision == HookAction::RewriteInput
                && record.hook_ids == vec!["hook.rewrite".to_string()]
    ));
}

#[tokio::test]
async fn unavailable_adapter_and_provider_return_structured_states() {
    let mut remote = hook(
        "hook.remote",
        0,
        HookSourceClass::Managed,
        HookAction::Block,
    );
    remote.adapter_kind = HookAdapterKind::Remote;
    let provider = HookSystemServiceProvider::local(vec![remote]);
    let result = decode(
        provider
            .call(command(
                PRE_TOOL_RUN_COMMAND,
                HookPreToolRun {
                    scope: scope(),
                    managed_only: false,
                    input_summary: None,
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        result.output,
        Some(HookServiceResponse::Run(run))
            if run.outcomes[0].status == "unavailable"
                && run.outcomes[0].error_code.as_deref() == Some("hook_adapter_unavailable")
    ));

    let unavailable = HookSystemServiceProvider::unavailable("hook disabled");
    let result = decode(
        unavailable
            .call(command(CATALOG_LIST_COMMAND, HookCatalogList::default()))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
}
