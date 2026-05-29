//! Application workspace preparation used by the Web composition root.
//!
//! Workspace creation is a generic platform concern, not an executor concern:
//! application-owned UI bundles, WASM host imports, framework tasks, and agent
//! loops all need the same app-scoped directory identity.  This module keeps
//! that lifecycle step small and testable so Web startup can prepare a
//! workspace even when an application intentionally has no app-scoped worker
//! agents.

use std::io;
use std::path::Path;

use macaca_proto::ApplicationId;
use tracing::info;

use crate::workspace::AppWorkspace;

#[cfg(test)]
use std::path::PathBuf;

/// Prepare the app-scoped workspace directory independently of executor setup.
///
/// The Web shell acts as a composition root here: it derives the workspace from
/// `workspace.root_dir`, creates only the generic directory layout owned by the
/// application platform, and returns the value that callers should register in
/// `AppConfig.app_workspaces`.  Keeping this step separate from executor
/// registration prevents UI-only or WASM-only applications from losing their
/// workspace identity just because they do not declare runnable worker agents.
pub(crate) fn prepare_app_workspace(
    workspace_root_dir: impl AsRef<Path>,
    app_id: &ApplicationId,
    agent_names: &[String],
) -> io::Result<AppWorkspace> {
    let workspace = AppWorkspace::new(workspace_root_dir, app_id);
    workspace.ensure_dirs(agent_names)?;
    info!(
        app_id = %app_id.0,
        workspace = %workspace.root.display(),
        agent_count = agent_names.len(),
        "application workspace prepared"
    );
    Ok(workspace)
}

#[cfg(test)]
fn test_app_id() -> ApplicationId {
    ApplicationId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000042").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_workspace_bootstrap_prepares_workspace_without_agents() {
        let root = std::env::temp_dir()
            .join("macaca-app-workspace-bootstrap-test")
            .join("workspaces");
        let _ = std::fs::remove_dir_all(&root);

        let workspace = prepare_app_workspace(&root, &test_app_id(), &[]).unwrap();

        assert_eq!(workspace.root, expected_root(&root));
        assert!(workspace.root.exists());
        assert!(workspace.shared.exists());
        assert_eq!(workspace.agents_root, expected_root(&root).join("agents"));

        let _ = std::fs::remove_dir_all(&root);
    }

    fn expected_root(root: &std::path::Path) -> PathBuf {
        root.join("00000000-0000-0000-0000-000000000042")
    }
}
