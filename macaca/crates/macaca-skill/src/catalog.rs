//! Skill Catalog — progressive disclosure per agentskills.io spec.
//!
//! Implements the three-tier loading strategy:
//! - **Tier 1 (Catalog)**: Name + description loaded at session start (~50-100 tokens/skill).
//! - **Tier 2 (Instructions)**: Full SKILL.md body loaded on activation (<5k tokens).
//! - **Tier 3 (Resources)**: Scripts, references, assets loaded on demand.
//!
//! The catalog keeps the base context small while giving the agent access
//! to specialized knowledge on demand.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use macaca_proto::{MacacaError, MacacaResult};

use crate::agent_skill::{AgentSkill, ActivatedSkill};
use crate::discovery;

/// Entry in the skill catalog (tier 1 — lightweight metadata).
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Path to the SKILL.md file.
    pub location: PathBuf,
}

impl From<&AgentSkill> for CatalogEntry {
    fn from(skill: &AgentSkill) -> Self {
        Self {
            name: skill.name.clone(),
            description: skill.description.clone(),
            location: skill.location.clone(),
        }
    }
}

/// Skill Catalog — manages available skills with progressive disclosure.
pub struct SkillCatalog {
    /// Skills indexed by name.
    skills: HashMap<String, AgentSkill>,
    /// Set of skills activated in this session (for deduplication).
    activated: HashMap<String, ActivatedSkill>,
}

