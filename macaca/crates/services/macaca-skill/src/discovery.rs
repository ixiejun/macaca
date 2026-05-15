//! Skill discovery — scan directories for `SKILL.md` files.
//!
//! Implements Agent OS skill discovery by scanning multiple directory scopes
//! for subdirectories containing `SKILL.md` files. Project-level skills remain
//! available for local development, while user-global discovery prefers
//! Macaca-managed skills before falling back to generic and client-owned
//! stores.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use macaca_proto::{MacacaError, MacacaResult};

use crate::agent_skill::AgentSkill;

/// Directories to skip during discovery.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    "dist",
    "build",
];

/// Maximum depth for recursive scanning (reserved for future use).
#[allow(dead_code)]
const MAX_DEPTH: usize = 4;

/// Maximum directories to scan before stopping.
const MAX_DIRS: usize = 2000;

/// Priority scope for skill discovery. Lower ordinal = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillScope {
    /// Project-level client-specific (highest priority).
    ProjectClient = 0,
    /// Project-level cross-client (.agents/skills/).
    ProjectCrossClient = 1,
    /// Agent OS central store (~/.macaca/skills/).
    AgentOsCentral = 2,
    /// Generic user-level Agent skills (~/.agent/skills/).
    UserAgentGeneric = 3,
    /// User-level client-specific (e.g., ~/.claude/skills/).
    UserClient = 4,
}

/// A discovered skill with its scope for precedence resolution.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub skill: AgentSkill,
    pub scope: SkillScope,
}

/// Discover all Agent Skills across standard directories.
///
/// Scans directories in priority order (project > user > central).
/// On name collision, the higher-priority (lower ordinal) scope wins.
///
/// # Arguments
/// * `project_dir` — The project working directory (for project-level skills).
///   Pass `None` to skip project-level scanning.
/// * `client_name` — The client name (e.g., "claude") for client-specific dirs.
///   Pass `None` to skip client-specific scanning.
/// * `extra_dirs` — Additional directories to scan (treated as AgentOsCentral scope).
pub async fn discover_skills(
    project_dir: Option<&Path>,
    client_name: Option<&str>,
    extra_dirs: &[PathBuf],
) -> MacacaResult<Vec<DiscoveredSkill>> {
    let mut scan_targets: Vec<(PathBuf, SkillScope)> = Vec::new();

    // Project-level scopes (highest priority).
    if let Some(proj) = project_dir {
        if let Some(client) = client_name {
            let client_dir = proj.join(format!(".{client}/skills"));
            scan_targets.push((client_dir, SkillScope::ProjectClient));
        }
        scan_targets.push((proj.join(".agents/skills"), SkillScope::ProjectCrossClient));
    }

    // User-level scopes. Macaca central skills win global collisions, then the
    // generic Agent directory, then common client-specific stores. This mirrors
    // the runtime snapshot source factory so every skill-facing API reports the
    // same precedence model.
    if let Some(home) = home_dir() {
        scan_targets.push((home.join(".macaca/skills"), SkillScope::AgentOsCentral));
        scan_targets.push((home.join(".agent/skills"), SkillScope::UserAgentGeneric));
        for client in common_skill_clients(client_name) {
            let client_dir = home.join(format!(".{client}/skills"));
            scan_targets.push((client_dir, SkillScope::UserClient));
        }
    }

    // Extra directories.
    for dir in extra_dirs {
        scan_targets.push((dir.clone(), SkillScope::AgentOsCentral));
    }

    // Scan all directories and collect skills.
    let mut skills_by_name: HashMap<String, DiscoveredSkill> = HashMap::new();

    for (dir, scope) in &scan_targets {
        if !dir.exists() {
            continue;
        }
        let found = scan_directory(dir, *scope).await;
        for ds in found {
            // Project-level overrides user-level on name collision.
            let should_insert = match skills_by_name.get(&ds.skill.name) {
                Some(existing) => ds.scope < existing.scope,
                None => true,
            };
            if should_insert {
                if skills_by_name.contains_key(&ds.skill.name) {
                    tracing::warn!(
                        skill = %ds.skill.name,
                        scope = ?ds.scope,
                        "skill name collision — overriding with higher-priority scope"
                    );
                }
                skills_by_name.insert(ds.skill.name.clone(), ds);
            }
        }
    }

    let mut result: Vec<DiscoveredSkill> = skills_by_name.into_values().collect();
    result.sort_by(|a, b| a.skill.name.cmp(&b.skill.name));
    Ok(result)
}

/// Scan a single directory for subdirectories containing `SKILL.md`.
async fn scan_directory(dir: &Path, scope: SkillScope) -> Vec<DiscoveredSkill> {
    let mut results = Vec::new();
    let mut dirs_scanned = 0usize;

    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return results;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        if dirs_scanned >= MAX_DIRS {
            tracing::warn!("hit max directory scan limit ({MAX_DIRS}) in {:?}", dir);
            break;
        }

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip excluded directories.
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if SKIP_DIRS.contains(&name) {
                continue;
            }
        }

        dirs_scanned += 1;

        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            match AgentSkill::from_path(&skill_md).await {
                Ok(skill) => {
                    tracing::debug!(
                        skill = %skill.name,
                        scope = ?scope,
                        "discovered skill at {:?}",
                        skill_md
                    );
                    results.push(DiscoveredSkill { skill, scope });
                }
                Err(e) => {
                    tracing::warn!("failed to parse {:?}: {}", skill_md, e);
                }
            }
        }
    }

    results
}

