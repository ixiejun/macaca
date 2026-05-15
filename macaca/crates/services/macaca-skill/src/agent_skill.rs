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
use serde_yaml::Value;

use macaca_proto::{MacacaError, MacacaResult};

/// Source scope for an AgentSkills-compatible knowledge skill.
///
/// Lower priority value means higher precedence when duplicate names exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillSourceScope {
    /// Session/application workspace `skills/`.
    Workspace = 0,
    /// Project/application `.agents/skills/`.
    ProjectAgents = 1,
    /// Application `skills/`.
    Application = 2,
    /// Macaca central store `~/.macaca/skills/`.
    MacacaCentral = 3,
    /// Generic user skill store `~/.agent/skills/`.
    UserAgentGeneric = 4,
    /// Client-specific user stores such as `~/.claude/skills/`.
    UserClient = 5,
    /// Bundled skills shipped with Macaca.
    Bundled = 6,
    /// Extra configured directories.
    Extra = 7,
}

impl SkillSourceScope {
    /// Stable source label used in diagnostics and trace payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::ProjectAgents => "project_agents",
            Self::Application => "application",
            Self::MacacaCentral => "macaca_central",
            Self::UserAgentGeneric => "user_agent_generic",
            Self::UserClient => "user_client",
            Self::Bundled => "bundled",
            Self::Extra => "extra",
        }
    }
}

impl Default for SkillSourceScope {
    fn default() -> Self {
        Self::Extra
    }
}

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
    /// Resolved canonical path to `SKILL.md`.
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
    /// Load an `AgentSkill` from a `SKILL.md` file path.
    ///
    /// Reads the file, parses YAML frontmatter, and stores metadata.
    /// The body content is NOT loaded eagerly — use [`load_content`] for
    /// tier-2 activation.
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
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| MacacaError::Io(e))?;
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

        Ok(Self {
            name: frontmatter.name,
            description: frontmatter.description,
            location: path,
            base_dir,
            canonical_location,
            canonical_base_dir,
            source_scope,
            source: source.into(),
            homepage: metadata.as_ref().and_then(|m| m.homepage.clone()),
            emoji: metadata.and_then(|m| m.emoji),
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

/// Invocation policy parsed from standard AgentSkills frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillInvocationPolicy {
    /// Whether the skill can be exposed as a user command.
    pub user_invocable: bool,
    /// Whether the skill should be hidden from model prompt injection.
    pub disable_model_invocation: bool,
}

/// Prompt/command exposure computed for a skill entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillExposure {
    /// Include in runtime status/debug inventory.
    pub include_in_runtime_registry: bool,
    /// Include in `<available_skills>` prompt catalog.
    pub include_in_available_skills_prompt: bool,
    /// Expose as a user-invocable command when command routing exists.
    pub user_invocable: bool,
}

impl Default for SkillExposure {
    fn default() -> Self {
        Self {
            include_in_runtime_registry: true,
            include_in_available_skills_prompt: true,
            user_invocable: true,
        }
    }
}

impl Default for SkillInvocationPolicy {
    fn default() -> Self {
        Self {
            user_invocable: true,
            disable_model_invocation: false,
        }
    }
}

/// Runtime metadata used for gating and UI display.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMetadata {
    pub always: bool,
    pub skill_key: Option<String>,
    pub primary_env: Option<String>,
    pub emoji: Option<String>,
    pub homepage: Option<String>,
    pub os: Vec<String>,
    pub requires_bins: Vec<String>,
    pub requires_any_bins: Vec<String>,
    pub requires_env: Vec<String>,
    pub requires_config: Vec<String>,
    pub install: Vec<SkillInstallSpec>,
    pub mcp_servers: Vec<SkillMcpServerConfig>,
}

/// Installer metadata from a standard skill manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillInstallSpec {
    pub id: Option<String>,
    pub kind: String,
    pub package: Option<String>,
    pub module: Option<String>,
    pub formula: Option<String>,
    pub bins: Vec<String>,
    pub label: Option<String>,
}

/// Macaca-owned MCP server declaration attached to a knowledge skill.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMcpServerConfig {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub transport: String,
    pub tool_prefix: Option<String>,
}

/// A parsed skill plus runtime policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub skill: AgentSkill,
    pub metadata: SkillMetadata,
    pub invocation: SkillInvocationPolicy,
    pub exposure: SkillExposure,
}

/// Result of parsing the full `SKILL.md` frontmatter.
#[derive(Debug, Clone)]
pub struct ParsedSkillMd {
    pub name: String,
    pub description: String,
    pub body: String,
    pub metadata: SkillMetadata,
    pub invocation: SkillInvocationPolicy,
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
    let value = parse_frontmatter_value(content)?;
    let frontmatter: SkillMdFrontmatter = serde_yaml::from_value(value)
        .map_err(|e| MacacaError::Config(format!("Invalid SKILL.md frontmatter: {e}")))?;

    if frontmatter.name.is_empty() {
        return Err(MacacaError::Config(
            "SKILL.md frontmatter must include a 'name' field".into(),
        ));
    }

    Ok(frontmatter)
}

