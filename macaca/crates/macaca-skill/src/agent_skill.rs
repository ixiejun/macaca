//! Agent Skill — represents a SKILL.md knowledge skill per the agentskills.io spec.
//!
//! Agent Skills are folders containing a `SKILL.md` file with YAML frontmatter
//! (name + description) and a markdown body of instructions. They are NOT
//! executable tools — they provide knowledge/instructions that get injected
//! into an AI agent's context via progressive disclosure.
//!
//! Agent OS discovers these skills, builds a catalog, and can provision them
//! to compatible clients (e.g., Claude Code, Cursor).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use macaca_proto::{MacacaError, MacacaResult};

/// A knowledge skill parsed from a `SKILL.md` file.
///
/// This is distinct from [`SkillDefinition`](crate::SkillDefinition) which
/// represents executable skills (shell commands, MCP servers, scripts).
/// An `AgentSkill` contains instructions that are injected into an agent's
/// context, not executed as a subprocess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    /// Unique skill name (from YAML frontmatter).
    pub name: String,
    /// Human-readable description (from YAML frontmatter).
    pub description: String,
    /// Absolute path to the `SKILL.md` file.
    pub location: PathBuf,
    /// The base directory containing the skill (parent of SKILL.md).
    pub base_dir: PathBuf,
}

impl AgentSkill {
    /// Load an `AgentSkill` from a `SKILL.md` file path.
    ///
    /// Reads the file, parses YAML frontmatter, and stores metadata.
    /// The body content is NOT loaded eagerly — use [`load_content`] for
    /// tier-2 activation.
    pub async fn from_path(path: impl AsRef<Path>) -> MacacaResult<Self> {
        let path = path.as_ref().to_path_buf();
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| MacacaError::Io(e))?;
        let frontmatter = parse_frontmatter(&content)?;
        let base_dir = path
            .parent()
            .ok_or_else(|| MacacaError::Config("SKILL.md has no parent directory".into()))?
            .to_path_buf();

        Ok(Self {
            name: frontmatter.name,
            description: frontmatter.description,
            location: path,
            base_dir,
        })
    }

    /// Load the full instruction content (tier-2 activation).
    ///
    /// Reads the SKILL.md file and returns the markdown body after
    /// stripping YAML frontmatter.
    pub async fn load_content(&self) -> MacacaResult<ActivatedSkill> {
        let raw = tokio::fs::read_to_string(&self.location)
            .await
            .map_err(|e| MacacaError::Io(e))?;
        let body = extract_body(&raw)?;
        let resources = self.list_resources().await;

        Ok(ActivatedSkill {
            name: self.name.clone(),
            description: self.description.clone(),
            content: body,
            resources,
            base_dir: self.base_dir.clone(),
        })
    }

    /// List bundled resources (scripts, references, assets) in the skill
    /// directory, excluding SKILL.md itself.
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
                // Include subdirectory paths (e.g., scripts/, references/)
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
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Full markdown instruction content (body of SKILL.md).
    pub content: String,
    /// Bundled resources (scripts, references, assets) in the skill directory.
    pub resources: Vec<PathBuf>,
    /// Base directory of the skill.
    pub base_dir: PathBuf,
}

/// YAML frontmatter from a SKILL.md file.
#[derive(Debug, Clone, Deserialize)]
struct SkillMdFrontmatter {
    name: String,
    #[serde(default)]
    description: String,
}

/// Parse YAML frontmatter from a SKILL.md file.
///
/// SKILL.md format:
/// ```text
/// ---
/// name: my-skill
/// description: What this skill does
/// ---
/// # My Skill
///
/// Instructions for the agent...
/// ```
fn parse_frontmatter(content: &str) -> MacacaResult<SkillMdFrontmatter> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Err(MacacaError::Config(
            "SKILL.md must start with YAML frontmatter (---)".into(),
        ));
    }

    let after_first = &content[3..];
    let end_idx = after_first.find("\n---").ok_or_else(|| {
        MacacaError::Config("SKILL.md missing closing frontmatter delimiter (---)".into())
    })?;

    let frontmatter_str = &after_first[..end_idx];

    // Lenient parsing: try to handle common YAML issues
    let frontmatter: SkillMdFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| MacacaError::Config(format!("Invalid SKILL.md frontmatter: {e}")))?;

    if frontmatter.name.is_empty() {
        return Err(MacacaError::Config(
            "SKILL.md frontmatter must include a 'name' field".into(),
        ));
    }

    Ok(frontmatter)
}

