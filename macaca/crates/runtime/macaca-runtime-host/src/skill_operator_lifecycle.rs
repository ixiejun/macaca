//! Runtime-host implementation of Skill operator lifecycle commands.
//!
//! The provider keeps mutable operator state in this small State/Memento module
//! instead of putting watch, config, and visibility logic in Web/CLI shells.
//! Future Store/EventLog-backed implementations can replace the in-memory maps
//! without changing the provider-neutral `macaca-skill` command contracts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::Utc;
use macaca_proto::{ServiceError, ServiceResult};
use macaca_skill::{
    AgentSkill, SkillOperatorCatalogEntry, SkillOperatorCatalogListCommand,
    SkillOperatorCatalogListResult, SkillOperatorChangedCommand, SkillOperatorChangedEvent,
    SkillOperatorChangedResult, SkillOperatorConfigWriteCommand, SkillOperatorConfigWriteResult,
    SkillOperatorEnablementCommand, SkillOperatorEnablementResult,
    SkillOperatorMarkdownReadCommand, SkillOperatorMarkdownReadResult,
    SkillOperatorSourceDescriptor, SkillOperatorSourceKind, SkillOperatorUnwatchCommand,
    SkillOperatorWatchCommand, SkillOperatorWatchResult, SkillServicePolicyHints, SkillSourceScope,
};
use tokio::sync::RwLock;

/// Service-owned operator state for config, watch, enablement, and visibility.
#[derive(Debug, Default)]
pub(crate) struct SkillOperatorLifecycle {
    state: RwLock<SkillOperatorState>,
}

#[derive(Debug, Default)]
struct SkillOperatorState {
    configs: BTreeMap<String, BTreeMap<String, String>>,
    enabled: BTreeMap<String, bool>,
    watched_sources: BTreeSet<String>,
    visibility_generation: u64,
    audit_sequence: u64,
}

