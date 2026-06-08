//! Workspace root paths and declarative workspace guide sources.
//!
//! Guide entries describe well-known Markdown filenames (for example `AGENTS.md`)
//! resolved relative to a workspace root — never application-specific agent names.

use serde::{Deserialize, Serialize};

/// Filesystem root for per-application workspace sandboxes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root_dir: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root_dir: "./data/workspaces".into(),
        }
    }
}

/// Ordered list of workspace guide Markdown files injected into context assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceGuideSourcesConfig {
    #[serde(default = "default_workspace_guide_entries")]
    pub entries: Vec<WorkspaceGuideEntry>,
}

impl Default for WorkspaceGuideSourcesConfig {
    fn default() -> Self {
        Self {
            entries: default_workspace_guide_entries(),
        }
    }
}

/// One workspace guide candidate: relative path, priority, and byte budget.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceGuideEntry {
    pub relative_path: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_workspace_guide_max_bytes")]
    pub max_bytes: u32,
}

fn default_workspace_guide_max_bytes() -> u32 {
    16 * 1024
}

fn default_workspace_guide_entries() -> Vec<WorkspaceGuideEntry> {
    vec![
        WorkspaceGuideEntry {
            relative_path: "AGENTS.md".into(),
            priority: 0,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "SOUL.md".into(),
            priority: 10,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "TOOLS.md".into(),
            priority: 20,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "IDENTITY.md".into(),
            priority: 30,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "USER.md".into(),
            priority: 40,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "HEARTBEAT.md".into(),
            priority: 50,
            max_bytes: default_workspace_guide_max_bytes(),
        },
    ]
}