/// Extract the markdown body from a SKILL.md file (everything after frontmatter).
pub fn extract_body(content: &str) -> MacacaResult<String> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Err(MacacaError::Config(
            "SKILL.md must start with YAML frontmatter (---)".into(),
        ));
    }

    let after_first = &content[3..];
    let end_idx = after_first.find("\n---").ok_or_else(|| {
        MacacaError::Config("SKILL.md missing closing frontmatter delimiter (---)".into())
    })?;

    let body_start = 3 + end_idx + 4; // skip "\n---"
    if body_start < content.len() {
        Ok(content[body_start..].trim().to_string())
    } else {
        Ok(String::new())
    }
}

/// Parse a SKILL.md file content into name, description, and body.
///
/// This is a convenience function that combines frontmatter parsing
/// and body extraction.
pub fn parse_skill_md(content: &str) -> MacacaResult<(String, String, String)> {
    let fm = parse_frontmatter(content)?;
    let body = extract_body(content)?;
    Ok((fm.name, fm.description, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_basic() {
        let md = "---\nname: golang\ndescription: Go patterns\n---\n# Go\nContent here.";
        let fm = parse_frontmatter(md).unwrap();
        assert_eq!(fm.name, "golang");
        assert_eq!(fm.description, "Go patterns");
    }

    #[test]
    fn extract_body_basic() {
        let md = "---\nname: golang\ndescription: Go patterns\n---\n# Go\n\nUse chi router.";
        let body = extract_body(md).unwrap();
        assert!(body.contains("chi router"));
        assert!(body.starts_with("# Go"));
    }

    #[test]
    fn extract_body_empty() {
        let md = "---\nname: empty\n---\n";
        let body = extract_body(md).unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn parse_skill_md_tuple() {
        let md = "---\nname: test\ndescription: Test skill\n---\nDo the thing.";
        let (name, desc, body) = parse_skill_md(md).unwrap();
        assert_eq!(name, "test");
        assert_eq!(desc, "Test skill");
        assert_eq!(body, "Do the thing.");
    }

    #[test]
    fn parse_frontmatter_no_delimiter() {
        let md = "# Just markdown\nNo frontmatter.";
        let err = parse_frontmatter(md).unwrap_err();
        assert!(err.to_string().contains("frontmatter"));
    }

    #[test]
    fn parse_frontmatter_missing_closing() {
        let md = "---\nname: broken\n# No closing";
        let err = parse_frontmatter(md).unwrap_err();
        assert!(err.to_string().contains("closing"));
    }

    #[test]
    fn parse_frontmatter_empty_name() {
        let md = "---\nname: \"\"\ndescription: no name\n---\nbody";
        let err = parse_frontmatter(md).unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[tokio::test]
    async fn agent_skill_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("test-skill");
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: A test\n---\n# Test\nDo stuff.",
        )
        .await
        .unwrap();

        let skill = AgentSkill::from_path(skill_dir.join("SKILL.md"))
            .await
            .unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test");
        assert_eq!(skill.base_dir, skill_dir);
    }

    #[tokio::test]
    async fn agent_skill_load_content() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("golang");
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: golang\ndescription: Go patterns\n---\n# Go\nUse chi router.",
        )
        .await
        .unwrap();
        // Add a bundled resource
        tokio::fs::write(skill_dir.join("helpers.sh"), "#!/bin/bash\necho hi")
            .await
            .unwrap();

        let skill = AgentSkill::from_path(skill_dir.join("SKILL.md"))
            .await
            .unwrap();
        let activated = skill.load_content().await.unwrap();
        assert_eq!(activated.name, "golang");
        assert!(activated.content.contains("chi router"));
        assert_eq!(activated.resources.len(), 1);
    }
}
