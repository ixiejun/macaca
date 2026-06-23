//! Memory service scope, identity, capability, and policy DTOs.
//!
//! These types define the isolation envelope for every memory command. Keeping
//! validation beside the scope DTO makes boundary rules auditable and prevents
//! individual providers from inventing divergent visibility semantics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AgentId, ApplicationId, MacacaError, MacacaResult, TraceContext};

/// Visibility classes supported by the memory fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryVisibility {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
}

/// Stable identity fields that tie a memory scope to the running Agent OS.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryIdentity {
    pub application_id: ApplicationId,
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
}

impl MemoryIdentity {
    /// Create an identity rooted at an application.
    pub fn new(application_id: ApplicationId) -> Self {
        Self {
            application_id,
            agent_id: None,
            agent_name: None,
            session_id: None,
            project_id: None,
        }
    }
}

/// Full scoping envelope for every memory-fabric request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryScope {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub namespace: Option<String>,
    pub identity: MemoryIdentity,
    pub visibility: MemoryVisibility,
}

impl MemoryScope {
    /// Create a new scope with only application identity and target visibility.
    pub fn new(application_id: ApplicationId, visibility: MemoryVisibility) -> Self {
        Self {
            tenant_id: None,
            user_id: None,
            namespace: None,
            identity: MemoryIdentity::new(application_id),
            visibility,
        }
    }

    /// Convenience constructor for agent-private memory keyed by concrete agent id.
    pub fn agent_private(application_id: ApplicationId, agent_id: AgentId) -> Self {
        Self::new(application_id, MemoryVisibility::AgentPrivate).agent_id(agent_id)
    }

    /// Convenience constructor for agent-private memory keyed by agent name.
    pub fn agent_private_named(
        application_id: ApplicationId,
        agent_name: impl Into<String>,
    ) -> Self {
        Self::new(application_id, MemoryVisibility::AgentPrivate).agent_name(agent_name)
    }

    /// Convenience constructor for session-shared memory.
    pub fn session_shared(application_id: ApplicationId, session_id: impl Into<String>) -> Self {
        Self::new(application_id, MemoryVisibility::SessionShared).session_id(session_id)
    }

    /// Convenience constructor for project-shared memory.
    pub fn project_shared(application_id: ApplicationId, project_id: impl Into<String>) -> Self {
        Self::new(application_id, MemoryVisibility::SessionShared).project_id(project_id)
    }

    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.identity.agent_id = Some(agent_id);
        self
    }

    pub fn agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.identity.agent_name = Some(agent_name.into());
        self
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.identity.session_id = Some(session_id.into());
        self
    }

    pub fn project_id(mut self, project_id: impl Into<String>) -> Self {
        self.identity.project_id = Some(project_id.into());
        self
    }

    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Validate the minimum identity required by the selected visibility.
    pub fn validate(&self) -> MacacaResult<()> {
        match self.visibility {
            MemoryVisibility::AgentPrivate => self.validate_agent_private(),
            MemoryVisibility::SessionShared => self.validate_session_shared(),
            MemoryVisibility::ApplicationShared => Ok(()),
            MemoryVisibility::UserScoped => self.require_non_empty(
                self.user_id.as_deref(),
                "UserScoped memory requires user_id",
            ),
            MemoryVisibility::GlobalSystem => Ok(()),
        }
    }

    /// Return the agent id if the scope already carries one.
    pub fn agent_id_value(&self) -> Option<AgentId> {
        self.identity.agent_id
    }

    fn validate_agent_private(&self) -> MacacaResult<()> {
        if self.identity.agent_id.is_none()
            && self
                .identity
                .agent_name
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(MacacaError::Memory(
                "AgentPrivate memory requires agent_id or agent_name".into(),
            ));
        }
        Ok(())
    }

    fn validate_session_shared(&self) -> MacacaResult<()> {
        if self
            .identity
            .session_id
            .as_deref()
            .is_none_or(str::is_empty)
            && self
                .identity
                .project_id
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(MacacaError::Memory(
                "SessionShared memory requires session_id or project_id".into(),
            ));
        }
        Ok(())
    }

    fn require_non_empty(&self, value: Option<&str>, message: &str) -> MacacaResult<()> {
        if value.is_none_or(str::is_empty) {
            return Err(MacacaError::Memory(message.into()));
        }
        Ok(())
    }
}

/// Capability bitmap for a memory provider or adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCapabilitySet {
    pub store: bool,
    pub search: bool,
    pub prompt: bool,
    pub lifecycle: bool,
    pub flush: bool,
    pub artifact: bool,
    pub governance: bool,
    pub knowledge: bool,
}

impl MemoryCapabilitySet {
    /// Helper for the common builtin case: the provider can write and recall memory.
    pub fn basic_store_search() -> Self {
        Self {
            store: true,
            search: true,
            ..Self::default()
        }
    }
}

/// Health/status snapshot returned by a `MemoryFacade`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStatusReport {
    pub provider_id: String,
    pub healthy: bool,
    pub capabilities: MemoryCapabilitySet,
    pub message: Option<String>,
}

impl MemoryStatusReport {
    /// Convenience constructor for a healthy provider status.
    pub fn healthy(provider_id: impl Into<String>, capabilities: MemoryCapabilitySet) -> Self {
        Self {
            provider_id: provider_id.into(),
            healthy: true,
            capabilities,
            message: None,
        }
    }
}

/// Provider-neutral policy hints for memory operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryPolicyHints {
    pub privacy_tier: Option<String>,
    pub max_results: Option<usize>,
    pub metadata: BTreeMap<String, String>,
}

pub(crate) fn validate_scope_and_trace(
    scope: &MemoryScope,
    trace: &TraceContext,
) -> MacacaResult<()> {
    scope.validate()?;
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "Memory service command requires trace_id".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_no_global_recall(scope: &MemoryScope) -> MacacaResult<()> {
    if matches!(
        scope.visibility,
        MemoryVisibility::ApplicationShared | MemoryVisibility::GlobalSystem
    ) {
        return Err(MacacaError::Memory(
            "Memory recall requires AgentPrivate or SessionShared scope".into(),
        ));
    }
    Ok(())
}
