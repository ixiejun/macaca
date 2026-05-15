//! AgentSkills runtime — discovery, filtering, prompt formatting, and snapshots.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use macaca_proto::MacacaResult;
use serde::{Deserialize, Serialize};

use crate::agent_skill::{AgentSkill, SkillEntry, SkillExposure, SkillSourceScope};
use crate::agent_skill::{SkillInstallSpec, SkillMcpServerConfig};
use crate::policy::{normalize_policy_set, PolicyDecision, SkillExposureContext, SkillPolicyChain};
use crate::source::SkillSourceSet;

const DEFAULT_MAX_CANDIDATES_PER_ROOT: usize = 300;
const DEFAULT_MAX_SKILLS_LOADED_PER_SOURCE: usize = 200;
const DEFAULT_MAX_SKILLS_IN_PROMPT: usize = 150;
const DEFAULT_MAX_SKILLS_PROMPT_CHARS: usize = 18_000;
const DEFAULT_MAX_SKILL_FILE_BYTES: u64 = 256_000;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    "dist",
    "build",
];

/// Per-agent skill visibility policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillPolicy {
    /// If present, only these skill names are visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Skill names to hide even if otherwise eligible.
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Runtime discovery and prompt size limits.
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
    pub workspace_dir: Option<PathBuf>,
    pub app_dir: Option<PathBuf>,
    pub bundled_dir: Option<PathBuf>,
    pub extra_dirs: Vec<PathBuf>,
    pub policy: SkillPolicy,
    pub config_flags: HashSet<String>,
    pub env_overrides: HashSet<String>,
    pub limits: SkillRuntimeLimits,
}

/// Skill entry visible in a frozen snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSnapshotEntry {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub source_location: PathBuf,
    #[serde(default)]
    pub source_base_dir: PathBuf,
    pub location: PathBuf,
    pub base_dir: PathBuf,
    pub source: String,
    pub source_scope: SkillSourceScope,
    pub primary_env: Option<String>,
    pub required_env: Vec<String>,
    pub install: Vec<SkillInstallSpec>,
    pub mcp_servers: Vec<SkillMcpServerConfig>,
}

/// Filtered skill diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilteredSkill {
    pub name: String,
    pub reason: String,
    pub source: String,
}

/// Frozen skill catalog for one agent run/session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSnapshot {
    pub agent: String,
    pub prompt: String,
    pub skills: Vec<SkillSnapshotEntry>,
    pub filtered: Vec<FilteredSkill>,
    pub truncated: bool,
    pub compact: bool,
    pub version: u64,
}

/// Skill runtime service.
#[derive(Debug, Clone, Default)]
pub struct SkillRuntime;

impl SkillRuntime {
    /// Build a frozen skill snapshot for one agent.
    pub async fn build_snapshot(
        &self,
        agent: impl Into<String>,
        options: SkillRuntimeOptions,
    ) -> MacacaResult<SkillSnapshot> {
        let agent = agent.into();
        let entries = discover_skill_entries(&options).await?;
        let (visible, filtered) = filter_entries(entries, &options);
        let (prompt, prompt_entries, truncated, compact) = build_prompt(&visible, &options).await?;

        Ok(SkillSnapshot {
            agent,
            prompt,
            skills: prompt_entries
                .into_iter()
                .map(|entry| SkillSnapshotEntry {
                    name: entry.skill.name.clone(),
                    description: entry.skill.description.clone(),
                    source_location: entry.skill.canonical_location.clone(),
                    source_base_dir: entry.skill.canonical_base_dir.clone(),
                    location: entry.skill.location.clone(),
                    base_dir: entry.skill.base_dir.clone(),
                    source: entry.skill.source.clone(),
                    source_scope: entry.skill.source_scope,
                    primary_env: entry.metadata.primary_env.clone(),
                    required_env: entry.metadata.requires_env.clone(),
                    install: entry.metadata.install.clone(),
                    mcp_servers: entry.metadata.mcp_servers.clone(),
                })
                .collect(),
            filtered,
            truncated,
            compact,
            version: 1,
        })
    }
}

