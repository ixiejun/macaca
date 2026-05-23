//! Skill alias resolution helpers for capability catalog assembly.
//!
//! The Web shell calls the Skill service for alias decisions, then uses these
//! helpers to reshape catalog metadata.  This keeps redirect semantics owned by
//! `service.skill` while the Context composer receives a compact, body-free
//! catalog that already honors the service-owned decision.

use macaca_context::{
    DeclaredCapabilityDependency, SkillCapabilityCatalog, SkillCapabilityRecord,
    SkillFilterDiagnostic,
};
use macaca_skill::{
    FilteredSkill, SkillAliasKind, SkillAliasResolutionStatus, SkillAliasResolveResult,
    SkillSnapshot, SkillSnapshotEntry,
};

/// Freeze a skill snapshot after applying Skill-service alias decisions.
///
/// Alias resolution is intentionally accepted as service output, not recomputed
/// here.  The Web shell remains an adapter: it calls `service.skill`, keeps the
/// historical snapshot immutable for cache/audit purposes, and only hides
/// source rows from the Context composer when the target skill is already
/// visible in the same frozen snapshot.  That fail-closed rule prevents the
/// shell from fabricating a catalog entry or reading another skill package.
#[must_use]
pub fn catalog_from_snapshot_with_aliases(
    snapshot: &SkillSnapshot,
    aliases: &[SkillAliasResolveResult],
) -> SkillCapabilityCatalog {
    let alias_view = AliasResolvedSkillSnapshot::from_snapshot(snapshot, aliases);

    let entries: Vec<SkillCapabilityRecord> = snapshot
        .skills
        .iter()
        .filter(|e| !alias_view.hidden_sources.contains(&e.name))
        .map(|e| {
            let stable_id = format!("skill:{}:{}", e.source_scope.as_str(), e.name);
            let declared_dependencies: Vec<DeclaredCapabilityDependency> = e
                .mcp_servers
                .iter()
                .map(|m| DeclaredCapabilityDependency {
                    capability_id: format!("mcp_server:{}", m.id),
                    notes: Some(format!(
                        "declared in SKILL.md manifest (transport={})",
                        m.transport
                    )),
                })
                .collect();
            SkillCapabilityRecord {
                stable_id,
                name: e.name.clone(),
                description: e.description.clone(),
                location_ref: e.location.display().to_string(),
                source_label: e.source.clone(),
                source_scope: e.source_scope.as_str().to_string(),
                declared_dependencies,
            }
        })
        .collect();

    let filtered: Vec<SkillFilterDiagnostic> = snapshot
        .filtered
        .iter()
        .map(|f| SkillFilterDiagnostic {
            name: f.name.clone(),
            reason: f.reason.clone(),
        })
        .chain(alias_view.filtered_for_context)
        .collect();

    SkillCapabilityCatalog {
        entries,
        filtered,
        truncated: snapshot.truncated,
    }
}

/// Apply service-owned alias resolutions to a frozen snapshot without reading
/// or fabricating skill content.
pub fn apply_to_snapshot(
    mut snapshot: SkillSnapshot,
    aliases: &[SkillAliasResolveResult],
) -> SkillSnapshot {
    let alias_view = AliasResolvedSkillSnapshot::from_snapshot(&snapshot, aliases);
    if alias_view.hidden_sources.is_empty() {
        return snapshot;
    }

    snapshot
        .skills
        .retain(|entry| !alias_view.hidden_sources.contains(&entry.name));
    snapshot.filtered.extend(
        alias_view
            .filtered_for_runtime
            .into_iter()
            .map(|(name, reason)| FilteredSkill {
                name,
                reason,
                source: "skill_alias".into(),
            }),
    );
    snapshot
}

struct AliasResolvedSkillSnapshot {
    hidden_sources: std::collections::BTreeSet<String>,
    filtered_for_context: Vec<SkillFilterDiagnostic>,
    filtered_for_runtime: Vec<(String, String)>,
}

impl AliasResolvedSkillSnapshot {
    fn from_snapshot(snapshot: &SkillSnapshot, aliases: &[SkillAliasResolveResult]) -> Self {
        let visible_names = snapshot
            .skills
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let mut hidden_sources = std::collections::BTreeSet::<String>::new();
        let mut filtered_for_context = Vec::<SkillFilterDiagnostic>::new();
        let mut filtered_for_runtime = Vec::<(String, String)>::new();

        for alias in aliases.iter().filter(|alias| alias.resolved) {
            let Some(source_name) = alias.requested_name.as_deref() else {
                continue;
            };
            let Some(target_name) = alias.target_name.as_deref() else {
                continue;
            };
            if !visible_names.contains(target_name) {
                tracing::warn!(
                    requested_skill_id = %alias.requested_skill_id,
                    source_name,
                    target_name,
                    "skill alias target was not visible in the frozen snapshot; keeping source row"
                );
                continue;
            }

            let reason = format!(
                "alias_resolved:{}->{} kind={}",
                source_name,
                target_name,
                alias_kind_label(alias)
            );
            hidden_sources.insert(source_name.to_string());
            filtered_for_context.push(SkillFilterDiagnostic {
                name: source_name.to_string(),
                reason: reason.clone(),
            });
            filtered_for_runtime.push((source_name.to_string(), reason));
        }

        Self {
            hidden_sources,
            filtered_for_context,
            filtered_for_runtime,
        }
    }
}