/// Scan a directory for SKILL.md files (non-recursive, single level).
///
/// This is a simpler variant for scanning a known skills directory
/// like `~/.macaca/skills/`.
pub async fn scan_skills_directory(dir: &Path) -> MacacaResult<Vec<AgentSkill>> {
    if !dir.exists() {
        return Err(MacacaError::NotFound(format!(
            "Skills directory not found: {}",
            dir.display()
        )));
    }

    let mut skills = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await.map_err(MacacaError::Io)?;

    while let Some(entry) = entries.next_entry().await.map_err(MacacaError::Io)? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            match AgentSkill::from_path(&skill_md).await {
                Ok(skill) => {
                    skills.push(skill);
                }
                Err(e) => {
                    tracing::warn!("failed to parse {:?}: {}", skill_md, e);
                }
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn common_skill_clients(preferred: Option<&str>) -> Vec<String> {
    let mut clients = Vec::new();
    if let Some(client) = preferred {
        clients.push(client.to_string());
    }
    for client in ["claude", "codex", "hermes", "openclaw"] {
        if !clients.iter().any(|existing| existing == client) {
            clients.push(client.to_string());
        }
    }
    clients
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scan_skills_directory_basic() {
        let dir = tempfile::tempdir().unwrap();

        // Create two skill subdirectories.
        for name in &["alpha", "beta"] {
            let skill_dir = dir.path().join(name);
            tokio::fs::create_dir(&skill_dir).await.unwrap();
            tokio::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: {name} skill\n---\n# {name}\nContent."),
            )
            .await
            .unwrap();
        }

        // Create a non-skill directory (no SKILL.md).
        let no_skill = dir.path().join("not-a-skill");
        tokio::fs::create_dir(&no_skill).await.unwrap();
        tokio::fs::write(no_skill.join("README.md"), "Not a skill")
            .await
            .unwrap();

        let skills = scan_skills_directory(dir.path()).await.unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "alpha");
        assert_eq!(skills[1].name, "beta");
    }

    #[tokio::test]
    async fn scan_skills_directory_missing() {
        let err = scan_skills_directory(Path::new("/nonexistent/path"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn discover_with_extra_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Test\n---\nContent.",
        )
        .await
        .unwrap();

        // Use a fake home to avoid picking up real user skills.
        let fake_home = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", fake_home.path());

        let skills = discover_skills(None, None, &[dir.path().to_path_buf()])
            .await
            .unwrap();

        // Restore HOME (best effort — tests may run in parallel).
        if let Ok(real_home) = std::env::var("REAL_HOME") {
            std::env::set_var("HOME", real_home);
        }

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].skill.name, "my-skill");
        assert_eq!(skills[0].scope, SkillScope::AgentOsCentral);
    }

    #[tokio::test]
    async fn discover_name_collision_project_wins() {
        let project_dir = tempfile::tempdir().unwrap();
        let user_dir = tempfile::tempdir().unwrap();

        // Project-level skill.
        let proj_agents = project_dir.path().join(".agents/skills/my-skill");
        tokio::fs::create_dir_all(&proj_agents).await.unwrap();
        tokio::fs::write(
            proj_agents.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Project version\n---\nProject content.",
        )
        .await
        .unwrap();

        // User-level skill (same name, lower priority).
        let user_skill = user_dir.path().join("my-skill");
        tokio::fs::create_dir(&user_skill).await.unwrap();
        tokio::fs::write(
            user_skill.join("SKILL.md"),
            "---\nname: my-skill\ndescription: User version\n---\nUser content.",
        )
        .await
        .unwrap();

        // Use a fake home to avoid picking up real user skills.
        let fake_home = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", fake_home.path());

        let skills = discover_skills(
            Some(project_dir.path()),
            None,
            &[user_dir.path().to_path_buf()],
        )
        .await
        .unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].skill.description, "Project version");
        assert_eq!(skills[0].scope, SkillScope::ProjectCrossClient);
    }

    #[tokio::test]
    async fn scan_skips_excluded_dirs() {
        let dir = tempfile::tempdir().unwrap();

        // Create a .git directory with a SKILL.md (should be skipped).
        let git_dir = dir.path().join(".git");
        tokio::fs::create_dir(&git_dir).await.unwrap();
        tokio::fs::write(
            git_dir.join("SKILL.md"),
            "---\nname: git\ndescription: fake\n---\nfake",
        )
        .await
        .unwrap();

        // Create a real skill.
        let real_dir = dir.path().join("real-skill");
        tokio::fs::create_dir(&real_dir).await.unwrap();
        tokio::fs::write(
            real_dir.join("SKILL.md"),
            "---\nname: real-skill\ndescription: Real\n---\nReal content.",
        )
        .await
        .unwrap();

        let found = scan_directory(dir.path(), SkillScope::AgentOsCentral).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].skill.name, "real-skill");
    }
}
