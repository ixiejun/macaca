//! Facade delegation, runtime-key refcount, lease, and session cleanup tests.

use macaca_proto::ApplicationId;

use crate::mcp_runtime::{
    McpRuntimeContext, McpRuntimeFacade, McpRuntimeStatusState, McpToolPolicy,
};

use super::super::manager::McpRuntimeManager;
use super::fixtures::stdio_definition;

#[tokio::test]
async fn runtime_key_reference_count_releases_on_last_owner() {
    let facade = McpRuntimeFacade::new();
    let definition = stdio_definition("playwright", "playwright-mcp");
    let context = McpRuntimeContext {
        app_id: Some(ApplicationId(uuid::Uuid::nil())),
        session_id: Some("session-a".into()),
        agent_name: Some("agent-a".into()),
    };
    let lease_a = facade.acquire_lease(&definition, &context).await;
    let lease_b = facade.acquire_lease(&definition, &context).await;

    assert!(facade.release_lease(lease_a).await.is_none());
    assert!(facade.release_lease(lease_b).await.is_some());
}

#[tokio::test]
async fn facade_delegates_definitions_and_probe() {
    let facade = McpRuntimeFacade::new();
    let mut definition = stdio_definition("disabled", "missing-binary");
    definition.enabled = false;
    facade.upsert_definition(definition).await;

    let definitions = facade.snapshot_server_definitions().await;
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].id, "disabled");

    let statuses = facade.probe(&McpToolPolicy::default()).await;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].server_id, "disabled");
    assert_eq!(statuses[0].state, McpRuntimeStatusState::Disabled);
}

#[tokio::test]
async fn facade_lease_release_delegates_to_runtime_manager() {
    let facade = McpRuntimeFacade::new();
    let definition = stdio_definition("playwright", "playwright-mcp");
    let context = McpRuntimeContext {
        app_id: Some(ApplicationId(uuid::Uuid::nil())),
        session_id: Some("session-a".into()),
        agent_name: Some("agent-a".into()),
    };

    let lease_a = facade.acquire_lease(&definition, &context).await;
    let lease_b = facade.acquire_lease(&definition, &context).await;

    assert!(facade.release_lease(lease_a).await.is_none());
    assert!(facade.release_lease(lease_b).await.is_some());
}

#[tokio::test]
async fn cleanup_session_forces_release_for_matching_leases() {
    let manager = McpRuntimeManager::new();
    let definition = stdio_definition("playwright", "playwright-mcp");
    let context = McpRuntimeContext {
        app_id: Some(ApplicationId(uuid::Uuid::nil())),
        session_id: Some("session-a".into()),
        agent_name: Some("agent-a".into()),
    };

    let _lease_a = manager.acquire_lease(&definition, &context).await;
    let _lease_b = manager.acquire_lease(&definition, &context).await;

    let statuses = manager.cleanup_session("session-a").await;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].server_id, "playwright");
}
