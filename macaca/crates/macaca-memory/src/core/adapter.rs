use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{MacacaError, MacacaResult, MemoryEntry, MemoryId};
use serde_json::{json, Value};

use crate::facade::{ForgetMemory, RecallQuery, RememberText};
use crate::isolated::IsolatedMemoryManager;
use crate::manager::MemoryManager;
use crate::store::{EmbeddingProvider, VectorStore};

use super::facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest, MemoryWriteRequest,
};
use super::router::{DefaultMemoryRouter, MemoryRoute, MemoryRouter};
use super::scope::MemoryVisibility;
use super::status::{MemoryCapabilitySet, MemoryStatusReport};

pub struct BuiltinAgentPrivateMemory<'a, V: VectorStore, E: EmbeddingProvider> {
    manager: &'a IsolatedMemoryManager<V, E>,
}

impl<'a, V: VectorStore, E: EmbeddingProvider> BuiltinAgentPrivateMemory<'a, V, E> {
    pub fn new(manager: &'a IsolatedMemoryManager<V, E>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl<V, E> MemoryFacade for BuiltinAgentPrivateMemory<'_, V, E>
where
    V: VectorStore,
    E: EmbeddingProvider,
{
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::AgentPrivate)?;
        let input = RememberText::new(request.content)
            .layer(request.layer)
            .metadata(with_scope_metadata(request.metadata, &request.scope))
            .agent_id(
                request
                    .scope
                    .agent_id_value()
                    .unwrap_or_else(|| self.manager.agent_id()),
            );
        self.manager.remember_text(input).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::AgentPrivate)?;
        let result = self
            .manager
            .recall(RecallQuery::new(request.query, request.limit))
            .await?;
        Ok(result.entries)
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::AgentPrivate)?;
        self.manager.get_memory(&request.id).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::AgentPrivate)?;
        self.manager.forget(ForgetMemory { id: request.id }).await
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy(
            "builtin-agent-private",
            MemoryCapabilitySet::basic_store_search(),
        )
    }
}

pub struct BuiltinSessionSharedMemory<'a, V: VectorStore, E: EmbeddingProvider> {
    manager: &'a MemoryManager<V, E>,
}

impl<'a, V: VectorStore, E: EmbeddingProvider> BuiltinSessionSharedMemory<'a, V, E> {
    pub fn new(manager: &'a MemoryManager<V, E>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl<V, E> MemoryFacade for BuiltinSessionSharedMemory<'_, V, E>
where
    V: VectorStore,
    E: EmbeddingProvider,
{
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::SessionShared)?;
        let mut input = RememberText::new(request.content)
            .layer(request.layer)
            .metadata(with_scope_metadata(request.metadata, &request.scope));
        if let Some(agent_id) = request.scope.agent_id_value() {
            input = input.agent_id(agent_id);
        }
        self.manager.remember_text(input).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::SessionShared)?;
        let result = self
            .manager
            .recall(RecallQuery::new(request.query, request.limit))
            .await?;
        Ok(result.entries)
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::SessionShared)?;
        self.manager.get_entry(&request.id).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        request.scope.validate()?;
        ensure_visibility(request.scope.visibility, MemoryVisibility::SessionShared)?;
        self.manager.forget(ForgetMemory { id: request.id }).await
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy(
            "builtin-session-shared",
            MemoryCapabilitySet::basic_store_search(),
        )
    }
}

pub struct MemoryFabricFacade<P, S, R = DefaultMemoryRouter> {
    private_memory: P,
    shared_memory: S,
    router: R,
}

impl<P, S> MemoryFabricFacade<P, S, DefaultMemoryRouter> {
    pub fn new(private_memory: P, shared_memory: S) -> Self {
        Self::with_router(private_memory, shared_memory, DefaultMemoryRouter)
    }
}

impl<P, S, R> MemoryFabricFacade<P, S, R> {
    pub fn with_router(private_memory: P, shared_memory: S, router: R) -> Self {
        Self {
            private_memory,
            shared_memory,
            router,
        }
    }
}

