use std::collections::HashMap;

use macaca_skill::SkillSnapshot;

use crate::skill_mcp_mapping_registry::{
    default_skill_mcp_mapping_registry, SkillMcpMappingRegistry,
};
use crate::env_bridge::McpEnvApplyOutcome;
use crate::mcp_runtime::{
    definitions_from_skill_snapshot_with_registry, McpDefinitionSource, McpRegistryConfig,
    McpServerDefinition,
};

/// Thin builder that keeps process-wide MCP env application behind a stable
/// runtime-host boundary.
pub struct RuntimeEnvBuilder;

impl RuntimeEnvBuilder {
    pub fn apply_process_env(env: &HashMap<String, String>) -> HashMap<String, McpEnvApplyOutcome> {
        #[allow(deprecated)]
        {
            crate::env_bridge::apply_mcp_env(env)
        }
    }
}

/// Thin factory that centralizes MCP definition construction entry points.
pub struct McpServerFactory<'a> {
    registry: &'a SkillMcpMappingRegistry,
}

impl<'a> McpServerFactory<'a> {
    pub fn new(registry: &'a SkillMcpMappingRegistry) -> Self {
        Self { registry }
    }

    /// Factory bound to the process-wide bundled skill MCP mapping registry.
    pub fn with_bundled_mapping_registry() -> Self {
        Self {
            registry: default_skill_mcp_mapping_registry(),
        }
    }

    pub fn from_registry_config(
        &self,
        config: McpRegistryConfig,
        source: McpDefinitionSource,
    ) -> Result<Vec<McpServerDefinition>, String> {
        config
            .mcp_servers
            .into_iter()
            .map(|(id, entry)| {
                let mut definition = entry.into_definition_with_registry(id, self.registry)?;
                definition.source = source.clone();
                Ok(definition)
            })
            .collect()
    }

    pub fn from_skill_snapshot(&self, snapshot: &SkillSnapshot) -> Vec<McpServerDefinition> {
        definitions_from_skill_snapshot_with_registry(snapshot, self.registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_runtime::McpLifecycleScope;
    use macaca_framework::mcp::McpSessionMode;
    use macaca_skill::{
        SkillInstallSpec, SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry, SkillSourceScope,
    };
    use std::path::PathBuf;

    #[test]
    fn factory_builds_registry_definitions_with_source() {
        let config: McpRegistryConfig = serde_yaml::from_str(
            r#"
mcpServers:
  browser:
    transport: stdio
    command: playwright-mcp
    args: ["--headless"]
    requiredBins: ["playwright-mcp"]
"#,
        )
        .unwrap();

        let definitions = McpServerFactory::with_bundled_mapping_registry()
            .from_registry_config(config, McpDefinitionSource::App)
            .unwrap();

        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].source, McpDefinitionSource::App);
        assert_eq!(definitions[0].lifecycle, McpLifecycleScope::Session);
        assert_eq!(definitions[0].session_mode, McpSessionMode::Stateful);
    }

    #[test]
    fn factory_builds_skill_snapshot_definitions() {
        let snapshot = SkillSnapshot {
            agent: "researcher".into(),
            prompt: String::new(),
            skills: vec![SkillSnapshotEntry {
                name: "playwright-mcp".into(),
                description: "Browser".into(),
                source_location: PathBuf::from("/tmp/playwright/SKILL.md"),
                source_base_dir: PathBuf::from("/tmp/playwright"),
                location: PathBuf::from("/tmp/playwright/SKILL.md"),
                base_dir: PathBuf::from("/tmp/playwright"),
                source: "test".into(),
                source_scope: SkillSourceScope::MacacaCentral,
                primary_env: None,
                required_env: Vec::new(),
                install: vec![SkillInstallSpec {
                    kind: "npm".into(),
                    package: Some("@playwright/mcp".into()),
                    bins: vec!["playwright-mcp".into()],
                    ..Default::default()
                }],
                mcp_servers: vec![SkillMcpServerConfig {
                    id: "browser".into(),
                    command: "playwright-mcp".into(),
                    args: vec!["--headless".into()],
                    transport: "stdio".into(),
                    tool_prefix: None,
                }],
            }],
            filtered: Vec::new(),
            truncated: false,
            compact: false,
            version: 1,
        };

        let definitions = McpServerFactory::with_bundled_mapping_registry().from_skill_snapshot(&snapshot);
        assert_eq!(definitions.len(), 2);
        assert!(definitions
            .iter()
            .all(|definition| definition.session_mode == McpSessionMode::Stateful));
    }
}
