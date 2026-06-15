//! Skill registry — discovers, loads, and manages installed skills.

use crate::definition::SkillDefinition;
use crate::snapshot::SkillRegistrySnapshot;
use std::collections::HashMap;

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

    /// Capture executable skill definitions as a reloadable snapshot.
    pub fn snapshot(&self) -> SkillRegistrySnapshot {
        SkillRegistrySnapshot::new(self.skills.values().cloned().collect())
    }

    /// Replace current registry state from a snapshot.
    pub fn reload_from_snapshot(&mut self, snapshot: SkillRegistrySnapshot) {
        self.skills.clear();
        for skill in snapshot.skills {
            self.register(skill);
        }
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

    #[test]
    fn snapshot_and_reload_registry() {
        let mut reg = SkillRegistry::new();
        reg.register(make_skill("a"));
        reg.register(make_skill("b"));

        let snapshot = reg.snapshot();
        assert_eq!(snapshot.len(), 2);

        let mut restored = SkillRegistry::new();
        restored.reload_from_snapshot(snapshot);
        assert!(restored.get("a").is_some());
        assert!(restored.get("b").is_some());
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

        let mut toolset = crate::ExecutableSkillToolSet::new();
        let loaded = toolset.load_from_directory(dir.path()).await.unwrap();
        let snapshot = toolset.snapshot();
        assert_eq!(loaded, 1);
        assert!(snapshot
            .skills
            .iter()
            .any(|skill| skill.name == "loaded-skill"));
    }

    #[tokio::test]
    async fn load_from_missing_directory() {
        let mut toolset = crate::ExecutableSkillToolSet::new();
        let err = toolset
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

        let mut toolset = crate::ExecutableSkillToolSet::new();
        let loaded = toolset.load_from_directory(dir.path()).await.unwrap();
        let snapshot = toolset.snapshot();
        // Only YAML skill loaded; SKILL.md subdirectory is skipped.
        assert_eq!(loaded, 1);
        assert!(snapshot
            .skills
            .iter()
            .any(|skill| skill.name == "shell-skill"));
        assert!(!snapshot
            .skills
            .iter()
            .any(|skill| skill.name == "shadcn-ui"));
    }

    #[test]
    fn executable_toolset_returns_named_tool() {
        let mut toolset = crate::ExecutableSkillToolSet::new();
        toolset.registry.register(make_skill("my-skill"));
        let tool = toolset.tool("my-skill").unwrap();
        assert_eq!(tool.name(), "my-skill");
    }

    #[test]
    fn executable_toolset_exports_all_tools() {
        let mut toolset = crate::ExecutableSkillToolSet::new();
        toolset.registry.register(make_skill("a"));
        toolset.registry.register(make_skill("b"));
        let tools = toolset.into_tools();
        assert_eq!(tools.len(), 2);
    }
}
