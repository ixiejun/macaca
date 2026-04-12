//! Skill Loader — loads global and app-specific skills.

use std::path::{Path, PathBuf};

use macaca_proto::MacacaResult;

/// Default global skills directory.
pub fn global_skills_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("macaca")
        .join("skills")
}

/// Skill loader that merges global and app-specific skills.
pub struct SkillLoader {
    /// Global skills directory.
    global_dir: PathBuf,
    /// App-specific skills directory (optional).
    app_dir: Option<PathBuf>,
}

impl SkillLoader {
    /// Create a new skill loader with default global directory.
    pub fn new() -> Self {
        Self {
            global_dir: global_skills_dir(),
            app_dir: None,
        }
    }

    /// Create a skill loader with custom global directory.
    pub fn with_global_dir(global_dir: PathBuf) -> Self {
        Self {
            global_dir,
            app_dir: None,
        }
    }

    /// Set the app-specific skills directory.
    pub fn with_app_dir(mut self, app_dir: PathBuf) -> Self {
        self.app_dir = Some(app_dir);
        self
    }

    /// Get the global skills directory path.
    pub fn global_dir(&self) -> &Path {
        &self.global_dir
    }

    /// Get the app skills directory path (if set).
    pub fn app_dir(&self) -> Option<&Path> {
        self.app_dir.as_deref()
    }

    /// List all skill directories (global + app).
    pub fn list_skill_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Add global skills directory if it exists
        if self.global_dir.exists() {
            dirs.push(self.global_dir.clone());
        }

        // Add app skills directory if it exists
        if let Some(ref app_dir) = self.app_dir {
            if app_dir.exists() {
                dirs.push(app_dir.clone());
            }
        }

        dirs
    }

    /// Check if a skill exists (by name).
    pub fn skill_exists(&self, name: &str) -> bool {
        // Check app skills first (higher priority)
        if let Some(ref app_dir) = self.app_dir {
            if app_dir.join(name).join("SKILL.md").exists() {
                return true;
            }
        }

        // Then check global skills
        if self.global_dir.join(name).join("SKILL.md").exists() {
            return true;
        }

        false
    }

    /// Get the path to a skill (app skills take priority).
    pub fn get_skill_path(&self, name: &str) -> Option<PathBuf> {
        // Check app skills first (higher priority)
        if let Some(ref app_dir) = self.app_dir {
            let skill_path = app_dir.join(name);
            if skill_path.join("SKILL.md").exists() {
                return Some(skill_path);
            }
        }

        // Then check global skills
        let global_skill = self.global_dir.join(name);
        if global_skill.join("SKILL.md").exists() {
            return Some(global_skill);
        }

        None
    }

    /// List all available skill names (merged from global and app).
    pub fn list_skill_names(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();

        // Scan global skills
        if self.global_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.global_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("SKILL.md").exists() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            names.insert(name.to_string());
                        }
                    }
                }
            }
        }

        // Scan app skills
        if let Some(ref app_dir) = self.app_dir {
            if app_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(app_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("SKILL.md").exists() {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                names.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        names.into_iter().collect()
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_loader_new() {
        let loader = SkillLoader::new();
        // Global dir should be set
        assert!(loader.global_dir().to_string_lossy().contains("macaca"));
    }

    #[test]
    fn skill_loader_with_app_dir() {
        let temp_dir = std::env::temp_dir().join("macaca_skill_test");
        let loader = SkillLoader::new().with_app_dir(temp_dir.clone());
        assert_eq!(loader.app_dir(), Some(temp_dir.as_path()));
    }

    #[test]
    fn list_skill_names_empty() {
        let temp_global = std::env::temp_dir().join("macaca_skill_test_global_empty");
        let temp_app = std::env::temp_dir().join("macaca_skill_test_app_empty");
        std::fs::create_dir_all(&temp_global).ok();
        std::fs::create_dir_all(&temp_app).ok();

        let loader =
            SkillLoader::with_global_dir(temp_global.clone()).with_app_dir(temp_app.clone());

        let names = loader.list_skill_names();
        assert!(names.is_empty());

        let _ = std::fs::remove_dir_all(&temp_global);
        let _ = std::fs::remove_dir_all(&temp_app);
    }

    #[test]
    fn list_skill_names_with_skills() {
        let temp_global = std::env::temp_dir().join("macaca_skill_test_global");
        let temp_app = std::env::temp_dir().join("macaca_skill_test_app");

        // Create global skill
        let global_skill = temp_global.join("global-skill");
        std::fs::create_dir_all(&global_skill).ok();
        std::fs::write(global_skill.join("SKILL.md"), "# Global Skill").ok();

        // Create app skill
        let app_skill = temp_app.join("app-skill");
        std::fs::create_dir_all(&app_skill).ok();
        std::fs::write(app_skill.join("SKILL.md"), "# App Skill").ok();

        let loader =
            SkillLoader::with_global_dir(temp_global.clone()).with_app_dir(temp_app.clone());

        let names = loader.list_skill_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"global-skill".to_string()));
        assert!(names.contains(&"app-skill".to_string()));

        let _ = std::fs::remove_dir_all(&temp_global);
        let _ = std::fs::remove_dir_all(&temp_app);
    }

    #[test]
    fn app_skill_takes_priority() {
        let temp_global = std::env::temp_dir().join("macaca_skill_priority_global");
        let temp_app = std::env::temp_dir().join("macaca_skill_priority_app");

        // Create same skill in both locations
        let global_skill = temp_global.join("common-skill");
        std::fs::create_dir_all(&global_skill).ok();
        std::fs::write(global_skill.join("SKILL.md"), "# Global Common").ok();

        let app_skill = temp_app.join("common-skill");
        std::fs::create_dir_all(&app_skill).ok();
        std::fs::write(app_skill.join("SKILL.md"), "# App Common").ok();

        let loader =
            SkillLoader::with_global_dir(temp_global.clone()).with_app_dir(temp_app.clone());

        let skill_path = loader.get_skill_path("common-skill").unwrap();
        // App skill should take priority
        assert!(skill_path.to_string_lossy().contains("priority_app"));

        let _ = std::fs::remove_dir_all(&temp_global);
        let _ = std::fs::remove_dir_all(&temp_app);
    }
}