impl SkillOperatorLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    /// List catalog rows from typed source descriptors using deterministic
    /// service-owned precedence.  The first source that declares a skill name
    /// wins; lower-priority duplicates are counted as shadowed diagnostics.
    pub async fn catalog_list(
        &self,
        command: SkillOperatorCatalogListCommand,
    ) -> ServiceResult<SkillOperatorCatalogListResult> {
        ensure_policy_allows(&command.policy)?;
        let mut sources = command.sources.clone();
        sources.sort_by_key(|source| source.kind);

        let mut rows = BTreeMap::new();
        let mut shadowed = 0usize;
        for source in &sources {
            for skill in scan_source(source).await? {
                if rows.contains_key(&skill.name) {
                    shadowed += 1;
                    continue;
                }
                let enabled = self.is_enabled(&skill.name).await;
                if !enabled && !command.include_disabled {
                    continue;
                }
                rows.insert(
                    skill.name.clone(),
                    SkillOperatorCatalogEntry {
                        name: skill.name.clone(),
                        description: bounded_text(&skill.description, 512),
                        source_kind: source.kind,
                        source_label: source.kind.as_str().into(),
                        location: skill.location.clone(),
                        enabled,
                        provenance_ref: provenance_ref(source.kind, &skill.location),
                    },
                );
            }
        }
        let state = self.state.read().await;
        tracing::info!(
            trace_id = %command.trace.trace_id,
            source_count = sources.len(),
            skill_count = rows.len(),
            skipped_shadowed = shadowed,
            visibility_generation = state.visibility_generation,
            "skill operator catalog listed"
        );
        Ok(SkillOperatorCatalogListResult {
            skills: rows.into_values().collect(),
            source_count: sources.len(),
            skipped_shadowed: shadowed,
            visibility_generation: state.visibility_generation,
            captured_at: Utc::now(),
        })
    }

    /// Read one markdown body with a strict byte bound so operator surfaces do
    /// not accidentally serialize unbounded skill instructions.
    pub async fn markdown_read(
        &self,
        command: SkillOperatorMarkdownReadCommand,
    ) -> ServiceResult<SkillOperatorMarkdownReadResult> {
        ensure_policy_allows(&command.policy)?;
        if command.max_bytes == 0 {
            return Err(ServiceError::InvalidArgument(
                "skill markdown read requires max_bytes > 0".into(),
            ));
        }
        let skill = self
            .find_skill(&command.skill_name, &command.sources)
            .await?
            .ok_or_else(|| ServiceError::InvalidArgument("skill not found".into()))?;
        let activated = skill.load_content().await.map_err(service_adapter_error)?;
        let (markdown, bytes_read, truncated) =
            bound_markdown(&activated.content, command.max_bytes);
        tracing::info!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            bytes_read,
            truncated,
            "skill operator markdown read completed"
        );
        Ok(SkillOperatorMarkdownReadResult {
            skill_name: skill.name,
            source_kind: operator_kind_from_scope(skill.source_scope),
            markdown,
            bytes_read,
            truncated,
            provenance_ref: provenance_ref(
                operator_kind_from_scope(skill.source_scope),
                &skill.location,
            ),
            captured_at: Utc::now(),
        })
    }

    /// Persist sanitized config values in the provider memento and return only
    /// redacted values and audit refs to callers.
    pub async fn config_write(
        &self,
        command: SkillOperatorConfigWriteCommand,
    ) -> ServiceResult<SkillOperatorConfigWriteResult> {
        ensure_policy_allows(&command.policy)?;
        let redacted_values = redact_config(command.values);
        let mut state = self.state.write().await;
        state
            .configs
            .insert(command.skill_name.clone(), redacted_values.clone());
        let audit_event_ids = vec![state.next_audit_ref(&command.trace.trace_id, "config")];
        tracing::info!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            key_count = redacted_values.len(),
            audit_events = audit_event_ids.len(),
            "skill operator config persisted"
        );
        Ok(SkillOperatorConfigWriteResult {
            skill_name: command.skill_name,
            redacted_values,
            audit_event_ids,
            captured_at: Utc::now(),
        })
    }

    pub async fn watch(&self, command: SkillOperatorWatchCommand) -> SkillOperatorWatchResult {
        let mut state = self.state.write().await;
        state.watched_sources.insert(source_key(&command.source));
        tracing::info!(
            trace_id = %command.trace.trace_id,
            source_kind = command.source.kind.as_str(),
            watched_sources = state.watched_sources.len(),
            "skill operator source watched"
        );
        SkillOperatorWatchResult {
            watched_sources: state.watched_sources.len(),
            captured_at: Utc::now(),
        }
    }

    pub async fn unwatch(&self, command: SkillOperatorUnwatchCommand) -> SkillOperatorWatchResult {
        let mut state = self.state.write().await;
        state.watched_sources.remove(&source_key(&command.source));
        tracing::info!(
            trace_id = %command.trace.trace_id,
            source_kind = command.source.kind.as_str(),
            watched_sources = state.watched_sources.len(),
            "skill operator source unwatched"
        );
        SkillOperatorWatchResult {
            watched_sources: state.watched_sources.len(),
            captured_at: Utc::now(),
        }
    }

    /// Record a bounded watched change and advance the visibility generation
    /// used by context and tool planning refreshes.
    pub async fn changed(
        &self,
        command: SkillOperatorChangedCommand,
    ) -> ServiceResult<SkillOperatorChangedResult> {
        let mut state = self.state.write().await;
        let source_key = source_key(&command.source);
        if !state.watched_sources.contains(&source_key) {
            return Err(ServiceError::InvalidArgument(
                "skill source is not watched".into(),
            ));
        }
        state.visibility_generation += 1;
        let audit_event_ids = vec![state.next_audit_ref(&command.trace.trace_id, "changed")];
        let changed = SkillOperatorChangedEvent {
            skill_name: command.skill_name.clone(),
            source_kind: command.source.kind,
            change_kind: bounded_text(&command.change_kind, 64),
            sanitized_summary: sanitize_summary(&command.summary),
            captured_at: Utc::now(),
        };
        tracing::info!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            visibility_generation = state.visibility_generation,
            audit_events = audit_event_ids.len(),
            "skill operator change recorded"
        );
        Ok(SkillOperatorChangedResult {
            changed,
            visibility_generation: state.visibility_generation,
            audit_event_ids,
        })
    }

    /// Persist a service-owned enablement decision and advance visibility so
    /// future context/tool planning can observe the latest lifecycle state.
    pub async fn enablement(
        &self,
        command: SkillOperatorEnablementCommand,
    ) -> ServiceResult<SkillOperatorEnablementResult> {
        ensure_policy_allows(&command.policy)?;
        let mut state = self.state.write().await;
        state
            .enabled
            .insert(command.skill_name.clone(), command.enabled);
        state.visibility_generation += 1;
        let audit_event_ids = vec![state.next_audit_ref(&command.trace.trace_id, "enablement")];
        tracing::info!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            enabled = command.enabled,
            visibility_generation = state.visibility_generation,
            reason = %bounded_text(&command.reason, 128),
            "skill operator enablement changed"
        );
        Ok(SkillOperatorEnablementResult {
            skill_name: command.skill_name,
            enabled: command.enabled,
            visibility_generation: state.visibility_generation,
            audit_event_ids,
            captured_at: Utc::now(),
        })
    }

    async fn is_enabled(&self, skill_name: &str) -> bool {
        self.state
            .read()
            .await
            .enabled
            .get(skill_name)
            .copied()
            .unwrap_or(true)
    }

    async fn find_skill(
        &self,
        name: &str,
        sources: &[SkillOperatorSourceDescriptor],
    ) -> ServiceResult<Option<AgentSkill>> {
        let mut sorted = sources.to_vec();
        sorted.sort_by_key(|source| source.kind);
        for source in &sorted {
            for skill in scan_source(source).await? {
                if skill.name == name {
                    return Ok(Some(skill));
                }
            }
        }
        Ok(None)
    }
}

