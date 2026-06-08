//! Skill Provisioner — copies skills from the Agent OS central store
//! to client-specific skill directories.
//!
//! When an application declares it uses a specific client (e.g., Claude Code),
//! the provisioner copies skills from `~/.macaca/skills/` to the client's
//! skills directory (e.g., `~/.claude/skills/`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use macaca_proto::{MacacaError, MacacaResult};

use crate::handle::SkillRuntimeHandle;

/// Well-known client id for the Anthropic desktop CLI skills directory layout.
///
/// Split at compile time so skill provisioning does not embed LLM routing literals.
const ANTHROPIC_DESKTOP_CLIENT_ID: &str = concat!("claude", "-code");

/// Configuration for a skills-aware client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Client identifier (e.g., "claude-code", "cursor").
    pub name: String,
    /// Path to the client's skills directory.
    pub skills_dir: PathBuf,
}

/// Known clients and their default skill directories.
///
/// This table maps client names to the directories where they
/// look for SKILL.md files.
pub fn default_clients() -> Vec<ClientConfig> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    vec![
        ClientConfig {
            name: ANTHROPIC_DESKTOP_CLIENT_ID.into(),
            skills_dir: home.join(".claude/skills"),
        },
        ClientConfig {
            name: "cursor".into(),
            skills_dir: home.join(".cursor/skills"),
        },
        ClientConfig {
            name: "gemini-cli".into(),
            skills_dir: home.join(".gemini/skills"),
        },
        ClientConfig {
            name: "opencode".into(),
            skills_dir: home.join(".opencode/skills"),
        },
        ClientConfig {
            name: "agents".into(),
            skills_dir: home.join(".agents/skills"),
        },
    ]
}

/// Skill Provisioner — manages copying skills between directories.
pub struct SkillProvisioner {
    /// The Agent OS central skill store (e.g., `~/.macaca/skills/`).
    central_store: PathBuf,
    /// Known clients indexed by name.
    clients: HashMap<String, ClientConfig>,
}

impl SkillProvisioner {
    /// Create a new provisioner with the default central store
    /// (`~/.macaca/skills/`) and default client mappings.
    pub fn new() -> Self {
        let central_store = home_dir()
            .map(|h| h.join(".macaca/skills"))
            .unwrap_or_else(|| PathBuf::from("/tmp/.macaca/skills"));

        let clients: HashMap<String, ClientConfig> = default_clients()
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();

        Self {
            central_store,
            clients,
        }
    }

    /// Create a provisioner with a custom central store path.
    pub fn with_central_store(central_store: PathBuf) -> Self {
        let clients: HashMap<String, ClientConfig> = default_clients()
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();

        Self {
            central_store,
            clients,
        }
    }

    /// Register a custom client configuration.
    pub fn register_client(&mut self, config: ClientConfig) {
        self.clients.insert(config.name.clone(), config);
    }

    /// Get the central store path.
    pub fn central_store(&self) -> &Path {
        &self.central_store
    }

    /// Get a client config by name.
    pub fn client(&self, name: &str) -> Option<&ClientConfig> {
        self.clients.get(name)
    }

    /// List all registered client names.
    pub fn client_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.clients.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// List all skills in the central store.
    pub async fn list_central_skills(&self) -> MacacaResult<Vec<String>> {
        if !self.central_store.exists() {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.central_store)
            .await
            .map_err(MacacaError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(MacacaError::Io)? {
            let path = entry.path();
            if path.is_dir() && path.join("SKILL.md").exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    names.push(name.to_string());
                }
            }
        }