fn parse_frontmatter_value(content: &str) -> MacacaResult<Value> {
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

    serde_yaml::from_str(frontmatter_str)
        .map_err(|e| MacacaError::Config(format!("Invalid SKILL.md frontmatter: {e}")))
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

/// Parse a `SKILL.md` file into full runtime metadata.
pub fn parse_skill_md_full(content: &str) -> MacacaResult<ParsedSkillMd> {
    let fm = parse_frontmatter(content)?;
    let body = extract_body(content)?;
    Ok(ParsedSkillMd {
        name: fm.name,
        description: fm.description,
        body,
        metadata: parse_skill_metadata(content)?.unwrap_or_default(),
        invocation: parse_invocation_policy(content)?,
    })
}

fn value_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_mapping()?.get(Value::String(key.to_string()))
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn value_bool(value: Option<&Value>, default: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(default)
}

fn value_string_vec(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

fn value_mapping(value: Option<&Value>) -> Option<&serde_yaml::Mapping> {
    value?.as_mapping()
}

fn parse_install_specs(metadata: &Value) -> Vec<SkillInstallSpec> {
    let Some(Value::Sequence(items)) = value_get(metadata, "install") else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let kind = value_string(value_get(item, "kind"))?;
            Some(SkillInstallSpec {
                id: value_string(value_get(item, "id")),
                kind,
                package: value_string(value_get(item, "package")),
                module: value_string(value_get(item, "module")),
                formula: value_string(value_get(item, "formula")),
                bins: value_string_vec(value_get(item, "bins")),
                label: value_string(value_get(item, "label")),
            })
        })
        .collect()
}

fn parse_mcp_servers(metadata: &Value) -> Vec<SkillMcpServerConfig> {
    let Some(servers) = value_mapping(value_get(metadata, "mcpServers")) else {
        return Vec::new();
    };
    servers
        .iter()
        .filter_map(|(key, value)| {
            let id = key.as_str()?.trim();
            if id.is_empty() {
                return None;
            }
            let command = value_string(value_get(value, "command"))?;
            Some(SkillMcpServerConfig {
                id: id.to_string(),
                command,
                args: value_string_vec(value_get(value, "args")),
                transport: value_string(value_get(value, "transport"))
                    .unwrap_or_else(|| "stdio".to_string()),
                tool_prefix: value_string(value_get(value, "toolPrefix")),
            })
        })
        .collect()
}

fn metadata_block(frontmatter: &Value) -> Option<&Value> {
    let metadata = value_get(frontmatter, "metadata")?;
    value_get(metadata, "macaca").or_else(|| value_get(metadata, "openclaw"))
}

fn parse_skill_metadata(content: &str) -> MacacaResult<Option<SkillMetadata>> {
    let frontmatter = parse_frontmatter_value(content)?;
    let Some(metadata) = metadata_block(&frontmatter) else {
        return Ok(None);
    };
    let requires = value_get(metadata, "requires");
    Ok(Some(SkillMetadata {
        always: value_bool(value_get(metadata, "always"), false),
        skill_key: value_string(value_get(metadata, "skillKey")),
        primary_env: value_string(value_get(metadata, "primaryEnv")),
        emoji: value_string(value_get(metadata, "emoji")),
        homepage: value_string(value_get(metadata, "homepage")),
        os: value_string_vec(value_get(metadata, "os")),
        requires_bins: value_string_vec(requires.and_then(|r| value_get(r, "bins"))),
        requires_any_bins: value_string_vec(requires.and_then(|r| value_get(r, "anyBins"))),
        requires_env: value_string_vec(requires.and_then(|r| value_get(r, "env"))),
        requires_config: value_string_vec(requires.and_then(|r| value_get(r, "config"))),
        install: parse_install_specs(metadata),
        mcp_servers: parse_mcp_servers(metadata),
    }))
}

fn parse_invocation_policy(content: &str) -> MacacaResult<SkillInvocationPolicy> {
    let frontmatter = parse_frontmatter_value(content)?;
    Ok(SkillInvocationPolicy {
        user_invocable: value_bool(value_get(&frontmatter, "user-invocable"), true),
        disable_model_invocation: value_bool(
            value_get(&frontmatter, "disable-model-invocation"),
            false,
        ),
    })
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

    #[test]
    fn parse_skill_md_full_macaca_mcp_servers() {
        let md = r#"---
name: browser
description: Browser MCP
metadata:
  macaca:
    mcpServers:
      playwright:
        command: playwright-mcp
        args: [--headless]
        transport: stdio
        toolPrefix: browser_
---
body"#;
        let parsed = parse_skill_md_full(md).unwrap();
        assert_eq!(parsed.metadata.mcp_servers.len(), 1);
        let server = &parsed.metadata.mcp_servers[0];
        assert_eq!(server.id, "playwright");
        assert_eq!(server.command, "playwright-mcp");
        assert_eq!(server.args, vec!["--headless"]);
        assert_eq!(server.transport, "stdio");
        assert_eq!(server.tool_prefix.as_deref(), Some("browser_"));
    }

    #[test]
    fn parse_skill_md_full_openclaw_install_metadata() {
        let md = r#"---
name: playwright-mcp
description: Browser automation
metadata:
  openclaw:
    install:
      - id: npm-playwright-mcp
        kind: npm
        package: "@playwright/mcp"
        bins: [playwright-mcp]
        label: Install Playwright MCP
---
body"#;
        let parsed = parse_skill_md_full(md).unwrap();
        assert_eq!(parsed.metadata.install.len(), 1);
        let install = &parsed.metadata.install[0];
        assert_eq!(install.kind, "npm");
        assert_eq!(install.package.as_deref(), Some("@playwright/mcp"));
        assert_eq!(install.bins, vec!["playwright-mcp"]);
    }
}
