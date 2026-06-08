use std::time::Duration;

use macaca_proto::{AgentId, ApplicationId, MemoryLayer};
use tempfile::TempDir;

use crate::core::{
    BuiltinAgentPrivateMemory, BuiltinSessionSharedMemory, DefaultMemoryRouter, MemoryFabricFacade,
    MemoryFacade, MemoryGetRequest, MemoryRoute, MemoryRouter, MemoryScope, MemorySearchRequest,
    MemoryVisibility, MemoryWriteRequest,
};
use crate::embedding::MockEmbedding;
use crate::file::FileMemory;
use crate::isolated::IsolatedMemoryManager;
use crate::manager::MemoryManager;
use crate::session::SessionMemory;
use crate::vector::InMemoryVectorStore;

fn private_manager(
    dir: &TempDir,
    app_id: ApplicationId,
    agent_id: AgentId,
) -> IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> {
    IsolatedMemoryManager::new(
        app_id,
        agent_id,
        dir.path().to_path_buf(),
        Duration::from_secs(60),
        None,
        None,
    )
}

fn shared_manager(dir: &TempDir) -> MemoryManager<InMemoryVectorStore, MockEmbedding> {
    MemoryManager::new(
        SessionMemory::new(Duration::from_secs(60)),
        FileMemory::new(dir.path().join("shared")),
        None,
        None,
    )
}

#[test]
fn agent_private_scope_requires_agent_identity() {
    let scope = MemoryScope::new(ApplicationId::new(), MemoryVisibility::AgentPrivate);

    assert!(scope.validate().is_err());
    assert!(
        MemoryScope::agent_private(ApplicationId::new(), AgentId::new())
            .validate()
            .is_ok()
    );
    assert!(
        MemoryScope::agent_private_named(ApplicationId::new(), "fixture-named-agent")
            .validate()
            .is_ok()
    );
}

#[test]
fn session_shared_scope_requires_session_or_project() {
    let scope = MemoryScope::new(ApplicationId::new(), MemoryVisibility::SessionShared);

    assert!(scope.validate().is_err());
    assert!(
        MemoryScope::session_shared(ApplicationId::new(), "session-1")
            .validate()
            .is_ok()
    );
    assert!(
        MemoryScope::project_shared(ApplicationId::new(), "project-1")
            .validate()
            .is_ok()
    );
}

#[test]
fn default_router_maps_visibility_to_routes() {
    let router = DefaultMemoryRouter;
    let private_scope = MemoryScope::agent_private(ApplicationId::new(), AgentId::new());
    let shared_scope = MemoryScope::session_shared(ApplicationId::new(), "session-1");

    assert_eq!(
        router.route(&private_scope).unwrap(),
        MemoryRoute::AgentPrivate
    );
    assert_eq!(
        router.route(&shared_scope).unwrap(),
        MemoryRoute::SessionShared
    );
    assert_eq!(
        router.recall_route(&private_scope).unwrap(),
        MemoryRoute::AgentPrivate
    );
    assert_eq!(
        router
            .recall_route(&private_scope.session_id("session-1"))
            .unwrap(),
        MemoryRoute::Composite(vec![MemoryRoute::AgentPrivate, MemoryRoute::SessionShared])
    );
}

#[tokio::test]
async fn builtin_agent_private_adapter_preserves_agent_isolation() {
    let dir = TempDir::new().unwrap();
    let app_id = ApplicationId::new();
    let planner_id = AgentId::new();
    let coder_id = AgentId::new();
    let planner = private_manager(&dir, app_id, planner_id);
    let coder = private_manager(&dir, app_id, coder_id);
    let planner_memory = BuiltinAgentPrivateMemory::new(&planner);
    let coder_memory = BuiltinAgentPrivateMemory::new(&coder);

    let planner_scope = MemoryScope::agent_private(app_id, planner_id);
    let coder_scope = MemoryScope::agent_private(app_id, coder_id);

    planner_memory
        .remember(
            MemoryWriteRequest::new(planner_scope.clone(), "alpha-only invariant")
                .layer(MemoryLayer::File),
        )
        .await
        .unwrap();

    let planner_results = planner_memory
        .search(MemorySearchRequest::new(
            planner_scope,
            "alpha-only invariant",
            10,
        ))
        .await
        .unwrap();
    let coder_results = coder_memory
        .search(MemorySearchRequest::new(
            coder_scope,
            "alpha-only invariant",
            10,
        ))
        .await
        .unwrap();

    assert_eq!(planner_results.len(), 1);
    assert!(coder_results.is_empty());
}

#[tokio::test]
async fn session_shared_adapter_exposes_shared_memory() {
    let dir = TempDir::new().unwrap();
    let app_id = ApplicationId::new();
    let shared = shared_manager(&dir);
    let shared_memory = BuiltinSessionSharedMemory::new(&shared);
    let scope = MemoryScope::session_shared(app_id, "session-1").agent_id(AgentId::new());

    let id = shared_memory
        .remember(MemoryWriteRequest::new(
            scope.clone(),
            "shared architecture decision",
        ))
        .await
        .unwrap();

    let results = shared_memory
        .search(MemorySearchRequest::new(
            scope.clone(),
            "architecture decision",
            10,
        ))
        .await
        .unwrap();
    let entry = shared_memory
        .get(MemoryGetRequest { scope, id })
        .await
        .unwrap()
        .unwrap();

    assert!(results.iter().any(|item| item.id == id));
    assert_eq!(entry.metadata["memory_scope"]["session_id"], "session-1");
}

#[tokio::test]
async fn fabric_does_not_promote_private_memory_to_shared() {
    let private_dir = TempDir::new().unwrap();
    let shared_dir = TempDir::new().unwrap();
    let app_id = ApplicationId::new();
    let agent_id = AgentId::new();
    let private = private_manager(&private_dir, app_id, agent_id);
    let shared = shared_manager(&shared_dir);
    let fabric = MemoryFabricFacade::new(
        BuiltinAgentPrivateMemory::new(&private),
        BuiltinSessionSharedMemory::new(&shared),
    );
    let private_scope = MemoryScope::agent_private(app_id, agent_id);
    let shared_scope = MemoryScope::session_shared(app_id, "session-1").agent_id(agent_id);

    fabric
        .remember(MemoryWriteRequest::new(
            private_scope,
            "private implementation note",
        ))
        .await
        .unwrap();

    let shared_results = fabric
        .search(MemorySearchRequest::new(
            shared_scope,
            "private implementation note",
            10,
        ))
        .await
        .unwrap();

    assert!(shared_results.is_empty());
}

#[tokio::test]
async fn fabric_private_recall_without_session_uses_private_route_only() {
    let private_dir = TempDir::new().unwrap();
    let shared_dir = TempDir::new().unwrap();
    let app_id = ApplicationId::new();
    let agent_id = AgentId::new();
    let private = private_manager(&private_dir, app_id, agent_id);
    let shared = shared_manager(&shared_dir);
    let fabric = MemoryFabricFacade::new(
        BuiltinAgentPrivateMemory::new(&private),
        BuiltinSessionSharedMemory::new(&shared),
    );
    let scope = MemoryScope::agent_private(app_id, agent_id);

    let id = fabric
        .remember(MemoryWriteRequest::new(
            scope.clone(),
            "private recall note",
        ))
        .await
        .unwrap();
    let results = fabric
        .search(MemorySearchRequest::new(scope, "private recall note", 10))
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
}