impl SkillCatalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            activated: HashMap::new(),
        }
    }

    /// Discover and populate the catalog from standard directories.
    ///
    /// Scans project-level, user-level, and Agent OS central skill stores.
    pub async fn discover(
        project_dir: Option<&Path>,
        client_name: Option<&str>,
        extra_dirs: &[PathBuf],
    ) -> MacacaResult<Self> {
        let discovered = discovery::discover_skills(project_dir, client_name, extra_dirs).await?;
        let mut catalog = Self::new();
        for ds in discovered {
            catalog.skills.insert(ds.skill.name.clone(), ds.skill);
        }
        Ok(catalog)
    }

    /// Load skills from a specific directory into the catalog.
    pub async fn load_from_directory(&mut self, dir: impl AsRef<Path>) -> MacacaResult<usize> {
        let skills = discovery::scan_skills_directory(dir.as_ref()).await?;
        let count = skills.len();
        for skill in skills {
            self.skills.insert(skill.name.clone(), skill);
        }
        Ok(count)
    }

    /// Register a skill directly.
    pub fn register(&mut self, skill: AgentSkill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a catalog entry by name (tier 1 — metadata only).
    pub fn get(&self, name: &str) -> Option<CatalogEntry> {
        self.skills.get(name).map(CatalogEntry::from)
    }

    /// Get the underlying AgentSkill by name.
    pub fn get_skill(&self, name: &str) -> Option<&AgentSkill> {
        self.skills.get(name)
    }

    /// Tier 1: Get the full catalog (name + description for all skills).
    ///
    /// Returns entries suitable for injecting into system prompt or
    /// tool descriptions. Cost: ~50-100 tokens per skill.
    pub fn catalog(&self) -> Vec<CatalogEntry> {
        let mut entries: Vec<CatalogEntry> = self.skills.values().map(CatalogEntry::from).collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    /// Tier 1: Render the catalog as a compact string for prompt injection.
    ///
    /// Format: XML-like structure per agentskills.io recommendation.
    pub fn catalog_prompt(&self) -> String {
        let entries = self.catalog();
        if entries.is_empty() {
            return String::new();
        }

        let mut out = String::from("<available_skills>\n");
        for entry in &entries {
            out.push_str("  <skill>\n");
            out.push_str(&format!("    <name>{}</name>\n", entry.name));
            out.push_str(&format!("    <description>{}</description>\n", entry.description));
            out.push_str(&format!("    <location>{}</location>\n", entry.location.display()));
            out.push_str("  </skill>\n");
        }
        out.push_str("</available_skills>");
        out
    }

    /// Tier 2: Activate a skill (load full instruction content).
    ///
    /// Returns the activated skill with full content. Caches the result
    /// to avoid re-reading on subsequent activations.
    pub async fn activate(&mut self, name: &str) -> MacacaResult<&ActivatedSkill> {
        // Check if already activated (deduplication).
        if self.activated.contains_key(name) {
            return self
                .activated
                .get(name)
                .ok_or_else(|| MacacaError::NotFound(format!("Skill '{name}' not found")));
        }

        let skill = self
            .skills
            .get(name)
            .ok_or_else(|| MacacaError::NotFound(format!("Skill '{name}' not in catalog")))?;

        let activated = skill.load_content().await?;
        self.activated.insert(name.to_string(), activated);

        Ok(self.activated.get(name).unwrap())
    }

    /// Check if a skill has been activated in this session.
    pub fn is_activated(&self, name: &str) -> bool {
        self.activated.contains_key(name)
    }

    /// Get a previously activated skill (does not load from disk).
    pub fn get_activated(&self, name: &str) -> Option<&ActivatedSkill> {
        self.activated.get(name)
    }

    /// List all skill names in the catalog.
    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.skills.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Number of skills in the catalog.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Number of activated skills.
    pub fn activated_count(&self) -> usize {
        self.activated.len()
    }

    /// Remove a skill from the catalog.
    pub fn remove(&mut self, name: &str) -> bool {
        self.activated.remove(name);
        self.skills.remove(name).is_some()
    }
}

impl Default for SkillCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_skill_dir(base: &Path, name: &str, desc: &str) -> PathBuf {
        let skill_dir = base.join(name);
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {desc}\n---\n# {name}\n\nInstructions for {name}."),
        )
        .await
        .unwrap();
        skill_dir
    }

    #[tokio::test]
    async fn catalog_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        make_skill_dir(dir.path(), "alpha", "Alpha skill").await;
        make_skill_dir(dir.path(), "beta", "Beta skill").await;

        let mut catalog = SkillCatalog::new();
        let loaded = catalog.load_from_directory(dir.path()).await.unwrap();
        assert_eq!(loaded, 2);
        assert_eq!(catalog.count(), 2);

        // Tier 1: catalog entries.
        let entries = catalog.catalog();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[1].name, "beta");

        // Tier 1: catalog prompt.
        let prompt = catalog.catalog_prompt();
        assert!(prompt.contains("<available_skills>"));
        assert!(prompt.contains("alpha"));
        assert!(prompt.contains("beta"));
    }

    #[tokio::test]
    async fn activate_skill() {
        let dir = tempfile::tempdir().unwrap();
        make_skill_dir(dir.path(), "golang", "Go patterns").await;

        let mut catalog = SkillCatalog::new();
        catalog.load_from_directory(dir.path()).await.unwrap();

        assert!(!catalog.is_activated("golang"));

        // Tier 2: activate.
        let activated = catalog.activate("golang").await.unwrap();
        assert_eq!(activated.name, "golang");
        assert!(activated.content.contains("Instructions for golang"));

        assert!(catalog.is_activated("golang"));
        assert_eq!(catalog.activated_count(), 1);

        // Re-activation uses cache.
        let activated2 = catalog.activate("golang").await.unwrap();
        assert_eq!(activated2.name, "golang");
    }

    #[tokio::test]
    async fn activate_nonexistent() {
        let mut catalog = SkillCatalog::new();
        let err = catalog.activate("nope").await.unwrap_err();
        assert!(err.to_string().contains("not in catalog"));
    }

    #[tokio::test]
    async fn empty_catalog_prompt() {
        let catalog = SkillCatalog::new();
        assert!(catalog.catalog_prompt().is_empty());
    }

    #[tokio::test]
    async fn remove_skill() {
        let dir = tempfile::tempdir().unwrap();
        make_skill_dir(dir.path(), "removable", "Will be removed").await;

        let mut catalog = SkillCatalog::new();
        catalog.load_from_directory(dir.path()).await.unwrap();
        assert_eq!(catalog.count(), 1);

        assert!(catalog.remove("removable"));
        assert_eq!(catalog.count(), 0);
        assert!(!catalog.remove("removable")); // Already gone.
    }
}