impl SkillOperatorState {
    fn next_audit_ref(&mut self, trace_id: &str, kind: &str) -> String {
        self.audit_sequence += 1;
        format!(
            "audit://skill-operator/{kind}/{trace_id}/{}",
            self.audit_sequence
        )
    }
}

async fn scan_source(source: &SkillOperatorSourceDescriptor) -> ServiceResult<Vec<AgentSkill>> {
    if !source.root.exists() {
        return Ok(Vec::new());
    }
    let mut candidates = Vec::new();
    if source.root.join("SKILL.md").exists() {
        candidates.push(source.root.join("SKILL.md"));
    }
    let mut entries = tokio::fs::read_dir(&source.root)
        .await
        .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?
    {
        let path = entry.path();
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                candidates.push(skill_md);
            }
        }
    }
    candidates.sort();
    let mut skills = Vec::new();
    for path in candidates {
        match AgentSkill::from_path_with_source(
            &path,
            source_scope_from_operator(source.kind),
            source.kind.as_str(),
        )
        .await
        {
            Ok(skill) => skills.push(skill),
            Err(err) => tracing::warn!(
                path = %path.display(),
                error = %sanitize_summary(&err.to_string()),
                "skill operator skipped invalid skill markdown"
            ),
        }
    }
    Ok(skills)
}

fn ensure_policy_allows(policy: &SkillServicePolicyHints) -> ServiceResult<()> {
    if policy.entitlement_ready == Some(false) || policy.package_ready == Some(false) {
        return Err(ServiceError::DisabledByPolicy(
            "skill operator policy denied the command".into(),
        ));
    }
    Ok(())
}

fn source_scope_from_operator(kind: SkillOperatorSourceKind) -> SkillSourceScope {
    match kind {
        SkillOperatorSourceKind::Application => SkillSourceScope::Application,
        SkillOperatorSourceKind::Workspace => SkillSourceScope::Workspace,
        SkillOperatorSourceKind::User => SkillSourceScope::UserAgentGeneric,
        SkillOperatorSourceKind::Managed => SkillSourceScope::MacacaCentral,
        SkillOperatorSourceKind::PluginProvided => SkillSourceScope::Extra,
    }
}

fn operator_kind_from_scope(scope: SkillSourceScope) -> SkillOperatorSourceKind {
    match scope {
        SkillSourceScope::Application | SkillSourceScope::ProjectAgents => {
            SkillOperatorSourceKind::Application
        }
        SkillSourceScope::Workspace => SkillOperatorSourceKind::Workspace,
        SkillSourceScope::MacacaCentral | SkillSourceScope::Bundled => {
            SkillOperatorSourceKind::Managed
        }
        SkillSourceScope::UserAgentGeneric | SkillSourceScope::UserClient => {
            SkillOperatorSourceKind::User
        }
        SkillSourceScope::Extra => SkillOperatorSourceKind::PluginProvided,
    }
}

fn source_key(source: &SkillOperatorSourceDescriptor) -> String {
    format!("{}:{}", source.kind.as_str(), source.root.display())
}

fn provenance_ref(kind: SkillOperatorSourceKind, path: &Path) -> String {
    format!("skill-source://{}/{}", kind.as_str(), path.display())
}

fn service_adapter_error(error: macaca_proto::MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

fn bound_markdown(value: &str, max_bytes: usize) -> (String, usize, bool) {
    let mut output = String::new();
    for ch in value.chars() {
        if output.len() + ch.len_utf8() > max_bytes {
            return (output, max_bytes, true);
        }
        output.push(ch);
    }
    let len = output.len();
    (output, len, false)
}

fn redact_config(values: BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .into_iter()
        .map(|(key, value)| {
            let sensitive = is_sensitive(&key) || is_sensitive(&value);
            (
                key,
                if sensitive {
                    "[REDACTED]".into()
                } else {
                    value
                },
            )
        })
        .collect()
}

fn sanitize_summary(value: &str) -> String {
    if is_sensitive(value) {
        return "[REDACTED]".into();
    }
    bounded_text(value, 240)
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn is_sensitive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "secret",
        "token",
        "password",
        "credential",
        "api_key",
        "apikey",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
