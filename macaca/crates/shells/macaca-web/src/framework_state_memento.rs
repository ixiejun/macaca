//! Framework state memento helpers for Web shell adapters.
//!
//! The Web shell sometimes needs to remember framework-owned coordination
//! objects such as plan notebooks and execution contexts. This adapter stores
//! those snapshots in the AgentScope 2.0-style `AgentState` memento contract so
//! shell routes never depend on framework-internal serialization traits.

use macaca_sdk::framework::execution::ExecutionContext;
use macaca_sdk::framework::plan::PlanNotebook;
use macaca_sdk::framework::runtime_context::{
    AgentSessionError, AgentSessionStore, AgentState, SessionKey,
};
use tracing::{debug, warn};

const HOST_TENANT_SCOPE: &str = "macaca-host";
const PLAN_NOTEBOOK_OWNER: &str = "plan_notebook";
const EXECUTION_CONTEXT_OWNER: &str = "execution_context";
const PLAN_NOTEBOOK_NAMESPACE: &str = "framework.plan_notebook";
const EXECUTION_CONTEXT_NAMESPACE: &str = "framework.execution_context";

/// Load the plan notebook snapshot for an application session.
pub(crate) async fn load_plan_notebook(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
) -> PlanNotebook {
    load_snapshot(
        store,
        application_id,
        session_id,
        PLAN_NOTEBOOK_OWNER,
        PLAN_NOTEBOOK_NAMESPACE,
    )
    .await
    .unwrap_or_default()
}

/// Save the plan notebook snapshot for an application session.
pub(crate) async fn save_plan_notebook(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
    notebook: &PlanNotebook,
) {
    save_snapshot(
        store,
        application_id,
        session_id,
        PLAN_NOTEBOOK_OWNER,
        PLAN_NOTEBOOK_NAMESPACE,
        notebook,
    )
    .await;
}

/// Load the execution context snapshot for an application session.
pub(crate) async fn load_execution_context(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
) -> Option<ExecutionContext> {
    load_snapshot(
        store,
        application_id,
        session_id,
        EXECUTION_CONTEXT_OWNER,
        EXECUTION_CONTEXT_NAMESPACE,
    )
    .await
}

/// Save the execution context snapshot for an application session.
pub(crate) async fn save_execution_context(
    store: &dyn AgentSessionStore,
    context: &ExecutionContext,
) {
    save_snapshot(
        store,
        &context.app_id,
        &context.session_id,
        EXECUTION_CONTEXT_OWNER,
        EXECUTION_CONTEXT_NAMESPACE,
        context,
    )
    .await;
}

/// Load a shell cache namespace from the canonical framework state memento.
pub(crate) async fn load_session_namespace<T>(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
    namespace: &str,
) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    load_snapshot(store, application_id, session_id, namespace, namespace).await
}

/// Save a shell cache namespace into the canonical framework state memento.
pub(crate) async fn save_session_namespace<T>(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
    namespace: &str,
    value: &T,
) where
    T: serde::Serialize,
{
    save_snapshot(
        store,
        application_id,
        session_id,
        namespace,
        namespace,
        value,
    )
    .await;
}

async fn load_snapshot<T>(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
    owner: &str,
    namespace: &str,
) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let key = match key(application_id, session_id, owner) {
        Ok(key) => key,
        Err(error) => {
            warn!(?error, "framework memento key rejected");
            return None;
        }
    };
    match store.load_state(&key).await {
        Ok(Some(state)) => state
            .custom_namespaces
            .get(namespace)
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        Ok(None) => None,
        Err(error) => {
            warn!(?error, session_key = %key.storage_key(), "framework memento load failed");
            None
        }
    }
}

async fn save_snapshot<T>(
    store: &dyn AgentSessionStore,
    application_id: &str,
    session_id: &str,
    owner: &str,
    namespace: &str,
    value: &T,
) where
    T: serde::Serialize,
{
    let key = match key(application_id, session_id, owner) {
        Ok(key) => key,
        Err(error) => {
            warn!(?error, "framework memento key rejected");
            return;
        }
    };
    let mut state = match store.load_state(&key).await {
        Ok(Some(state)) => state,
        Ok(None) => AgentState::empty(),
        Err(error) => {
            warn!(?error, session_key = %key.storage_key(), "framework memento reload failed");
            AgentState::empty()
        }
    };
    match serde_json::to_value(value) {
        Ok(json) => {
            if let Err(error) = state.put_custom_namespace(namespace, json) {
                warn!(?error, namespace, "framework memento namespace rejected");
                return;
            }
        }
        Err(error) => {
            warn!(?error, namespace, "framework memento serialization failed");
            return;
        }
    }
    if let Err(error) = store.save_state(&key, state).await {
        warn!(?error, session_key = %key.storage_key(), "framework memento save failed");
    } else {
        debug!(session_key = %key.storage_key(), namespace, "framework memento saved");
    }
}

fn key(
    application_id: &str,
    session_id: &str,
    owner: &str,
) -> Result<SessionKey, AgentSessionError> {
    SessionKey::new(HOST_TENANT_SCOPE, application_id, session_id, owner)
}
