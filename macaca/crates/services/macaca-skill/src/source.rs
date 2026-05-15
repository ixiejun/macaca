//! Skill source factory primitives.

use std::env;
use std::path::PathBuf;

use crate::agent_skill::SkillSourceScope;
use crate::runtime::SkillRuntimeOptions;

/// One skill discovery source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    pub root: PathBuf,
    pub scope: SkillSourceScope,
    pub label: String,
}

/// Ordered skill discovery source set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillSourceSet {
    sources: Vec<SkillSource>,
}

impl SkillSourceSet {
    pub fn from_options(options: &SkillRuntimeOptions) -> Self {
        Self::from_options_with_home(options, home_dir())
    }

    fn from_options_with_home(options: &SkillRuntimeOptions, home: Option<PathBuf>) -> Self {
        let mut sources = Vec::new();
        if let Some(workspace) = &options.workspace_dir {
            sources.push(SkillSource {
                root: workspace.join("skills"),
                scope: SkillSourceScope::Workspace,
                label: "workspace".into(),
            });
        }
        if let Some(app_dir) = &options.app_dir {
            sources.push(SkillSource {
                root: app_dir.join(".agents").join("skills"),
                scope: SkillSourceScope::ProjectAgents,
                label: "project_agents".into(),
            });
            sources.push(SkillSource {
                root: app_dir.join("skills"),
                scope: SkillSourceScope::Application,
                label: "application".into(),
            });
        }
        if let Some(home) = home {
            // Macaca-owned skills are the platform source of truth for Agent OS
            // runtimes.  Generic and client-specific stores remain supported as
            // fallback sources so Macaca can consume skills authored for other
            // agent clients without letting those clients shadow OS-managed
            // skills with the same declared name.
            sources.push(SkillSource {
                root: home.join(".macaca").join("skills"),
                scope: SkillSourceScope::MacacaCentral,
                label: "macaca_central".into(),
            });
            sources.push(SkillSource {
                root: home.join(".agent").join("skills"),
                scope: SkillSourceScope::UserAgentGeneric,
                label: "user_agent_generic".into(),
            });
            for client in ["claude", "codex", "hermes", "openclaw"] {
                sources.push(SkillSource {
                    root: home.join(format!(".{client}")).join("skills"),
                    scope: SkillSourceScope::UserClient,
                    label: format!("user_client_{client}"),
                });
            }
        }
        if let Some(bundled) = &options.bundled_dir {
            sources.push(SkillSource {
                root: bundled.clone(),
                scope: SkillSourceScope::Bundled,
                label: "bundled".into(),
            });
        }
        for dir in &options.extra_dirs {
            sources.push(SkillSource {
                root: dir.clone(),
                scope: SkillSourceScope::Extra,
                label: "extra".into(),
            });
        }
        Self { sources }
    }

    pub fn iter(&self) -> impl Iterator<Item = &SkillSource> {
        self.sources.iter()
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_global_sources_follow_macaca_first_fallback_order() {
        let home = PathBuf::from("/tmp/macaca-home");
        let set =
            SkillSourceSet::from_options_with_home(&SkillRuntimeOptions::default(), Some(home));
        let roots: Vec<_> = set
            .iter()
            .map(|source| source.root.display().to_string())
            .collect();

        assert_eq!(roots[0], "/tmp/macaca-home/.macaca/skills");
        assert_eq!(roots[1], "/tmp/macaca-home/.agent/skills");
        assert_eq!(roots[2], "/tmp/macaca-home/.claude/skills");
        assert_eq!(roots[3], "/tmp/macaca-home/.codex/skills");
        assert_eq!(roots[4], "/tmp/macaca-home/.hermes/skills");
        assert_eq!(roots[5], "/tmp/macaca-home/.openclaw/skills");
    }
}
