//! Filesystem discovery for AgentSkills (`SKILL.md`) entries.
//!
//! Scans configured source roots with bounded iteration and merges duplicates
//! using source-scope precedence (workspace wins over app over bundled).

use std::collections::HashMap;
use std::path::Path;

use macaca_proto::MacacaResult;
use tracing::{debug, warn};

use crate::agent_skill::{
    AgentSkill, SkillEntry, SkillExposure, SkillSourceScope,
};
use crate::source::SkillSourceSet;

use super::config::SkillRuntimeLimits;
use super::config::SkillRuntimeOptions;

/// Directory names skipped during breadth-first skill root scans.
pub(crate) const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    "dist",
    "build",
];

/// Discover skill entries from all configured sources, deduplicated by name.
pub(crate) async fn discover_skill_entries(
    options: &SkillRuntimeOptions,
) -> MacacaResult<Vec<SkillEntry>> {
    let sources = SkillSourceSet::from_options(options);
    let mut by_name: HashMap<String, SkillEntry> = HashMap::new();

    for source in sources.iter() {
        debug!(
            root = %source.root.display(),
            scope = ?source.scope,
            label = %source.label,
            "skill runtime: scanning source root"
        );
        for entry in
            scan_source_dir(&source.root, source.scope, &source.label, &options.limits).await?
        {
            let should_insert = by_name
                .get(&entry.skill.name)
                .map(|existing| entry.skill.source_scope < existing.skill.source_scope)
                .unwrap_or(true);
            if should_insert {
                by_name.insert(entry.skill.name.clone(), entry);
            }
        }
    }

    let mut entries: Vec<_> = by_name.into_values().collect();
    entries.sort_by(|a, b| a.skill.name.cmp(&b.skill.name));
    Ok(entries)
}

/// Scan one source directory for `SKILL.md` files (single-root skill or child folders).
async fn scan_source_dir(
    dir: &Path,
    scope: SkillSourceScope,
    source: &str,
    limits: &SkillRuntimeLimits,
) -> MacacaResult<Vec<SkillEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let root = match tokio::fs::canonicalize(dir).await {
        Ok(path) => path,
        Err(_) => return Ok(Vec::new()),
    };
    let mut entries = Vec::new();

    // Support a source root that is itself one skill (./SKILL.md at root).
    let root_skill = dir.join("SKILL.md");
    if root_skill.exists() {
        if let Some(entry) = load_skill_entry(
            &root,
            &root_skill,
            scope,
            source,
            limits.max_skill_file_bytes,
        )
        .await
        {
            entries.push(entry);
        }
        return Ok(entries);
    }

    let mut children = match tokio::fs::read_dir(dir).await {
        Ok(children) => children,
        Err(_) => return Ok(Vec::new()),
    };
    let mut scanned = 0usize;
    while let Some(child) = children.next_entry().await? {
        if scanned >= limits.max_candidates_per_root
            || entries.len() >= limits.max_skills_loaded_per_source
        {
            warn!(
                root = %dir.display(),
                scanned,
                loaded = entries.len(),
                "skill runtime: discovery limits reached for source root"
            );
            break;
        }
        let path = child.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            continue;
        }
        scanned += 1;
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        if let Some(entry) =
            load_skill_entry(&root, &skill_md, scope, source, limits.max_skill_file_bytes).await
        {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Load and parse one `SKILL.md` file when it passes path and size guards.
async fn load_skill_entry(
    root: &Path,
    skill_md: &Path,
    scope: SkillSourceScope,
    source: &str,
    max_file_bytes: u64,
) -> Option<SkillEntry> {
    let canonical = tokio::fs::canonicalize(skill_md).await.ok()?;
    if !canonical.starts_with(root) {
        warn!(path = %skill_md.display(), "skipping skill outside source root");
        return None;
    }
    if tokio::fs::metadata(&canonical).await.ok()?.len() > max_file_bytes {
        warn!(path = %skill_md.display(), "skipping oversized SKILL.md");
        return None;
    }
    let raw = tokio::fs::read_to_string(&canonical).await.ok()?;
    let parsed = crate::agent_skill::parse_skill_md_full(&raw).ok()?;
    let skill = AgentSkill::from_path_with_source(&canonical, scope, source.to_string())
        .await
        .ok()?;
    Some(SkillEntry {
        skill,
        metadata: parsed.metadata,
        exposure: SkillExposure {
            include_in_runtime_registry: true,
            include_in_available_skills_prompt: !parsed.invocation.disable_model_invocation,
            user_invocable: parsed.invocation.user_invocable,
        },
        invocation: parsed.invocation,
    })
}
