//! `SKILL.md` frontmatter parser (Builder for `ParsedSkillMd`).
//!
//! Parses agentskills.io YAML frontmatter, optional macaca/openclaw
//! metadata blocks, and invocation policy flags. All functions are pure string
//! transforms suitable for unit testing without filesystem I/O.

use serde::Deserialize;
use serde_yaml::Value;

use macaca_proto::{MacacaError, MacacaResult};

use super::metadata::{
    ParsedSkillMd, SkillInstallSpec, SkillInvocationPolicy, SkillMcpServerConfig, SkillMetadata,
};

/// YAML frontmatter name/description pair from a `SKILL.md` file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SkillMdFrontmatter {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
}

/// Parse YAML frontmatter from a `SKILL.md` file.
pub(crate) fn parse_frontmatter(content: &str) -> MacacaResult<SkillMdFrontmatter> {
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

/// Parse raw YAML frontmatter mapping between `---` delimiters.
pub(crate) fn parse_frontmatter_value(content: &str) -> MacacaResult<Value> {
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

/// Extract the markdown body from a `SKILL.md` file (everything after frontmatter).
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

    let body_start = 3 + end_idx + 4;
    if body_start < content.len() {
        Ok(content[body_start..].trim().to_string())
    } else {
        Ok(String::new())
    }
}

/// Parse `SKILL.md` content into `(name, description, body)`.
pub fn parse_skill_md(content: &str) -> MacacaResult<(String, String, String)> {
    let fm = parse_frontmatter(content)?;
    let body = extract_body(content)?;
    Ok((fm.name, fm.description, body))
}

/// Parse a `SKILL.md` file into full runtime metadata and body.
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

/// Parse optional macaca/openclaw metadata block from frontmatter.
pub(crate) fn parse_skill_metadata(content: &str) -> MacacaResult<Option<SkillMetadata>> {
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

/// Resolve metadata block from known provider-neutral namespaces.
fn metadata_block(frontmatter: &Value) -> Option<&Value> {
    let metadata = value_get(frontmatter, "metadata")?;
    value_get(metadata, "macaca").or_else(|| value_get(metadata, "openclaw"))
}
