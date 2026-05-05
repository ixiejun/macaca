use macaca_proto::{AgentId, ApplicationId, MacacaError, MacacaResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryVisibility {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryIdentity {
    pub application_id: ApplicationId,
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
}

impl MemoryIdentity {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryScope {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub namespace: Option<String>,
    pub identity: MemoryIdentity,
    pub visibility: MemoryVisibility,
}

impl MemoryScope {
    pub fn new(application_id: ApplicationId, visibility: MemoryVisibility) -> Self {
        Self {
            tenant_id: None,
            user_id: None,
            namespace: None,
            identity: MemoryIdentity::new(application_id),
            visibility,
        }
    }

    pub fn agent_private(application_id: ApplicationId, agent_id: AgentId) -> Self {
        Self::new(application_id, MemoryVisibility::AgentPrivate).agent_id(agent_id)
    }

    pub fn agent_private_named(
        application_id: ApplicationId,
        agent_name: impl Into<String>,
    ) -> Self {
        Self::new(application_id, MemoryVisibility::AgentPrivate).agent_name(agent_name)
    }

    pub fn session_shared(application_id: ApplicationId, session_id: impl Into<String>) -> Self {
        Self::new(application_id, MemoryVisibility::SessionShared).session_id(session_id)
    }

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
