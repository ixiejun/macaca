// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It separates transient
// runtime context from durable agent state so framework providers remain
// replaceable and session persistence remains provider-neutral.

//! Runtime context, durable agent state, and session contracts.
//!
//! AgentScope 2.0 separates per-call `RuntimeContext` from persisted `AgentState`.
//! Macaca keeps that split as an OS boundary:
//!
//! - **Command**: framework calls carry an explicit `RuntimeContext`.
//! - **Memento**: `AgentState` is the durable snapshot saved under a
//!   `SessionKey`.
//! - **Strategy/Bridge**: `AgentSessionStore` lets runtime-host provide memory,
//!   persist, remote, plugin, or unavailable storage without changing callers.
//! - **Specification**: key validation and bounded extras reject ambiguous or
//!   unbounded state before it crosses provider boundaries.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::message::Msg;

const MAX_RUNTIME_EXTRA_KEYS: usize = 64;
const MAX_CUSTOM_STATE_NAMESPACES: usize = 128;

/// Per-call context that must not be persisted as durable agent state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeContext {
    pub trace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

impl RuntimeContext {
    /// Create a runtime context with the required trace id.
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            invocation_id: Some(Uuid::new_v4().to_string()),
            tenant_id: None,
            application_id: None,
            session_id: None,
            user_id: None,
            task_id: None,
            extras: BTreeMap::new(),
        }
    }

    /// Add generic bounded metadata. Callers should keep values sanitized
    /// because context can be copied into trace and audit records.
    pub fn with_extra(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<Self, RuntimeContextError> {
        if self.extras.len() >= MAX_RUNTIME_EXTRA_KEYS {
            return Err(RuntimeContextError::TooManyExtras {
                limit: MAX_RUNTIME_EXTRA_KEYS,
            });
        }
        self.extras.insert(key.into(), value);
        Ok(self)
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_application_id(mut self, application_id: impl Into<String>) -> Self {
        self.application_id = Some(application_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }
}

/// Runtime-context validation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeContextError {
    #[error("runtime context has too many extra keys: limit {limit}")]
    TooManyExtras { limit: usize },
}

/// Stable session key used to isolate durable agent state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionKey {
    pub tenant_id: String,
    pub application_id: String,
    pub session_id: String,
    pub agent_id: String,
}

impl SessionKey {
    /// Build a validated key. Empty segments are rejected because they create
    /// ambiguous persistence paths and weaken tenant/application isolation.
    pub fn new(
        tenant_id: impl Into<String>,
        application_id: impl Into<String>,
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
    ) -> Result<Self, AgentSessionError> {
        let key = Self {
            tenant_id: tenant_id.into(),
            application_id: application_id.into(),
            session_id: session_id.into(),
            agent_id: agent_id.into(),
        };
        key.validate()?;
        Ok(key)
    }

    /// Return a stable storage path for diagnostics and simple store adapters.
    pub fn storage_key(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.tenant_id, self.application_id, self.session_id, self.agent_id
        )
    }

    fn validate(&self) -> Result<(), AgentSessionError> {
        for (field, value) in [
            ("tenant_id", &self.tenant_id),
            ("application_id", &self.application_id),
            ("session_id", &self.session_id),
            ("agent_id", &self.agent_id),
        ] {
            if value.trim().is_empty() {
                return Err(AgentSessionError::InvalidKey {
                    field: field.to_string(),
                });
            }
        }
        Ok(())
    }
}

/// Pending tool state needed for suspend/resume and policy decisions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingToolState {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub requested_at: DateTime<Utc>,
}

/// Permission/HITL state that survives process restarts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Clear,
    WaitingForUserConfirm {
        request_id: String,
        tool_call_id: Option<String>,
        prompt: String,
    },
    WaitingForExternalExecution {
        request_id: String,
        tool_call_id: Option<String>,
    },
    Denied {
        reason: String,
    },
}

impl Default for PermissionState {
    fn default() -> Self {
        Self::Clear
    }
}

/// Agent-local plan-mode state. It deliberately does not model Macaca TaskBoard
/// ownership; task mutation must go through task/execution-control services.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanModeState {
    pub enabled: bool,
    pub plan_id: Option<String>,
    pub notes: Vec<String>,
}

impl Default for PlanModeState {
    fn default() -> Self {
        Self {
            enabled: false,
            plan_id: None,
            notes: Vec::new(),
        }
    }
}

/// Generic task context reference carried by durable state for resume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskContextState {
    pub task_id: String,
    pub checkpoint_ref: Option<String>,
    pub metadata: serde_json::Value,
}

/// Bounded read-cache entry. Values should be sanitized summaries or references,
/// not raw secrets, credentials, provider payloads, or unbounded file contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachedRead {
    pub value: serde_json::Value,
    pub cached_at: DateTime<Utc>,
}

/// Durable provider-neutral agent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub schema_version: String,
    pub conversation: Vec<Msg>,
    pub pending_tool: Option<PendingToolState>,
    pub permission: PermissionState,
    pub plan_mode: PlanModeState,
    pub task_context: Option<TaskContextState>,
    #[serde(default)]
    pub read_cache: BTreeMap<String, CachedRead>,
    #[serde(default)]
    pub custom_namespaces: BTreeMap<String, serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

