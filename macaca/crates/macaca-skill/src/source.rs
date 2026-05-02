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
        if let Some(home) = home_dir() {
            sources.push(SkillSource {
                root: home.join(".agents").join("skills"),
                scope: SkillSourceScope::UserAgents,
                label: "user_agents".into(),
            });
            sources.push(SkillSource {
                root: home.join(".macaca").join("skills"),
                scope: SkillSourceScope::MacacaCentral,
                label: "macaca_central".into(),
            });
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
