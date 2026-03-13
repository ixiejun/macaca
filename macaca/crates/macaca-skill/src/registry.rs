//! Skill registry — discovers, loads, and manages installed skills.

use std::collections::HashMap;
use std::path::Path;

use macaca_proto::{MacacaError, MacacaResult};

use crate::definition::SkillDefinition;
use crate::tool::SkillTool;

/// Registry of available skills.
pub struct SkillRegistry {
    skills: HashMap<String, SkillDefinition>,
}

impl SkillRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill definition.
    pub fn register(&mut self, skill: SkillDefinition) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    /// List all registered skill names.
    pub fn list(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    /// Number of registered skills.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Unregister a skill by name.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.skills.remove(name).is_some()
    }

    /// Load executable skills from a directory.
    ///
    /// Discovers `*.yaml` and `*.yml` files and parses them as `SkillDefinition`.
    /// For SKILL.md knowledge skills, use [`SkillCatalog`](crate::SkillCatalog) instead.
    ///
    /// Silently skips files that fail to parse.
    pub async fn load_from_directory(&mut self, dir: impl AsRef<Path>) -> MacacaResult<usize> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Err(MacacaError::NotFound(format!(
                "Skills directory not found: {}",
                dir.display()
            )));
        }

        let mut loaded = 0;
        let mut entries = tokio::fs::read_dir(dir).await.map_err(MacacaError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(MacacaError::Io)? {
            let path = entry.path();

            // Skip directories — SKILL.md subdirectories are handled by SkillCatalog.
            if path.is_dir() {
                continue;
            }

            // YAML skill files.
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("yaml") && ext != Some("yml") {
                continue;
            }

            match tokio::fs::read_to_string(&path).await {
                Ok(content) => match serde_yaml::from_str::<SkillDefinition>(&content) {
                    Ok(skill) => {
                        tracing::debug!(skill = %skill.name, "loaded skill from {:?}", path);
                        self.register(skill);
                        loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!("failed to parse skill {:?}: {}", path, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("failed to read {:?}: {}", path, e);
                }
            }
        }

        Ok(loaded)
    }

    /// Create a `Tool` instance from a registered skill.
    pub fn instantiate_tool(&self, name: &str) -> MacacaResult<SkillTool> {
        let skill = self
            .skills
            .get(name)
            .ok_or_else(|| MacacaError::NotFound(format!("Skill '{}' not found", name)))?;
        Ok(SkillTool::new(skill.clone()))
    }

    /// Create `Tool` instances for all registered skills.
    pub fn instantiate_all_tools(&self) -> Vec<Box<dyn macaca_tools::Tool>> {
        self.skills
            .values()
            .map(|s| Box::new(SkillTool::new(s.clone())) as Box<dyn macaca_tools::Tool>)
            .collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{SkillDefinition, SkillEntryPoint};
    use macaca_tools::Tool;

    fn make_skill(name: &str) -> SkillDefinition {
        SkillDefinition {
            name: name.into(),
            description: format!("{name} skill"),
            version: "0.1.0".into(),
            entry_point: SkillEntryPoint::ShellCommand {
                command: "echo".into(),
                args: vec![name.into()],
            },
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = SkillRegistry::new();
        reg.register(make_skill("test"));
        assert_eq!(reg.count(), 1);
        assert!(reg.get("test").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn list_skills() {
        let mut reg = SkillRegistry::new();
        reg.register(make_skill("a"));
        reg.register(make_skill("b"));
        let mut names = reg.list();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn unregister() {
        let mut reg = SkillRegistry::new();
        reg.register(make_skill("removable"));
        assert!(reg.unregister("removable"));
        assert_eq!(reg.count(), 0);
        assert!(!reg.unregister("removable")); // Already gone
    }

    #[tokio::test]
    async fn load_from_directory() {
        let dir = tempfile::tempdir().unwrap();

        // Write a valid skill YAML.
        let skill_yaml = r#"
name: loaded-skill
description: From directory
entry_point:
  type: shell
  command: echo
  args: ["hello"]
"#;
        tokio::fs::write(dir.path().join("skill.yaml"), skill_yaml)
            .await
            .unwrap();

        // Write an invalid file (should be skipped).
        tokio::fs::write(dir.path().join("bad.yaml"), "not valid yaml: [")
            .await
            .unwrap();

        // Write a non-yaml file (should be skipped).
        tokio::fs::write(dir.path().join("readme.txt"), "not a skill")
            .await
            .unwrap();

        let mut reg = SkillRegistry::new();
        let loaded = reg.load_from_directory(dir.path()).await.unwrap();
        assert_eq!(loaded, 1);
        assert!(reg.get("loaded-skill").is_some());
    }

    #[tokio::test]
    async fn load_from_missing_directory() {
        let mut reg = SkillRegistry::new();
        let err = reg
            .load_from_directory("/nonexistent/path")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn load_from_directory_skips_subdirectories() {
        let dir = tempfile::tempdir().unwrap();

        // YAML skill (should be loaded).
        let yaml = r#"
name: shell-skill
description: A shell skill
entry_point:
  type: shell
  command: echo
  args: ["hi"]
"#;
        tokio::fs::write(dir.path().join("shell.yaml"), yaml)
            .await
            .unwrap();

        // SKILL.md subdirectory (should be skipped by SkillRegistry).
        let skill_dir = dir.path().join("shadcn-ui");
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: shadcn-ui\ndescription: UI components\n---\nUse shadcn-ui.",
        )
        .await
        .unwrap();

        let mut reg = SkillRegistry::new();
        let loaded = reg.load_from_directory(dir.path()).await.unwrap();
        // Only YAML skill loaded; SKILL.md subdirectory is skipped.
        assert_eq!(loaded, 1);
        assert!(reg.get("shell-skill").is_some());
        assert!(reg.get("shadcn-ui").is_none());
    }

    #[test]
    fn instantiate_tool() {
        let mut reg = SkillRegistry::new();
        reg.register(make_skill("my-skill"));
        let tool = reg.instantiate_tool("my-skill").unwrap();
        assert_eq!(tool.name(), "my-skill");
    }

    #[test]
    fn instantiate_all_tools() {
        let mut reg = SkillRegistry::new();
        reg.register(make_skill("a"));
        reg.register(make_skill("b"));
        let tools = reg.instantiate_all_tools();
        assert_eq!(tools.len(), 2);
    }
}