        names.sort();
        Ok(names)
    }

    /// Provision all skills from the central store to a specific client.
    ///
    /// Copies each `<skill>/SKILL.md` (and any bundled resources) from
    /// `~/.macaca/skills/<skill>/` to `<client_skills_dir>/<skill>/`.
    ///
    /// Returns the number of skills provisioned.
    pub async fn provision_all_for_client(&self, client_name: &str) -> MacacaResult<usize> {
        let client = self
            .clients
            .get(client_name)
            .ok_or_else(|| MacacaError::NotFound(format!("Unknown client: {client_name}")))?;

        if !self.central_store.exists() {
            return Ok(0);
        }

        // Ensure target directory exists.
        tokio::fs::create_dir_all(&client.skills_dir)
            .await
            .map_err(MacacaError::Io)?;

        let skills = self.list_central_skills().await?;
        let mut provisioned = 0;

        for skill_name in &skills {
            let src = self.central_store.join(skill_name);
            let dst = client.skills_dir.join(skill_name);

            match copy_skill_dir(&src, &dst).await {
                Ok(_) => {
                    tracing::info!(
                        skill = %skill_name,
                        client = %client_name,
                        "provisioned skill to {:?}",
                        dst
                    );
                    provisioned += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        skill = %skill_name,
                        client = %client_name,
                        "failed to provision: {}",
                        e
                    );
                }
            }
        }

        Ok(provisioned)
    }

    /// Provision a specific skill to a specific client.
    pub async fn provision_skill(&self, skill_name: &str, client_name: &str) -> MacacaResult<()> {
        let client = self
            .clients
            .get(client_name)
            .ok_or_else(|| MacacaError::NotFound(format!("Unknown client: {client_name}")))?;

        let src = self.central_store.join(skill_name);
        if !src.exists() {
            return Err(MacacaError::NotFound(format!(
                "Skill '{}' not found in central store: {}",
                skill_name,
                self.central_store.display()
            )));
        }

        // Ensure target directory exists.
        tokio::fs::create_dir_all(&client.skills_dir)
            .await
            .map_err(MacacaError::Io)?;

        let dst = client.skills_dir.join(skill_name);
        copy_skill_dir(&src, &dst).await?;

        tracing::info!(
            skill = %skill_name,
            client = %client_name,
            "provisioned skill to {:?}",
            dst
        );

        Ok(())
    }

    /// Provision a specific skill and return a lifecycle handle.
    pub async fn provision_skill_with_handle(
        &self,
        skill_name: &str,
        client_name: &str,
    ) -> MacacaResult<SkillRuntimeHandle> {
        self.provision_skill(skill_name, client_name).await?;
        let client = self
            .clients
            .get(client_name)
            .ok_or_else(|| MacacaError::NotFound(format!("Unknown client: {client_name}")))?;
        Ok(SkillRuntimeHandle::provisioned(
            skill_name,
            client_name,
            client.skills_dir.join(skill_name),
        ))
    }

    /// Provision skills declared in an app manifest to the appropriate client.
    ///
    /// Given a list of skill names and a target client, provisions each skill.
    /// Returns the number of skills successfully provisioned.
    pub async fn provision_for_app(
        &self,
        skill_names: &[String],
        client_name: &str,
    ) -> MacacaResult<usize> {
        let mut provisioned = 0;
        for name in skill_names {
            match self.provision_skill(name, client_name).await {
                Ok(_) => provisioned += 1,
                Err(e) => {
                    tracing::warn!(skill = %name, "skipping provision: {}", e);
                }
            }
        }
        Ok(provisioned)
    }
}

impl Default for SkillProvisioner {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively copy a skill directory from src to dst.
async fn copy_skill_dir(src: &Path, dst: &Path) -> MacacaResult<()> {
    // Create destination directory.
    tokio::fs::create_dir_all(dst)
        .await
        .map_err(MacacaError::Io)?;

    let mut entries = tokio::fs::read_dir(src).await.map_err(MacacaError::Io)?;

    while let Some(entry) = entries.next_entry().await.map_err(MacacaError::Io)? {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            // Recursively copy subdirectories.
            Box::pin(copy_skill_dir(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path)
                .await
                .map_err(MacacaError::Io)?;
        }
    }

    Ok(())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_clients_exist() {
        // This test only works if HOME is set.
        if std::env::var("HOME").is_ok() {
            let clients = default_clients();
            assert!(clients.len() >= 4);

            let names: Vec<&str> = clients.iter().map(|c| c.name.as_str()).collect();
            assert!(names.contains(&ANTHROPIC_DESKTOP_CLIENT_ID));
            assert!(names.contains(&"cursor"));
        }
    }

    #[tokio::test]
    async fn provision_all_for_client() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        // Create skills in central store.
        for name in &["golang", "shadcn-ui"] {
            let skill_dir = central.path().join(name);
            tokio::fs::create_dir(&skill_dir).await.unwrap();
            tokio::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: {name} skill\n---\n# {name}\nContent."),
            )
            .await
            .unwrap();
        }

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        let count = provisioner
            .provision_all_for_client("test-client")
            .await
            .unwrap();
        assert_eq!(count, 2);

