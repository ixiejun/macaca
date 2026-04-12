//! Skill definition types for executable skills.
//!
//! Defines [`SkillDefinition`] and [`SkillEntryPoint`] for executable skills
//! backed by shell commands, MCP servers, or scripts. These are declared in
//! YAML files (`.yaml`/`.yml`).
//!
//! For knowledge skills (SKILL.md format per agentskills.io), see
//! [`agent_skill`](crate::agent_skill).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How a skill executes its functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SkillEntryPoint {
    /// Execute a shell command. Arguments are passed as JSON via stdin.
    #[serde(rename = "shell")]
    ShellCommand {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Connect to an MCP server that provides the skill's tools.
    #[serde(rename = "mcp")]
    McpServer {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Run a script file (Python, Node, etc.).
    #[serde(rename = "script")]
    Script {
        path: String,
        #[serde(default)]
        interpreter: Option<String>,
    },
}

/// Definition of a skill — metadata + entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Unique skill name (e.g., "web-search", "code-review").
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Skill version.
    #[serde(default = "default_version")]
    pub version: String,
    /// How to execute this skill.
    pub entry_point: SkillEntryPoint,
    /// JSON Schema for the skill's input parameters.
    #[serde(default = "default_schema")]
    pub parameters: Value,
}

fn default_version() -> String {
    "0.1.0".into()
}

fn default_schema() -> Value {
    serde_json::json!({"type": "object", "properties": {}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_definition_yaml_roundtrip() {
        let yaml = r#"
name: web-search
description: Search the web
version: "1.0.0"
entry_point:
  type: shell
  command: curl
  args: ["-s"]
parameters:
  type: object
  properties:
    query:
      type: string
  required: ["query"]
"#;
        let skill: SkillDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(skill.name, "web-search");
        assert_eq!(skill.version, "1.0.0");
        match &skill.entry_point {
            SkillEntryPoint::ShellCommand { command, args } => {
                assert_eq!(command, "curl");
                assert_eq!(args, &["-s"]);
            }
            _ => panic!("Expected ShellCommand"),
        }
    }

    #[test]
    fn skill_mcp_entry_point() {
        let yaml = r#"
name: context7
description: Documentation lookup
entry_point:
  type: mcp
  command: npx
  args: ["@context7/mcp-server"]
"#;
        let skill: SkillDefinition = serde_yaml::from_str(yaml).unwrap();
        match &skill.entry_point {
            SkillEntryPoint::McpServer { command, args } => {
                assert_eq!(command, "npx");
                assert!(args[0].contains("context7"));
            }
            _ => panic!("Expected McpServer"),
        }
    }

    #[test]
    fn skill_script_entry_point() {
        let yaml = r#"
name: analyzer
description: Analyze code
entry_point:
  type: script
  path: scripts/analyze.py
  interpreter: python3
"#;
        let skill: SkillDefinition = serde_yaml::from_str(yaml).unwrap();
        match &skill.entry_point {
            SkillEntryPoint::Script { path, interpreter } => {
                assert_eq!(path, "scripts/analyze.py");
                assert_eq!(interpreter.as_deref(), Some("python3"));
            }
            _ => panic!("Expected Script"),
        }
    }
}
