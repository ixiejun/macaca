//! AgentSkills runtime — discovery, filtering, prompt formatting, and snapshots.
//!
//! Split from monolithic `runtime.rs` (P5 iteration 85) using a Facade module tree.
//! Builds per-agent frozen skill catalogs for progressive disclosure in agentic loops.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_skill::runtime::*`.
//! - **Memento**: `SkillSnapshot` freezes catalog + prompt for one agent session.
//! - **Chain of Responsibility**: `SkillPolicyChain` filters entries during snapshot build.
//! - **Strategy**: source-scope precedence + compact/full prompt formatting strategies.
//! - **Value Object**: `SkillRuntimeOptions`, `SkillRuntimeLimits`, `SkillSnapshotEntry`.

mod config;
mod discovery;
mod filter;
mod path_policy;
mod projection;
mod prompt;
mod types;

#[cfg(test)]
mod tests;

pub use config::{SkillPolicy, SkillRuntimeLimits, SkillRuntimeOptions};
pub use path_policy::path_belongs_to_snapshot_skill;
pub use types::{FilteredSkill, SkillSnapshot, SkillSnapshotEntry};

use macaca_proto::MacacaResult;
use tracing::{debug, info};

use discovery::discover_skill_entries;
use filter::filter_entries;
use prompt::build_prompt;

/// Skill runtime service (stateless Facade over discovery → filter → prompt pipeline).
#[derive(Debug, Clone, Default)]
pub struct SkillRuntime;

impl SkillRuntime {
    /// Build a frozen skill snapshot for one agent.
    ///
    /// Pipeline:
    /// 1. Discover SKILL.md entries from configured source roots (workspace > app > bundled).
    /// 2. Apply allow/deny policy chain and metadata gates (env, OS, invocation flags).
    /// 3. Optionally project visible skills into `workspace/available_skills/` for model paths.
    /// 4. Render XML prompt block with size limits (full → compact → truncate).
    pub async fn build_snapshot(
        &self,
        agent: impl Into<String>,
        options: SkillRuntimeOptions,
    ) -> MacacaResult<SkillSnapshot> {
        let agent = agent.into();
        info!(
            agent = %agent,
            workspace = ?options.workspace_dir,
            app_dir = ?options.app_dir,
            "skill runtime: building snapshot"
        );

        let entries = discover_skill_entries(&options).await?;
        debug!(
            agent = %agent,
            discovered = entries.len(),
            "skill runtime: discovery complete"
        );

        let (visible, filtered) = filter_entries(entries, &options);
        info!(
            agent = %agent,
            visible = visible.len(),
            filtered = filtered.len(),
            "skill runtime: policy filter complete"
        );

        let (prompt, prompt_entries, truncated, compact) = build_prompt(&visible, &options).await?;
        debug!(
            agent = %agent,
            prompt_chars = prompt.len(),
            prompt_skills = prompt_entries.len(),
            truncated,
            compact,
            "skill runtime: prompt assembly complete"
        );

        let snapshot = SkillSnapshot {
            agent,
            prompt,
            skills: prompt_entries
                .into_iter()
                .map(SkillSnapshotEntry::from_skill_entry)
                .collect(),
            filtered,
            truncated,
            compact,
            version: 1,
        };

        info!(
            agent = %snapshot.agent,
            skills = snapshot.skills.len(),
            "skill runtime: snapshot ready"
        );
        Ok(snapshot)
    }
}
