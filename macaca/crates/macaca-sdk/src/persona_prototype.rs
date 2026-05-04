//! Prototype support for reusable agent personas.

use crate::persona::AgentPersona;

/// Section overrides applied when instantiating a persona prototype.
#[derive(Debug, Clone, Default)]
pub struct PersonaOverrides {
    pub bootstrap: Option<String>,
    pub identity: Option<String>,
    pub agents: Option<String>,
    pub soul: Option<String>,
    pub user: Option<String>,
    pub tools: Option<String>,
    pub heartbeat: Option<String>,
}

impl PersonaOverrides {
    /// Create empty overrides.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override identity.
    pub fn with_identity(mut self, identity: impl Into<String>) -> Self {
        self.identity = Some(identity.into());
        self
    }

    /// Override tool guidance.
    pub fn with_tools(mut self, tools: impl Into<String>) -> Self {
        self.tools = Some(tools.into());
        self
    }

    /// Override heartbeat guidance.
    pub fn with_heartbeat(mut self, heartbeat: impl Into<String>) -> Self {
        self.heartbeat = Some(heartbeat.into());
        self
    }
}

/// Reusable prototype for agent persona variants.
#[derive(Debug, Clone)]
pub struct PersonaPrototype {
    base: AgentPersona,
}

impl PersonaPrototype {
    /// Create a prototype from an existing persona.
    pub fn new(base: AgentPersona) -> Self {
        Self { base }
    }

    /// Access the immutable base persona.
    pub fn base(&self) -> &AgentPersona {
        &self.base
    }

    /// Instantiate a persona by cloning the base and applying overrides.
    pub fn instantiate(&self, overrides: PersonaOverrides) -> AgentPersona {
        let mut persona = self.base.clone();

        if let Some(value) = overrides.bootstrap {
            persona.bootstrap = Some(value);
        }
        if let Some(value) = overrides.identity {
            persona.identity = Some(value);
        }
        if let Some(value) = overrides.agents {
            persona.agents = Some(value);
        }
        if let Some(value) = overrides.soul {
            persona.soul = Some(value);
        }
        if let Some(value) = overrides.user {
            persona.user = Some(value);
        }
        if let Some(value) = overrides.tools {
            persona.tools = Some(value);
        }
        if let Some(value) = overrides.heartbeat {
            persona.heartbeat = Some(value);
        }

        persona
    }
}

impl From<AgentPersona> for PersonaPrototype {
    fn from(value: AgentPersona) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate_override_does_not_mutate_base() {
        let base = AgentPersona {
            identity: Some("Base identity".into()),
            tools: Some("Base tools".into()),
            ..Default::default()
        };
        let prototype = PersonaPrototype::new(base);

        let persona = prototype.instantiate(
            PersonaOverrides::new()
                .with_identity("Override identity")
                .with_heartbeat("Stay alive"),
        );

        assert_eq!(persona.identity.as_deref(), Some("Override identity"));
        assert_eq!(persona.tools.as_deref(), Some("Base tools"));
        assert_eq!(persona.heartbeat.as_deref(), Some("Stay alive"));
        assert_eq!(prototype.base().identity.as_deref(), Some("Base identity"));
        assert!(prototype.base().heartbeat.is_none());
    }
}