pub fn alias_kind_label(alias: &SkillAliasResolveResult) -> &'static str {
    match &alias.kind {
        Some(SkillAliasKind::Redirect) => "redirect",
        Some(SkillAliasKind::SupersededBy) => "superseded_by",
        Some(SkillAliasKind::AbsorbedInto) => "absorbed_into",
        None => "unknown",
    }
}

pub fn alias_status_label(alias: &SkillAliasResolveResult) -> &'static str {
    match &alias.status {
        SkillAliasResolutionStatus::Miss => "miss",
        SkillAliasResolutionStatus::Redirected => "redirected",
        SkillAliasResolutionStatus::WarnAndRedirected => "warn_and_redirected",
        SkillAliasResolutionStatus::Denied => "denied",
        SkillAliasResolutionStatus::Expired => "expired",
        SkillAliasResolutionStatus::LoopPrevented => "loop_prevented",
    }
}

pub fn governed_skill_id(entry: &SkillSnapshotEntry) -> String {
    format!("skill://{}/{}", entry.source_scope.as_str(), entry.name)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use macaca_skill::{
        SkillAliasKind, SkillAliasResolutionPolicy, SkillAliasResolutionStatus,
        SkillAliasResolveResult, SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry,
        SkillSourceScope,
    };

    use super::*;

    #[test]
    fn skill_capability_catalog_applies_service_owned_alias_resolution() {
        let snapshot = SkillSnapshot {
            agent: "architect".into(),
            prompt: "".into(),
            skills: vec![
                SkillSnapshotEntry {
                    name: "old-debugging".into(),
                    description: "Narrow deprecated debugging skill".into(),
                    source_location: PathBuf::from("/app/skills/old-debugging/SKILL.md"),
                    source_base_dir: PathBuf::from("/app/skills/old-debugging"),
                    location: PathBuf::from("/app/skills/old-debugging/SKILL.md"),
                    base_dir: PathBuf::from("/app/skills/old-debugging"),
                    source: "app".into(),
                    source_scope: SkillSourceScope::Application,
                    primary_env: None,
                    required_env: vec![],
                    install: vec![],
                    mcp_servers: vec![],
                },
                SkillSnapshotEntry {
                    name: "debugging".into(),
                    description: "Umbrella debugging skill".into(),
                    source_location: PathBuf::from("/app/skills/debugging/SKILL.md"),
                    source_base_dir: PathBuf::from("/app/skills/debugging"),
                    location: PathBuf::from("/app/skills/debugging/SKILL.md"),
                    base_dir: PathBuf::from("/app/skills/debugging"),
                    source: "app".into(),
                    source_scope: SkillSourceScope::Application,
                    primary_env: None,
                    required_env: vec![],
                    install: vec![],
                    mcp_servers: vec![],
                },
            ],
            filtered: vec![],
            truncated: false,
            compact: true,
            version: 1,
        };
        let alias = SkillAliasResolveResult {
            requested_skill_id: "skill://application/old-debugging".into(),
            requested_name: Some("old-debugging".into()),
            resolved: true,
            status: SkillAliasResolutionStatus::Redirected,
            target_skill_id: Some("skill://application/debugging".into()),
            target_name: Some("debugging".into()),
            kind: Some(SkillAliasKind::AbsorbedInto),
            resolution_policy: Some(SkillAliasResolutionPolicy::Redirect),
            rationale: Some("absorbed into umbrella debugging skill".into()),
            evidence_ids: vec!["evidence://alias/debugging".into()],
            captured_at: Utc::now(),
        };

        let cat = catalog_from_snapshot_with_aliases(&snapshot, &[alias]);

        assert_eq!(cat.entries.len(), 1);
        assert_eq!(cat.entries[0].name, "debugging");
        assert_eq!(cat.filtered.len(), 1);
        assert_eq!(cat.filtered[0].name, "old-debugging");
        assert!(cat.filtered[0].reason.contains("absorbed_into"));
        assert!(cat.filtered[0].reason.contains("debugging"));
    }

    #[test]
    fn skill_capability_catalog_derives_stable_ids_and_mcp_dependencies() {
        let snapshot = SkillSnapshot {
            agent: "architect".into(),
            prompt: "".into(),
            skills: vec![SkillSnapshotEntry {
                name: "figma-mcp".into(),
                description: "Figma context".into(),
                source_location: PathBuf::from("/app/skills/figma-mcp/SKILL.md"),
                source_base_dir: PathBuf::from("/app/skills/figma-mcp"),
                location: PathBuf::from("/app/skills/figma-mcp/SKILL.md"),
                base_dir: PathBuf::from("/app/skills/figma-mcp"),
                source: "app".into(),
                source_scope: SkillSourceScope::Application,
                primary_env: None,
                required_env: vec![],
                install: vec![],
                mcp_servers: vec![SkillMcpServerConfig {
                    id: "figma".into(),
                    command: "npx".into(),
                    args: vec![],
                    transport: "stdio".into(),
                    tool_prefix: None,
                }],
            }],
            filtered: vec![],
            truncated: false,
            compact: true,
            version: 1,
        };

        let cat = catalog_from_snapshot_with_aliases(&snapshot, &[]);
        assert_eq!(cat.entries.len(), 1);
        let row = &cat.entries[0];
        assert_eq!(row.stable_id, "skill:application:figma-mcp");
        assert_eq!(row.declared_dependencies.len(), 1);
        assert_eq!(
            row.declared_dependencies[0].capability_id,
            "mcp_server:figma"
        );
        assert!(row.declared_dependencies[0]
            .notes
            .as_ref()
            .expect("transport note")
            .contains("stdio"));
    }
}