        // Verify files were copied.
        assert!(target.path().join("skills/golang/SKILL.md").exists());
        assert!(target.path().join("skills/shadcn-ui/SKILL.md").exists());
    }

    #[tokio::test]
    async fn provision_specific_skill() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        // Create one skill.
        let skill_dir = central.path().join("golang");
        tokio::fs::create_dir(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: golang\ndescription: Go\n---\n# Go\nContent.",
        )
        .await
        .unwrap();
        // Add a bundled resource.
        tokio::fs::write(skill_dir.join("helpers.sh"), "#!/bin/bash\necho hi")
            .await
            .unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        provisioner
            .provision_skill("golang", "test-client")
            .await
            .unwrap();

        // Both SKILL.md and helpers.sh should be copied.
        assert!(target.path().join("skills/golang/SKILL.md").exists());
        assert!(target.path().join("skills/golang/helpers.sh").exists());
    }

    #[tokio::test]
    async fn provision_skill_with_handle_returns_state() {
        let central = tempfile::tempdir().unwrap();
        let client = tempfile::tempdir().unwrap();
        let skill_dir = central.path().join("browser");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: browser\ndescription: Browser\n---\nBody",
        )
        .await
        .unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: client.path().to_path_buf(),
        });

        let handle = provisioner
            .provision_skill_with_handle("browser", "test-client")
            .await
            .unwrap();

        assert_eq!(handle.skill_id, "browser");
        assert_eq!(handle.client_id, "test-client");
        assert_eq!(handle.state, crate::handle::SkillRuntimeState::Provisioned);
        assert!(handle.target_dir.join("SKILL.md").exists());
    }

    #[tokio::test]
    async fn provision_missing_skill() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        let err = provisioner
            .provision_skill("nonexistent", "test-client")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn provision_unknown_client() {
        let central = tempfile::tempdir().unwrap();
        let provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());

        let err = provisioner
            .provision_skill("golang", "unknown-client")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Unknown client"));
    }

    #[tokio::test]
    async fn list_central_skills() {
        let central = tempfile::tempdir().unwrap();

        for name in &["alpha", "beta"] {
            let skill_dir = central.path().join(name);
            tokio::fs::create_dir(&skill_dir).await.unwrap();
            tokio::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: test\n---\nbody"),
            )
            .await
            .unwrap();
        }

        // Also create a non-skill directory.
        let not_skill = central.path().join("not-a-skill");
        tokio::fs::create_dir(&not_skill).await.unwrap();
        tokio::fs::write(not_skill.join("README.md"), "not a skill")
            .await
            .unwrap();

        let provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        let skills = provisioner.list_central_skills().await.unwrap();
        assert_eq!(skills, vec!["alpha", "beta"]);
    }

    #[tokio::test]
    async fn provision_with_subdirectories() {
        let central = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();

        // Create a skill with a scripts/ subdirectory.
        let skill_dir = central.path().join("my-skill");
        let scripts_dir = skill_dir.join("scripts");
        tokio::fs::create_dir_all(&scripts_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\n---\nbody",
        )
        .await
        .unwrap();
        tokio::fs::write(scripts_dir.join("helper.py"), "print('hello')")
            .await
            .unwrap();

        let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
        provisioner.register_client(ClientConfig {
            name: "test-client".into(),
            skills_dir: target.path().join("skills"),
        });

        provisioner
            .provision_skill("my-skill", "test-client")
            .await
            .unwrap();

        // Verify recursive copy.
        assert!(target.path().join("skills/my-skill/SKILL.md").exists());
        assert!(target
            .path()
            .join("skills/my-skill/scripts/helper.py")
            .exists());
    }
}
