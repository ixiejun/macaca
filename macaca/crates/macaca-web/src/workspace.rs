//! Workspace directory management for Macaca OS applications.
//!
//! Each application gets an isolated workspace under `{data_dir}/workspaces/{app_id}/`:
//! - `shared/`           — shared by all agents (read/write)
//! - `agents/{name}/`    — private to each agent (read/write by that agent only)

use std::path::{Path, PathBuf};

use macaca_proto::ApplicationId;

/// Workspace directory structure for an application.
#[derive(Debug, Clone)]
pub struct AppWorkspace {
    /// Root directory: `{data_dir}/workspaces/{app_id}/`
    pub root: PathBuf,
    /// Shared workspace directory accessible by all agents.
    pub shared: PathBuf,
    /// Parent directory for per-agent private workspaces.
    pub agents_root: PathBuf,
}

impl AppWorkspace {
    /// Create workspace paths for an application.
    ///
    /// Root is: `{data_dir}/workspaces/{app_id}/`
    /// Directories are NOT created here — call [`ensure_dirs`] to create them.
    pub fn new(data_dir: impl AsRef<Path>, app_id: &ApplicationId) -> Self {
        let root = data_dir
            .as_ref()
            .join(app_id.0.to_string());
        Self {
            shared: root.join("shared"),
            agents_root: root.join("agents"),
            root,
        }
    }

    /// Get the private workspace path for a specific agent.
    pub fn agent_workspace(&self, agent_name: &str) -> PathBuf {
        self.agents_root.join(agent_name)
    }

    /// Get the allowed paths for a regular worker agent:
    /// - shared workspace (read/write)
    /// - own private workspace (read/write)
    pub fn agent_allowed_paths(&self, agent_name: &str) -> Vec<String> {
        vec![
            self.shared.to_string_lossy().into_owned(),
            self.agent_workspace(agent_name)
                .to_string_lossy()
                .into_owned(),
        ]
    }

    /// Get the allowed paths for supervisor agents (coordinator/planner):
    /// - shared workspace
    /// - all agent workspaces root (covers every agent's private dir)
    pub fn supervisor_allowed_paths(&self) -> Vec<String> {
        vec![
            self.shared.to_string_lossy().into_owned(),
            self.agents_root.to_string_lossy().into_owned(),
        ]
    }

    /// Create all workspace directories. Safe to call multiple times (idempotent).
    pub fn ensure_dirs(&self, agent_names: &[String]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.shared)?;
        for name in agent_names {
            std::fs::create_dir_all(self.agent_workspace(name))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app_id() -> ApplicationId {
        ApplicationId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
    }

    #[test]
    fn workspace_paths_are_correct() {
        let ws = AppWorkspace::new("/data", &test_app_id());
        assert_eq!(
            ws.shared,
            PathBuf::from("/data/workspaces/00000000-0000-0000-0000-000000000001/shared")
        );
        assert_eq!(
            ws.agents_root,
            PathBuf::from("/data/workspaces/00000000-0000-0000-0000-000000000001/agents")
        );
        assert_eq!(
            ws.agent_workspace("backend"),
            PathBuf::from(
                "/data/workspaces/00000000-0000-0000-0000-000000000001/agents/backend"
            )
        );
    }

    #[test]
    fn agent_allowed_paths_contains_shared_and_private() {
        let ws = AppWorkspace::new("/data", &test_app_id());
        let paths = ws.agent_allowed_paths("backend");
        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("/shared"));
        assert!(paths[1].ends_with("/agents/backend"));
    }

    #[test]
    fn supervisor_allowed_paths_contains_shared_and_agents_root() {
        let ws = AppWorkspace::new("/data", &test_app_id());
        let paths = ws.supervisor_allowed_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("/shared"));
        assert!(paths[1].ends_with("/agents"));
    }

    #[test]
    fn ensure_dirs_is_idempotent() {
        let tmp = std::env::temp_dir().join("macaca-workspace-test");
        let ws = AppWorkspace::new(&tmp, &test_app_id());
        let agents = vec!["backend".to_string(), "frontend".to_string()];
        ws.ensure_dirs(&agents).unwrap();
        ws.ensure_dirs(&agents).unwrap(); // second call must not error
        assert!(ws.shared.exists());
        assert!(ws.agent_workspace("backend").exists());
        assert!(ws.agent_workspace("frontend").exists());
        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