#[async_trait]
impl<P, S, R> MemoryFacade for MemoryFabricFacade<P, S, R>
where
    P: MemoryFacade,
    S: MemoryFacade,
    R: MemoryRouter,
{
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        match self.router.route(&request.scope)? {
            MemoryRoute::AgentPrivate => self.private_memory.remember(request).await,
            MemoryRoute::SessionShared => self.shared_memory.remember(request).await,
            route => unsupported_route(route),
        }
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        match self.router.recall_route(&request.scope)? {
            MemoryRoute::AgentPrivate => self.private_memory.search(request).await,
            MemoryRoute::SessionShared => self.shared_memory.search(request).await,
            MemoryRoute::Composite(routes) => self.search_composite(request, routes).await,
            route => unsupported_route(route),
        }
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        match self.router.route(&request.scope)? {
            MemoryRoute::AgentPrivate => self.private_memory.get(request).await,
            MemoryRoute::SessionShared => self.shared_memory.get(request).await,
            route => unsupported_route(route),
        }
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        match self.router.route(&request.scope)? {
            MemoryRoute::AgentPrivate => self.private_memory.delete(request).await,
            MemoryRoute::SessionShared => self.shared_memory.delete(request).await,
            route => unsupported_route(route),
        }
    }

    fn status(&self) -> MemoryStatusReport {
        let private = self.private_memory.status();
        let shared = self.shared_memory.status();
        MemoryStatusReport::healthy(
            "memory-fabric",
            MemoryCapabilitySet {
                store: private.capabilities.store && shared.capabilities.store,
                search: private.capabilities.search && shared.capabilities.search,
                ..MemoryCapabilitySet::default()
            },
        )
    }
}

impl<P, S, R> MemoryFabricFacade<P, S, R>
where
    P: MemoryFacade,
    S: MemoryFacade,
    R: MemoryRouter,
{
    async fn search_composite(
        &self,
        request: MemorySearchRequest,
        routes: Vec<MemoryRoute>,
    ) -> MacacaResult<Vec<MemoryEntry>> {
        let mut entries = Vec::new();
        for route in routes {
            let mut scoped = request.clone();
            scoped.scope.visibility = match route {
                MemoryRoute::AgentPrivate => MemoryVisibility::AgentPrivate,
                MemoryRoute::SessionShared => MemoryVisibility::SessionShared,
                route => return unsupported_route(route),
            };
            let mut routed = match route {
                MemoryRoute::AgentPrivate => self.private_memory.search(scoped).await?,
                MemoryRoute::SessionShared => self.shared_memory.search(scoped).await?,
                route => return unsupported_route(route),
            };
            entries.append(&mut routed);
        }
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.dedup_by_key(|entry| entry.id);
        entries.truncate(request.limit);
        Ok(entries)
    }
}

fn ensure_visibility(actual: MemoryVisibility, expected: MemoryVisibility) -> MacacaResult<()> {
    if actual != expected {
        return Err(MacacaError::Memory(format!(
            "memory adapter expected {expected:?}, got {actual:?}"
        )));
    }
    Ok(())
}

fn unsupported_route<T>(route: MemoryRoute) -> MacacaResult<T> {
    Err(MacacaError::Memory(format!(
        "memory route {route:?} is not implemented by builtin fabric"
    )))
}

fn with_scope_metadata(mut metadata: Value, scope: &super::scope::MemoryScope) -> Value {
    let scope_value = json!({
        "application_id": scope.identity.application_id.to_string(),
        "agent_id": scope.identity.agent_id.map(|id| id.to_string()),
        "agent_name": scope.identity.agent_name,
        "session_id": scope.identity.session_id,
        "project_id": scope.identity.project_id,
        "tenant_id": scope.tenant_id,
        "user_id": scope.user_id,
        "namespace": scope.namespace,
        "visibility": format!("{:?}", scope.visibility),
        "captured_at": Utc::now(),
    });
    match &mut metadata {
        Value::Object(map) => {
            map.insert("memory_scope".into(), scope_value);
            metadata
        }
        _ => json!({
            "value": metadata,
            "memory_scope": scope_value,
        }),
    }
}
