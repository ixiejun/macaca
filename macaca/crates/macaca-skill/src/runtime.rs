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
        let (prompt, prompt_entries, truncated, compact) = build_prompt(&visible, &options.limits);

        Ok(SkillSnapshot {
            agent,
            prompt,
            skills: prompt_entries
                .into_iter()
                .map(|entry| SkillSnapshotEntry {
                    name: entry.skill.name.clone(),
                    description: entry.skill.description.clone(),
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

fn build_prompt(
    visible: &[SkillEntry],
    limits: &SkillRuntimeLimits,
) -> (String, Vec<SkillEntry>, bool, bool) {
    let mut prompt_entries: Vec<SkillEntry> = visible
        .iter()
        .take(limits.max_skills_in_prompt)
        .cloned()
        .collect();
    let mut truncated = visible.len() > prompt_entries.len();
    let mut compact = false;

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
    (prompt, prompt_entries, truncated, compact)
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
        canonical.starts_with(base)
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
}
