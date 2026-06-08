//! Agent card discovery Strategy implementations.
//!
//! **Design pattern:** Strategy — [`AgentCardResolver`] abstracts how an
//! [`AgentCard`] is obtained (file, HTTP, registry, …). [`FileCardResolver`]
//! is the filesystem-backed reference implementation for local deployments.

use async_trait::async_trait;
use tracing::{debug, warn};

use super::error::A2AError;
use super::types::AgentCard;

/// Resolves an `AgentCard` from some source (file, HTTP, registry, …).
#[async_trait]
pub trait AgentCardResolver: Send + Sync {
    /// Fetch the agent card.
    async fn resolve(&self) -> Result<AgentCard, A2AError>;
}

/// Load an `AgentCard` from a local JSON file.
pub struct FileCardResolver {
    path: std::path::PathBuf,
}

impl FileCardResolver {
    /// Create a new resolver that reads from `path`.
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl AgentCardResolver for FileCardResolver {
    async fn resolve(&self) -> Result<AgentCard, A2AError> {
        debug!(
            target: "macaca_framework::a2a::resolver",
            path = %self.path.display(),
            "FileCardResolver::resolve entry — loading AgentCard from filesystem"
        );

        let data = tokio::fs::read_to_string(&self.path).await.map_err(|e| {
            warn!(
                target: "macaca_framework::a2a::resolver",
                path = %self.path.display(),
                error = %e,
                "FileCardResolver::resolve failed — could not read agent card file"
            );
            A2AError::Discovery(format!(
                "Failed to read card file '{}': {e}",
                self.path.display()
            ))
        })?;

        let card: AgentCard = serde_json::from_str(&data).map_err(|e| {
            warn!(
                target: "macaca_framework::a2a::resolver",
                path = %self.path.display(),
                error = %e,
                "FileCardResolver::resolve failed — JSON parse error"
            );
            A2AError::Discovery(format!("Failed to parse agent card: {e}"))
        })?;

        debug!(
            target: "macaca_framework::a2a::resolver",
            path = %self.path.display(),
            agent_name = %card.name,
            skill_count = card.skills.len(),
            "FileCardResolver::resolve complete"
        );

        Ok(card)
    }
}
