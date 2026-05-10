use macaca_proto::{AgentId, ApplicationId, MacacaError, MacacaResult};
use serde::{Deserialize, Serialize};

/// Visibility classes supported by the memory fabric.
///
/// The enum expresses *who is allowed to observe and mutate a memory* rather
/// than where the memory is stored. Routing and provider selection may map the
/// same visibility to different concrete backends over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryVisibility {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
}

/// Stable identity fields that tie a memory scope to the running Agent OS.
///
/// The fields intentionally separate application, agent, session, and project
/// identity so routing/policy logic can widen or restrict access without
/// relying on ad-hoc string parsing later.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryIdentity {
    pub application_id: ApplicationId,
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
}

impl MemoryIdentity {
    /// Create an identity rooted at an application, leaving narrower fields unset.
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
///
/// `MemoryScope` is the main isolation primitive introduced by the proposal.
/// Callers construct it once and pass it through write/search/get/delete APIs.
/// Routers, adapters, and providers can then make decisions from the same
/// structured scope instead of inferring ownership from arbitrary metadata.
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
    ///
    /// This exists for call sites that do not yet have a stable `AgentId` at
    /// construction time but still need the private visibility contract.
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
    ///
    /// The current implementation reuses `SessionShared` visibility because the
    /// builtin fabric only differentiates "shared within collaborative context"
    /// versus strictly private visibility. Future routers may split this out.
    pub fn project_shared(application_id: ApplicationId, project_id: impl Into<String>) -> Self {
        Self::new(application_id, MemoryVisibility::SessionShared).project_id(project_id)
    }

    /// Attach a concrete agent id to the scope.
    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.identity.agent_id = Some(agent_id);
        self
    }

    /// Attach a human-readable agent name to the scope.
    pub fn agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.identity.agent_name = Some(agent_name.into());
        self
    }

    /// Attach a session id to the scope.
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.identity.session_id = Some(session_id.into());
        self
    }

    /// Attach a project id to the scope.
    pub fn project_id(mut self, project_id: impl Into<String>) -> Self {
        self.identity.project_id = Some(project_id.into());
        self
    }

    /// Attach a tenant boundary for future multi-tenant routing/governance.
    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Attach an end-user identity for user-scoped memory.
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Attach a logical namespace so callers can further partition memories.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Validate that the scope carries the minimum identity required by its visibility.
    ///
    /// This function is the first safety gate of the memory fabric. It prevents
    /// ambiguous requests like "AgentPrivate without an agent" from reaching a
    /// router or provider that would otherwise have to guess the owner.
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

    /// Agent-private visibility requires a concrete agent identity.
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

    /// Shared visibility requires a collaboration anchor so the scope does not
    /// accidentally become application-global.
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

    /// Shared helper for "must be present and non-empty" checks used by scoped visibilities.
    fn require_non_empty(&self, value: Option<&str>, message: &str) -> MacacaResult<()> {
        if value.is_none_or(str::is_empty) {
            return Err(MacacaError::Memory(message.into()));
        }
        Ok(())
    }
}
