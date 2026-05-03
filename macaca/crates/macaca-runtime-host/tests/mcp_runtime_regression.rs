#![allow(deprecated)]

use std::collections::BTreeMap;

use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_proto::ApplicationId;
use macaca_runtime_host::mcp_runtime::{
    McpDefinitionSource, McpLifecycleScope, McpRuntimeContext, McpRuntimeFacade,
    McpServerDefinition,
};

fn stateful_session_definition(id: &str) -> McpServerDefinition {
    McpServerDefinition {
        id: id.to_string(),
        transport: McpTransportConfig::Stdio {
            command: "playwright-mcp".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
        },
        lifecycle: McpLifecycleScope::Session,
        session_mode: McpSessionMode::Stateful,
        tool_prefix: None,
        required_bins: vec!["playwright-mcp".to_string()],
        enabled: true,
        source: McpDefinitionSource::App,
        concurrency_isolation: None,
    }
}

fn runtime_context(app_id: ApplicationId, session_id: &str, agent_name: &str) -> McpRuntimeContext {
    McpRuntimeContext {
        app_id: Some(app_id),
        session_id: Some(session_id.to_string()),
        agent_name: Some(agent_name.to_string()),
    }
}

#[tokio::test]
async fn concurrent_stateful_sessions_cleanup_isolated_by_session() {
    let facade = McpRuntimeFacade::new();
    let definition = stateful_session_definition("playwright");
    let app_id = ApplicationId(uuid::Uuid::nil());

    let context_a1 = runtime_context(app_id.clone(), "session-a", "agent-a");
    let context_a2 = context_a1.clone();
    let context_b1 = runtime_context(app_id.clone(), "session-b", "agent-b");
    let context_b2 = context_b1.clone();
    let facade_a1 = facade.clone();
    let facade_a2 = facade.clone();
    let facade_b1 = facade.clone();
    let facade_b2 = facade.clone();
    let definition_a1 = definition.clone();
    let definition_a2 = definition.clone();
    let definition_b1 = definition.clone();

    let (lease_a1, lease_a2, lease_b1, lease_b2) = tokio::join!(
        facade_a1.acquire_lease(&definition_a1, &context_a1),
        facade_a2.acquire_lease(&definition_a2, &context_a2),
        facade_b1.acquire_lease(&definition_b1, &context_b1),
        facade_b2.acquire_lease(&definition, &context_b2),
    );

    let statuses = facade.cleanup_session("session-a").await;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].server_id, "playwright");
    assert_eq!(statuses[0].lifecycle, McpLifecycleScope::Session);

    assert!(facade.release_lease(lease_a1).await.is_none());
    assert!(facade.release_lease(lease_a2).await.is_none());

    assert!(facade.release_lease(lease_b1).await.is_none());
    let remaining = facade
        .release_lease(lease_b2)
        .await
        .expect("session-b lease should remain active until its final release");
    assert_eq!(remaining.server_id, "playwright");
    assert_eq!(remaining.lifecycle, McpLifecycleScope::Session);
}

#[tokio::test]
async fn task_failure_cleanup_forces_release_of_outstanding_stateful_leases() {
    let facade = McpRuntimeFacade::new();
    let definition = stateful_session_definition("playwright");
    let app_id = ApplicationId(uuid::Uuid::nil());

    let context_1 = runtime_context(app_id.clone(), "task-failed", "agent-a");
    let context_2 = context_1.clone();
    let context_3 = context_1.clone();
    let facade_1 = facade.clone();
    let facade_2 = facade.clone();
    let facade_3 = facade.clone();
    let definition_1 = definition.clone();
    let definition_2 = definition.clone();

    let (lease_1, lease_2, lease_3) = tokio::join!(
        facade_1.acquire_lease(&definition_1, &context_1),
        facade_2.acquire_lease(&definition_2, &context_2),
        facade_3.acquire_lease(&definition, &context_3),
    );

    let statuses = facade.cleanup_session("task-failed").await;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].server_id, "playwright");

    for lease in [lease_1, lease_2, lease_3] {
        assert!(
            facade.release_lease(lease).await.is_none(),
            "forced session cleanup should make delayed close releases no-op"
        );
    }

    assert!(
        facade.cleanup_session("task-failed").await.is_empty(),
        "session cleanup should stay idempotent after forced release"
    );
}

#[tokio::test]
async fn task_timeout_cleanup_is_idempotent_against_late_close_callback() {
    let facade = McpRuntimeFacade::new();
    let definition = stateful_session_definition("playwright");
    let app_id = ApplicationId(uuid::Uuid::nil());
    let context = runtime_context(app_id, "task-timeout", "agent-a");

    let lease = facade.acquire_lease(&definition, &context).await;

    let statuses = facade.cleanup_session("task-timeout").await;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].server_id, "playwright");

    assert!(
        facade.cleanup_session("task-timeout").await.is_empty(),
        "timeout cleanup should be safe to retry"
    );
    assert!(
        facade.release_lease(lease).await.is_none(),
        "late close callback should not resurrect a timed-out session runtime"
    );
}
