//! Consumer-facing skill runtime facades.

use std::path::{Path, PathBuf};

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::Tool;

use crate::adapter::SkillToolAdapter;
use crate::catalog::SkillCatalog;
use crate::definition::SkillDefinition;
use crate::registry::SkillRegistry;
use crate::request::SkillSnapshotRequest;
use crate::runtime::{SkillRuntime, SkillSnapshot};
use crate::snapshot::SkillRegistrySnapshot;
use crate::source::SkillSourceSet;
use crate::tool::SkillTool;

/// Stable facade for building skill snapshots.
#[derive(Debug, Clone, Default)]
pub struct SkillRuntimeFacade {
    runtime: SkillRuntime,
}

impl SkillRuntimeFacade {
    pub fn new() -> Self {
        Self {
            runtime: SkillRuntime,
        }
    }

    pub async fn build_snapshot(
        &self,
        request: SkillSnapshotRequest,
    ) -> MacacaResult<SkillSnapshot> {
        self.runtime
            .build_snapshot(request.agent, request.options)
            .await
    }
}

/// Facade for loading executable YAML skills and exposing them as tools.
#[derive(Default)]
pub struct ExecutableSkillToolSet {
    pub(crate) registry: SkillRegistry,
}

impl ExecutableSkillToolSet {
    pub fn new() -> Self {
        Self {
            registry: SkillRegistry::new(),
        }
    }

    pub async fn load_from_directory(&mut self, dir: impl AsRef<Path>) -> MacacaResult<usize> {
        let definitions = load_executable_skill_definitions(dir.as_ref()).await?;
        let count = definitions.len();
        for definition in definitions {
            self.registry.register(definition);
        }
        Ok(count)
    }

    pub fn snapshot(&self) -> SkillRegistrySnapshot {
        self.registry.snapshot()
    }

    pub fn into_tools(self) -> Vec<Box<dyn Tool>> {
        self.registry
            .snapshot()
            .skills
            .into_iter()
            .map(|definition| {
                Box::new(SkillTool::from_adapter(SkillToolAdapter::local(definition)))
                    as Box<dyn Tool>
            })
            .collect()
    }

    pub fn tool(&self, name: &str) -> MacacaResult<SkillTool> {
        let definition = self
            .registry
            .get(name)
            .ok_or_else(|| MacacaError::NotFound(format!("Skill '{name}' not found")))?;
        Ok(SkillTool::from_adapter(SkillToolAdapter::local(
            definition.clone(),
        )))
    }
}

/// View over canonical skill source directories.
#[derive(Debug, Clone)]
pub struct SkillCatalogSourceView {
    sources: SkillSourceSet,
}

impl SkillCatalogSourceView {
    pub fn new(sources: SkillSourceSet) -> Self {
        Self { sources }
    }

    pub fn directories(&self) -> Vec<PathBuf> {
        self.sources
            .iter()
            .map(|source| source.root.clone())
            .collect()
    }

    pub async fn load_catalog(&self) -> MacacaResult<SkillCatalog> {
        let mut catalog = SkillCatalog::new();
        for source in self.sources.iter() {
            if source.root.exists() {
                catalog.load_from_directory(&source.root).await?;
            }
        }
        Ok(catalog)
    }
}

/// Load executable YAML skill definitions from one directory.
pub async fn load_executable_skill_definitions(dir: &Path) -> MacacaResult<Vec<SkillDefinition>> {
    if !dir.exists() {
        return Err(MacacaError::NotFound(format!(
            "Skills directory not found: {}",
            dir.display()
        )));
    }

    let mut definitions = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await.map_err(MacacaError::Io)?;
    while let Some(entry) = entries.next_entry().await.map_err(MacacaError::Io)? {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match serde_yaml::from_str::<SkillDefinition>(&content) {
                Ok(definition) => definitions.push(definition),
                Err(e) => tracing::warn!("failed to parse skill {:?}: {}", path, e),
            },
            Err(e) => tracing::warn!("failed to read {:?}: {}", path, e),
        }
    }
    definitions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(definitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_tools::{Tool, ToolCommand, ToolCommandExecutor};

    #[tokio::test]
    async fn executable_toolset_loads_yaml_and_exposes_tools() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("echo.yaml"),
            r#"
name: echo-skill
description: Echoes input
entry_point:
  type: shell
  command: echo
  args: ["hello"]
"#,
        )
        .await
        .unwrap();

        let mut toolset = ExecutableSkillToolSet::new();
        let loaded = toolset.load_from_directory(dir.path()).await.unwrap();
        assert_eq!(loaded, 1);

        let tool = toolset.tool("echo-skill").unwrap();
        assert_eq!(tool.name(), "echo-skill");
        let result =
            ToolCommandExecutor::execute_command(&tool, ToolCommand::new(serde_json::json!({})))
                .await
                .unwrap();
        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn runtime_facade_builds_snapshot_from_request() {
        let app = tempfile::tempdir().unwrap();
        let skill_dir = app.path().join("skills").join("writer");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: writer\ndescription: Writing\n---\nBody",
        )
        .await
        .unwrap();

        let request = SkillSnapshotRequest::builder("agent")
            .app_dir(Some(app.path().to_path_buf()))
            .build();
        let snapshot = SkillRuntimeFacade::new()
            .build_snapshot(request)
            .await
            .unwrap();
        assert_eq!(snapshot.skills.len(), 1);
        assert_eq!(snapshot.skills[0].name, "writer");
    }
}