impl AgentState {
    /// Create an empty durable state snapshot.
    pub fn empty() -> Self {
        Self {
            schema_version: "agent_state.v1".to_string(),
            conversation: Vec::new(),
            pending_tool: None,
            permission: PermissionState::Clear,
            plan_mode: PlanModeState::default(),
            task_context: None,
            read_cache: BTreeMap::new(),
            custom_namespaces: BTreeMap::new(),
            updated_at: Utc::now(),
        }
    }

    /// Append a conversation message and update the memento timestamp.
    pub fn push_message(&mut self, message: Msg) {
        self.conversation.push(message);
        self.touch();
    }

    /// Store provider-neutral custom state under a namespace.
    pub fn put_custom_namespace(
        &mut self,
        namespace: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<(), AgentSessionError> {
        if self.custom_namespaces.len() >= MAX_CUSTOM_STATE_NAMESPACES {
            return Err(AgentSessionError::TooManyCustomNamespaces {
                limit: MAX_CUSTOM_STATE_NAMESPACES,
            });
        }
        self.custom_namespaces.insert(namespace.into(), value);
        self.touch();
        Ok(())
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// Session store health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionHealthState {
    Healthy,
    Degraded,
    Unavailable,
}

/// Bounded session-store health result for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionHealth {
    pub state: AgentSessionHealthState,
    pub reason: String,
    pub stored_state_count: usize,
}

/// Sanitized session-store snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionStoreSnapshot {
    pub health: AgentSessionHealth,
    pub stored_keys: Vec<String>,
}

/// Structured errors for provider-neutral session stores.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentSessionError {
    #[error("invalid session key field: {field}")]
    InvalidKey { field: String },
    #[error("agent session not found: {key}")]
    NotFound { key: String },
    #[error("agent session store unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("agent session store failed: {reason}")]
    Failed { reason: String },
    #[error("agent state has too many custom namespaces: limit {limit}")]
    TooManyCustomNamespaces { limit: usize },
}

/// Provider-neutral durable state store for AgentScope 2.0-equivalent sessions.
#[async_trait]
pub trait AgentSessionStore: Send + Sync {
    async fn save_state(
        &self,
        key: &SessionKey,
        state: AgentState,
    ) -> Result<(), AgentSessionError>;
    async fn load_state(&self, key: &SessionKey) -> Result<Option<AgentState>, AgentSessionError>;
    async fn delete_state(&self, key: &SessionKey) -> Result<(), AgentSessionError>;
    async fn list_keys(&self) -> Result<Vec<SessionKey>, AgentSessionError>;
    fn health(&self) -> AgentSessionHealth;
    fn snapshot(&self) -> AgentSessionStoreSnapshot;
}

/// In-memory session store used for tests and local provider composition.
///
/// Production composition should bridge this trait to the persistence/session
/// service in runtime-host. Keeping this implementation small makes it useful
/// as a deterministic test double without making framework own storage policy.
pub struct InMemoryAgentSessionStore {
    states: tokio::sync::RwLock<BTreeMap<SessionKey, AgentState>>,
}

impl InMemoryAgentSessionStore {
    pub fn new() -> Self {
        info!("in-memory agent session store initialized");
        Self {
            states: tokio::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl Default for InMemoryAgentSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentSessionStore for InMemoryAgentSessionStore {
    async fn save_state(
        &self,
        key: &SessionKey,
        state: AgentState,
    ) -> Result<(), AgentSessionError> {
        debug!(session_key = %key.storage_key(), "saving durable agent state");
        self.states.write().await.insert(key.clone(), state);
        Ok(())
    }

    async fn load_state(&self, key: &SessionKey) -> Result<Option<AgentState>, AgentSessionError> {
        debug!(session_key = %key.storage_key(), "loading durable agent state");
        Ok(self.states.read().await.get(key).cloned())
    }

    async fn delete_state(&self, key: &SessionKey) -> Result<(), AgentSessionError> {
        debug!(session_key = %key.storage_key(), "deleting durable agent state");
        self.states.write().await.remove(key);
        Ok(())
    }

    async fn list_keys(&self) -> Result<Vec<SessionKey>, AgentSessionError> {
        Ok(self.states.read().await.keys().cloned().collect())
    }

    fn health(&self) -> AgentSessionHealth {
        AgentSessionHealth {
            state: AgentSessionHealthState::Healthy,
            reason: "in-memory session store is available".to_string(),
            stored_state_count: self
                .states
                .try_read()
                .map(|states| states.len())
                .unwrap_or(0),
        }
    }

    fn snapshot(&self) -> AgentSessionStoreSnapshot {
        match self.states.try_read() {
            Ok(states) => AgentSessionStoreSnapshot {
                health: AgentSessionHealth {
                    state: AgentSessionHealthState::Healthy,
                    reason: "snapshot captured".to_string(),
                    stored_state_count: states.len(),
                },
                stored_keys: states.keys().map(SessionKey::storage_key).collect(),
            },
            Err(_) => {
                warn!("agent session store snapshot skipped because the store is locked");
                AgentSessionStoreSnapshot {
                    health: AgentSessionHealth {
                        state: AgentSessionHealthState::Degraded,
                        reason: "store lock is busy".to_string(),
                        stored_state_count: 0,
                    },
                    stored_keys: Vec::new(),
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "runtime_context_tests.rs"]
mod runtime_context_tests;