async fn discover_skill_entries(options: &SkillRuntimeOptions) -> MacacaResult<Vec<SkillEntry>> {
    let sources = SkillSourceSet::from_options(options);
    let mut by_name: HashMap<String, SkillEntry> = HashMap::new();
    for source in sources.iter() {
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

async fn load_skill_entry(
    root: &Path,
    skill_md: &Path,
    scope: SkillSourceScope,
    source: &str,
    max_file_bytes: u64,
) -> Option<SkillEntry> {
    let canonical = tokio::fs::canonicalize(skill_md).await.ok()?;
    if !canonical.starts_with(root) {
        tracing::warn!(path = %skill_md.display(), "skipping skill outside source root");
        return None;
    }
    if tokio::fs::metadata(&canonical).await.ok()?.len() > max_file_bytes {
        tracing::warn!(path = %skill_md.display(), "skipping oversized SKILL.md");
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

fn filter_entries(
    entries: Vec<SkillEntry>,
    options: &SkillRuntimeOptions,
) -> (Vec<SkillEntry>, Vec<FilteredSkill>) {
    let allow = options
        .policy
        .allow
        .as_ref()
        .map(|items| normalize_policy_set(items.iter().map(String::as_str)));
    let deny = normalize_policy_set(options.policy.deny.iter().map(String::as_str));
    let chain = SkillPolicyChain::default_chain();

    let mut visible = Vec::new();
    let mut filtered = Vec::new();
    for entry in entries {
        let name = entry.skill.name.clone();
        let source = entry.skill.source.clone();
        let ctx = SkillExposureContext {
            allow: allow.as_ref(),
            deny: &deny,
            options,
        };
        match chain.evaluate(&entry, &ctx) {
            PolicyDecision::Allow | PolicyDecision::AllowFinal => visible.push(entry),
            PolicyDecision::Deny(reason) => filtered.push(FilteredSkill {
                name,
                reason,
                source,
            }),
        }
    }
    (visible, filtered)
}

async fn build_prompt(
    visible: &[SkillEntry],
    options: &SkillRuntimeOptions,
) -> MacacaResult<(String, Vec<SkillEntry>, bool, bool)> {
    let limits = &options.limits;
    let mut prompt_entries: Vec<SkillEntry> = visible
        .iter()
        .take(limits.max_skills_in_prompt)
        .cloned()
        .collect();
    let mut truncated = visible.len() > prompt_entries.len();
    let mut compact = false;

    if let Some(workspace_dir) = &options.workspace_dir {
        project_prompt_entries(&mut prompt_entries, workspace_dir).await?;
    }

    let mut prompt = format_full_prompt(&prompt_entries);
    if prompt.len() > limits.max_skills_prompt_chars {
        compact = true;
        prompt = format_compact_prompt(&prompt_entries);
    }
    while prompt.len() > limits.max_skills_prompt_chars && !prompt_entries.is_empty() {
        prompt_entries.pop();
        truncated = true;
        prompt = format_compact_prompt(&prompt_entries);
    }
    Ok((prompt, prompt_entries, truncated, compact))
}

async fn project_prompt_entries(
    entries: &mut [SkillEntry],
    workspace_dir: &Path,
) -> MacacaResult<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let projection_root = workspace_dir.join("available_skills");
    tokio::fs::create_dir_all(&projection_root).await?;
    let mut used_slugs: HashMap<String, usize> = HashMap::new();

    for entry in entries {
        let base_slug = stable_skill_slug(&entry.skill.name);
        let counter = used_slugs.entry(base_slug.clone()).or_insert(0);
        let slug = if *counter == 0 {
            base_slug
        } else {
            format!("{base_slug}_{counter}")
        };
        *counter += 1;

        let target_dir = projection_root.join(slug);
        if target_dir.exists() {
            tokio::fs::remove_dir_all(&target_dir).await?;
        }
        copy_skill_dir_without_symlinks(&entry.skill.canonical_base_dir, &target_dir).await?;

        // The projected path is the model-facing contract.  The canonical
        // source fields remain untouched so audit and file-policy code can
        // still reason about the original installation path.
        entry.skill.location = target_dir.join("SKILL.md");
        entry.skill.base_dir = target_dir;
    }

    Ok(())
}

fn stable_skill_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_was_separator = false;
        } else if !previous_was_separator && !slug.is_empty() {
            slug.push('_');
            previous_was_separator = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "skill".into()
    } else {
        slug
    }
}

async fn copy_skill_dir_without_symlinks(source: &Path, target: &Path) -> MacacaResult<()> {
    tokio::fs::create_dir_all(target).await?;
    let mut stack = vec![(source.to_path_buf(), target.to_path_buf())];

    while let Some((from_dir, to_dir)) = stack.pop() {
        tokio::fs::create_dir_all(&to_dir).await?;
        let mut children = tokio::fs::read_dir(&from_dir).await?;
        while let Some(child) = children.next_entry().await? {
            let from_path = child.path();
            let file_type = tokio::fs::symlink_metadata(&from_path).await?.file_type();
            if file_type.is_symlink() {
                tracing::warn!(
                    path = %from_path.display(),
                    "skipping symlink while projecting skill directory"
                );
                continue;
            }

            let to_path = to_dir.join(child.file_name());
            if file_type.is_dir() {
                stack.push((from_path, to_path));
            } else if file_type.is_file() {
                tokio::fs::copy(&from_path, &to_path).await?;
            }
        }
    }

    Ok(())
}

fn format_full_prompt(entries: &[SkillEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = skill_prompt_preamble();
    out.push_str("\n<available_skills>\n");
    for entry in entries {
        out.push_str("  <skill>\n");
        out.push_str(&format!(
            "    <name>{}</name>\n",
            escape_xml(&entry.skill.name)
        ));
        out.push_str(&format!(
            "    <description>{}</description>\n",
            escape_xml(&entry.skill.description)
        ));
        out.push_str(&format!(
            "    <location>{}</location>\n",
            escape_xml(&entry.skill.location.display().to_string())
        ));
        out.push_str("  </skill>\n");
    }
    out.push_str("</available_skills>");
    out
}

fn format_compact_prompt(entries: &[SkillEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = skill_prompt_preamble();
    out.push_str("\n<available_skills>\n");
    for entry in entries {
        out.push_str("  <skill>\n");
        out.push_str(&format!(
            "    <name>{}</name>\n",
            escape_xml(&entry.skill.name)
        ));
        out.push_str(&format!(
            "    <location>{}</location>\n",
            escape_xml(&entry.skill.location.display().to_string())
        ));
        out.push_str("  </skill>\n");
    }
    out.push_str("</available_skills>");
    out
}

fn skill_prompt_preamble() -> String {
    [
        "The following skills provide specialized instructions for specific tasks.",
        "Use a skill only when the task matches its name or description.",
        "Before applying a skill, read the SKILL.md file at its location.",
        "When a skill references relative files, resolve them against the skill directory.",
        "Do not assume unlisted skills exist.",
        "",
    ]
    .join("\n")
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Return true when `path` is inside any skill in a snapshot.
pub fn path_belongs_to_snapshot_skill(snapshot: &SkillSnapshot, path: &Path) -> bool {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    snapshot.skills.iter().any(|skill| {
        let base =
            std::fs::canonicalize(&skill.base_dir).unwrap_or_else(|_| skill.base_dir.clone());
        let matches_projected = canonical.starts_with(base);
        let matches_source = if skill.source_base_dir.as_os_str().is_empty() {
            false
        } else {
            let source_base = std::fs::canonicalize(&skill.source_base_dir)
                .unwrap_or_else(|_| skill.source_base_dir.clone());
            canonical.starts_with(source_base)
        };
        matches_projected || matches_source
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::os_matches_current;
    use std::env;

    async fn write_skill(root: &Path, dir: &str, body: &str) {
        let skill_dir = root.join(dir);
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(skill_dir.join("SKILL.md"), body)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn source_precedence_workspace_wins() {
        let app = tempfile::tempdir().unwrap();
        let ws = tempfile::tempdir().unwrap();
        write_skill(
            app.path(),
            "skills/demo",
            "---\nname: demo\ndescription: app\n---\napp",
        )
        .await;
        write_skill(
            ws.path(),
            "skills/demo",
            "---\nname: demo\ndescription: workspace\n---\nws",
        )
        .await;

        let snapshot = SkillRuntime
            .build_snapshot(
                "agent",
                SkillRuntimeOptions {
                    workspace_dir: Some(ws.path().to_path_buf()),
                    app_dir: Some(app.path().to_path_buf()),
                    policy: SkillPolicy {
                        allow: Some(vec!["demo".into()]),
                        deny: Vec::new(),
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(snapshot.skills.len(), 1);
        assert_eq!(snapshot.skills[0].description, "workspace");
    }

    #[tokio::test]
    async fn openclaw_metadata_filters_missing_env() {
        let app = tempfile::tempdir().unwrap();
        write_skill(
            app.path(),
            "skills/needs-env",
            "---\nname: needs-env\ndescription: env\nmetadata:\n  openclaw:\n    requires:\n      env: [MACACA_TEST_MISSING_ENV]\n---\nbody",
        )
        .await;

        let snapshot = SkillRuntime
            .build_snapshot(
                "agent",
                SkillRuntimeOptions {
                    app_dir: Some(app.path().to_path_buf()),
                    policy: SkillPolicy {
                        allow: Some(vec!["needs-env".into()]),
                        deny: Vec::new(),
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(snapshot.skills.is_empty());
        assert_eq!(snapshot.filtered[0].reason, "missing_env");
    }

    #[test]
    fn darwin_metadata_matches_macos_runtime() {
        assert!(os_matches_current("darwin") || env::consts::OS != "macos");
    }

    #[tokio::test]
    async fn allowlist_limits_visible_skills() {
        let app = tempfile::tempdir().unwrap();
        write_skill(
            app.path(),
            "skills/a",
            "---\nname: a\ndescription: A\n---\nA",
        )
        .await;
        write_skill(
            app.path(),
            "skills/b",
            "---\nname: b\ndescription: B\n---\nB",
        )
        .await;

        let snapshot = SkillRuntime
            .build_snapshot(
                "agent",
                SkillRuntimeOptions {
                    app_dir: Some(app.path().to_path_buf()),
                    policy: SkillPolicy {
                        allow: Some(vec!["b".into()]),
                        deny: Vec::new(),
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(snapshot.skills.len(), 1);
        assert_eq!(snapshot.skills[0].name, "b");
    }

    #[tokio::test]
    async fn snapshot_projects_visible_skill_into_workspace_available_skills() {
        let app = tempfile::tempdir().unwrap();
        let ws = tempfile::tempdir().unwrap();
        write_skill(
            app.path(),
            "skills/crypto-market",
            "---\nname: Crypto Market\ndescription: market data\n---\nUse scripts/crypto.py.",
        )
        .await;
        let script_dir = app.path().join("skills/crypto-market/scripts");
        tokio::fs::create_dir_all(&script_dir).await.unwrap();
        tokio::fs::write(script_dir.join("crypto.py"), "print('ticker')\n")
            .await
            .unwrap();

        let snapshot = SkillRuntime
            .build_snapshot(
                "technical_analyst",
                SkillRuntimeOptions {
                    workspace_dir: Some(ws.path().to_path_buf()),
                    app_dir: Some(app.path().to_path_buf()),
                    policy: SkillPolicy {
                        allow: Some(vec!["Crypto Market".into()]),
                        deny: Vec::new(),
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let entry = &snapshot.skills[0];
        let projected_skill = ws.path().join("available_skills/crypto_market/SKILL.md");
        let projected_script = ws
            .path()
            .join("available_skills/crypto_market/scripts/crypto.py");

        assert_eq!(entry.location, projected_skill);
        assert!(entry
            .source_location
            .ends_with("skills/crypto-market/SKILL.md"));
        assert!(projected_skill.exists());
        assert!(projected_script.exists());
        assert!(snapshot
            .prompt
            .contains("available_skills/crypto_market/SKILL.md"));
        assert!(path_belongs_to_snapshot_skill(&snapshot, &projected_script));
        assert!(path_belongs_to_snapshot_skill(
            &snapshot,
            &entry.source_base_dir.join("SKILL.md")
        ));
    }
}
