use macaca_proto::{MacacaError, MacacaResult};
use serde::{Deserialize, Serialize};

use crate::core::scope::{MemoryScope, MemoryVisibility};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorTopologyKey {
    pub application_id: String,
    pub namespace: String,
    pub unit: String,
}

impl VectorTopologyKey {
    pub fn new(
        application_id: impl Into<String>,
        namespace: impl Into<String>,
        unit: impl Into<String>,
    ) -> Self {
        Self {
            application_id: application_id.into(),
            namespace: namespace.into(),
            unit: unit.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorMemoryTopology {
    pub database: String,
    pub collection: String,
    pub shared_collection: Option<String>,
}

impl VectorMemoryTopology {
    pub fn new(
        database: impl Into<String>,
        collection: impl Into<String>,
        shared_collection: Option<String>,
    ) -> Self {
        Self {
            database: database.into(),
            collection: collection.into(),
            shared_collection,
        }
    }
}

pub trait VectorTopologyResolver: Send + Sync {
    fn resolve(&self, scope: &MemoryScope) -> MacacaResult<VectorMemoryTopology>;
}

#[derive(Debug, Clone, Default)]
pub struct DefaultVectorTopologyResolver;

impl VectorTopologyResolver for DefaultVectorTopologyResolver {
    fn resolve(&self, scope: &MemoryScope) -> MacacaResult<VectorMemoryTopology> {
        scope.validate()?;
        let database = format!(
            "app_{}",
            sanitize_identifier(&scope.identity.application_id.to_string())
        );
        let collection = match scope.visibility {
            MemoryVisibility::AgentPrivate => {
                let unit = scope
                    .identity
                    .agent_name
                    .clone()
                    .or_else(|| scope.identity.agent_id.map(|id| id.to_string()))
                    .ok_or_else(|| {
                        MacacaError::Memory(
                            "AgentPrivate scope requires agent_id or agent_name".into(),
                        )
                    })?;
                sanitize_identifier(&format!("agent_{unit}"))
            }
            MemoryVisibility::SessionShared => {
                let unit = scope
                    .identity
                    .session_id
                    .clone()
                    .or_else(|| scope.identity.project_id.clone())
                    .ok_or_else(|| {
                        MacacaError::Memory(
                            "SessionShared scope requires session_id or project_id".into(),
                        )
                    })?;
                sanitize_identifier(&format!("shared_{unit}"))
            }
            MemoryVisibility::ApplicationShared => "application_shared".into(),
            MemoryVisibility::UserScoped => {
                let unit = scope.user_id.clone().ok_or_else(|| {
                    MacacaError::Memory("UserScoped scope requires user_id".into())
                })?;
                sanitize_identifier(&format!("user_{unit}"))
            }
            MemoryVisibility::GlobalSystem => "global_system".into(),
        };
        let shared_collection =
            matches!(scope.visibility, MemoryVisibility::SessionShared).then(|| collection.clone());
        Ok(VectorMemoryTopology::new(
            database,
            collection,
            shared_collection,
        ))
    }
}

pub fn sanitize_identifier(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push('_');
        }
    }
    let trimmed = output.trim_matches('_');
    if trimmed.is_empty() {
        "default".into()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentId, ApplicationId};

    #[test]
    fn sanitizes_identifiers() {
        assert_eq!(sanitize_identifier("App-Name/1"), "app_name_1");
        assert_eq!(sanitize_identifier("___"), "default");
    }

    #[test]
    fn resolves_agent_private_topology() {
        let resolver = DefaultVectorTopologyResolver;
        let scope = MemoryScope::agent_private(ApplicationId::new(), AgentId::new());
        let topology = resolver.resolve(&scope).unwrap();

        assert!(topology.database.starts_with("app_"));
        assert!(topology.collection.starts_with("agent_"));
        assert!(topology.shared_collection.is_none());
    }

    #[test]
    fn resolves_session_shared_topology() {
        let resolver = DefaultVectorTopologyResolver;
        let scope = MemoryScope::session_shared(ApplicationId::new(), "session-1");
        let topology = resolver.resolve(&scope).unwrap();

        assert!(topology.database.starts_with("app_"));
        assert!(topology.collection.starts_with("shared_"));
        assert_eq!(
            topology.shared_collection,
            Some(topology.collection.clone())
        );
    }
}
