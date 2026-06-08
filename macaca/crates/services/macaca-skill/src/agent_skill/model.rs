//! `AgentSkill` entity and tier-2 activation (`ActivatedSkill`).
//!
//! Tier-1 loads frontmatter metadata only; tier-2 reads the markdown body and
//! bundled resource listing on demand (progressive disclosure).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use macaca_proto::{MacacaError, MacacaResult};

use super::parser::{extract_body, parse_frontmatter, parse_skill_metadata};
use super::scope::SkillSourceScope;

/// A knowledge skill parsed from a `SKILL.md` file.
///
/// Distinct from [`SkillDefinition`](crate::SkillDefinition) (executable YAML skills).
/// Body content is not loaded until [`load_content`](Self::load_content) (tier-2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    /// Unique skill name (from YAML frontmatter).
    pub name: String,
    /// Human-readable description (from YAML frontmatter).
    pub description: String,
    /// Absolute path to the `SKILL.md` file (may be projected for model-facing use).
    pub location: PathBuf,
    /// Base directory containing the skill (parent of SKILL.md).
    pub base_dir: PathBuf,
    /// Resolved canonical path to `SKILL.md` (audit / file-policy).
    #[serde(default)]
    pub canonical_location: PathBuf,
    /// Resolved canonical skill base directory.
    #[serde(default)]
    pub canonical_base_dir: PathBuf,
    /// Source scope used for precedence and diagnostics.
    #[serde(default)]
    pub source_scope: SkillSourceScope,
    /// Human-readable source label.
    #[serde(default)]
    pub source: String,
    /// Optional homepage URL from frontmatter metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Optional UI emoji from frontmatter metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

impl AgentSkill {
    /// Load an `AgentSkill` from a `SKILL.md` file path (default extra source scope).
    pub async fn from_path(path: impl AsRef<Path>) -> MacacaResult<Self> {
        Self::from_path_with_source(path, SkillSourceScope::Extra, "extra").await
    }

    /// Load an `AgentSkill` from a `SKILL.md` file with source metadata.
    pub async fn from_path_with_source(
        path: impl AsRef<Path>,
        source_scope: SkillSourceScope,
        source: impl Into<String>,
    ) -> MacacaResult<Self> {
        let path = path.as_ref().to_path_buf();
        let source = source.into();
        info!(
            path = %path.display(),
            scope = ?source_scope,
            source = %source,
            "agent skill: loading SKILL.md metadata (tier-1)"
        );

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(MacacaError::Io)?;
        let frontmatter = parse_frontmatter(&content)?;
        let metadata = parse_skill_metadata(&content).ok().flatten();
        let base_dir = path
            .parent()
            .ok_or_else(|| MacacaError::Config("SKILL.md has no parent directory".into()))?
            .to_path_buf();
        let canonical_location = tokio::fs::canonicalize(&path)
            .await
            .unwrap_or_else(|_| path.clone());
        let canonical_base_dir = tokio::fs::canonicalize(&base_dir)
            .await
            .unwrap_or_else(|_| base_dir.clone());

        debug!(
            name = %frontmatter.name,
            canonical = %canonical_location.display(),
            "agent skill: tier-1 metadata parsed"
        );

        Ok(Self {
            name: frontmatter.name,
            description: frontmatter.description,
            location: path,
            base_dir,
            canonical_location,
            canonical_base_dir,
            source_scope,
            source,
            homepage: metadata.as_ref().and_then(|m| m.homepage.clone()),
            emoji: metadata.and_then(|m| m.emoji),
        })
    }

    /// Load full instruction content (tier-2 activation / progressive disclosure).
    pub async fn load_content(&self) -> MacacaResult<ActivatedSkill> {
        info!(
            name = %self.name,
            path = %self.location.display(),
            "agent skill: activating tier-2 body content"
        );
        let raw = tokio::fs::read_to_string(&self.location)
            .await
            .map_err(MacacaError::Io)?;
        let body = extract_body(&raw)?;
        let resources = self.list_resources().await;

        debug!(
            name = %self.name,
            body_chars = body.len(),
            resources = resources.len(),
            "agent skill: tier-2 activation complete"
        );

        Ok(ActivatedSkill {
            name: self.name.clone(),
            description: self.description.clone(),
            content: body,
            resources,
            base_dir: self.base_dir.clone(),
        })
    }

    /// List bundled resources in the skill directory (excluding `SKILL.md`).
    async fn list_resources(&self) -> Vec<PathBuf> {
        let mut resources = Vec::new();
        let Ok(mut entries) = tokio::fs::read_dir(&self.base_dir).await else {
            return resources;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name != "SKILL.md" {
                        resources.push(path);
                    }
                }
            } else if path.is_dir() {
                resources.push(path);
            }
        }
        resources.sort();
        resources
    }
}

/// A skill whose full content has been loaded (tier-2 activated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatedSkill {
    pub name: String,
    pub description: String,
    /// Full markdown instruction content (body of SKILL.md).
    pub content: String,
    /// Bundled resources (scripts, references, assets) in the skill directory.
    pub resources: Vec<PathBuf>,
    pub base_dir: PathBuf,
}
