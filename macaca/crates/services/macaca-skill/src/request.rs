//! Consumer-facing skill snapshot request builder.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::runtime::{SkillPolicy, SkillRuntimeLimits, SkillRuntimeOptions};

/// Request used by upper crates to build a per-agent skill snapshot.
#[derive(Debug, Clone)]
pub struct SkillSnapshotRequest {
    pub agent: String,
    pub options: SkillRuntimeOptions,
}

/// Builder for [`SkillSnapshotRequest`].
#[derive(Debug, Clone)]
pub struct SkillSnapshotRequestBuilder {
    agent: String,
    workspace_dir: Option<PathBuf>,
    app_dir: Option<PathBuf>,
    bundled_dir: Option<PathBuf>,
    extra_dirs: Vec<PathBuf>,
    policy: SkillPolicy,
    config_flags: HashSet<String>,
    env_overrides: HashSet<String>,
    limits: SkillRuntimeLimits,
}

impl SkillSnapshotRequest {
    pub fn builder(agent: impl Into<String>) -> SkillSnapshotRequestBuilder {
        SkillSnapshotRequestBuilder {
            agent: agent.into(),
            workspace_dir: None,
            app_dir: None,
            bundled_dir: None,
            extra_dirs: Vec::new(),
            policy: SkillPolicy::default(),
            config_flags: HashSet::new(),
            env_overrides: HashSet::new(),
            limits: SkillRuntimeLimits::default(),
        }
    }
}

impl SkillSnapshotRequestBuilder {
    pub fn workspace_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.workspace_dir = dir;
        self
    }

    pub fn app_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.app_dir = dir;
        self
    }

    pub fn bundled_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.bundled_dir = dir;
        self
    }

    pub fn extra_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.extra_dirs = dirs;
        self
    }

    pub fn policy(mut self, policy: SkillPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn config_flags(mut self, flags: HashSet<String>) -> Self {
        self.config_flags = flags;
        self
    }

    pub fn env_overrides(mut self, env: HashSet<String>) -> Self {
        self.env_overrides = env;
        self
    }

    pub fn limits(mut self, limits: SkillRuntimeLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn build(self) -> SkillSnapshotRequest {
        SkillSnapshotRequest {
            agent: self.agent,
            options: SkillRuntimeOptions {
                workspace_dir: self.workspace_dir,
                app_dir: self.app_dir,
                bundled_dir: self.bundled_dir,
                extra_dirs: self.extra_dirs,
                policy: self.policy,
                config_flags: self.config_flags,
                env_overrides: self.env_overrides,
                limits: self.limits,
            },
        }
    }
}
