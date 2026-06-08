//! Runtime configuration value objects for skill discovery and prompt limits.
//!
//! These types are application-agnostic: paths and policy sets come from the caller
//! (SDK, runtime-host provider, or shell adapter), never from hardcoded app names.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Default cap on directory children scanned per skill root.
pub(crate) const DEFAULT_MAX_CANDIDATES_PER_ROOT: usize = 300;
/// Default cap on skills loaded from one source root per snapshot build.
pub(crate) const DEFAULT_MAX_SKILLS_LOADED_PER_SOURCE: usize = 200;
/// Default cap on skills listed in the model-facing prompt.
pub(crate) const DEFAULT_MAX_SKILLS_IN_PROMPT: usize = 150;
/// Default maximum character length of the assembled skills prompt block.
pub(crate) const DEFAULT_MAX_SKILLS_PROMPT_CHARS: usize = 18_000;
/// Default maximum SKILL.md file size accepted during discovery (bytes).
pub(crate) const DEFAULT_MAX_SKILL_FILE_BYTES: u64 = 256_000;

/// Per-agent skill visibility policy (allowlist optional, denylist always applied).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillPolicy {
    /// When set, only these skill names are eligible (normalized matching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Skill names hidden even when otherwise eligible.
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Runtime discovery and prompt size limits (tunable per deployment).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRuntimeLimits {
    pub max_candidates_per_root: usize,
    pub max_skills_loaded_per_source: usize,
    pub max_skills_in_prompt: usize,
    pub max_skills_prompt_chars: usize,
    pub max_skill_file_bytes: u64,
}

impl Default for SkillRuntimeLimits {
    fn default() -> Self {
        Self {
            max_candidates_per_root: DEFAULT_MAX_CANDIDATES_PER_ROOT,
            max_skills_loaded_per_source: DEFAULT_MAX_SKILLS_LOADED_PER_SOURCE,
            max_skills_in_prompt: DEFAULT_MAX_SKILLS_IN_PROMPT,
            max_skills_prompt_chars: DEFAULT_MAX_SKILLS_PROMPT_CHARS,
            max_skill_file_bytes: DEFAULT_MAX_SKILL_FILE_BYTES,
        }
    }
}

/// Inputs used to build a per-agent skill snapshot.
#[derive(Debug, Clone, Default)]
pub struct SkillRuntimeOptions {
    /// Agent workspace root (highest-precedence skill source when present).
    pub workspace_dir: Option<PathBuf>,
    /// Application install root (persona/skills tree).
    pub app_dir: Option<PathBuf>,
    /// OS-bundled skills directory.
    pub bundled_dir: Option<PathBuf>,
    /// Additional provider-neutral search roots.
    pub extra_dirs: Vec<PathBuf>,
    pub policy: SkillPolicy,
    /// Config feature flags referenced by skill metadata gates.
    pub config_flags: HashSet<String>,
    /// Environment variable names treated as satisfied overrides.
    pub env_overrides: HashSet<String>,
    pub limits: SkillRuntimeLimits,
}
