//! Materialized Skill package recovery for the built-in Skill provider.
//!
//! The local provider keeps governance records in memory, but materialized
//! Skill packages can outlive the process.  This module treats each bounded
//! `SKILL.md` file as a recovery Memento: it extracts only frontmatter identity
//! and explicit provenance refs, then re-enters the existing usage event path so
//! the read model remains traceable and service-owned.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAuthorKind, SkillGovernanceRecordUsageCommand, SkillServiceScope, SkillUsageEventKind,
    SkillUsageObservation,
};
use serde::Deserialize;
use tokio::fs;

use crate::skill_service_provider_state::SkillProviderGovernanceState;

const MAX_RECOVERY_CANDIDATES_PER_ROOT: usize = 300;
const MAX_RECOVERY_SKILL_MD_BYTES: u64 = 256_000;

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
}

#[derive(Debug)]
struct MaterializedPackageRecoveryCandidate {
    name: String,
    description: String,
    package_dir: PathBuf,
    proposal_ref: String,
    task_ref: String,
    trace_ref: String,
}

impl SkillProviderGovernanceState {
    /// Rebuild missing governance identities from already materialized packages.
    ///
    /// Recovery is intentionally identity-only. It records a `Created`
    /// observation so later usage telemetry can attach to the same governed
    /// Skill, but it does not attempt to reconstruct historical activation or
    /// task counters from logs. That keeps restart behavior deterministic and
    /// prevents shell/application-specific replay logic from entering the Skill
    /// service boundary.
    pub(crate) async fn recover_materialized_skill_packages(&self, roots: &[PathBuf]) -> usize {
        let mut recovered = 0usize;
        for root in roots {
            let candidates = discover_recovery_candidates(root).await;
            for candidate in candidates {
                if self.recover_materialized_skill_candidate(candidate).await {
                    recovered = recovered.saturating_add(1);
                }
            }
        }
        tracing::info!(
            roots = roots.len(),
            recovered,
            "skill governance package recovery completed"
        );
        recovered
    }

    async fn recover_materialized_skill_candidate(
        &self,
        candidate: MaterializedPackageRecoveryCandidate,
    ) -> bool {
        let skill_id = format!("skill://agent/{}", candidate.name);
        if self.records.lock().await.contains_key(&skill_id) {
            tracing::debug!(
                skill_id = %skill_id,
                package_dir = %candidate.package_dir.display(),
                "skill governance package recovery skipped existing record"
            );
            return false;
        }

        let mut metadata = BTreeMap::new();
        metadata.insert("task_id".into(), candidate.task_ref.clone());
        metadata.insert("trace_id".into(), candidate.trace_ref.clone());
        metadata.insert("evidence_ref.1".into(), candidate.task_ref.clone());
        metadata.insert("evidence_ref.2".into(), candidate.trace_ref.clone());
        metadata.insert("skill.metadata_valid".into(), "true".into());
        let trace = TraceContext::new(format!("skill-package-recovery:{}", candidate.name));
        self.record_usage(SkillGovernanceRecordUsageCommand {
            trace,
            scope: SkillServiceScope::default(),
            observation: SkillUsageObservation {
                skill_id: skill_id.clone(),
                name: candidate.name.clone(),
                source: candidate.package_dir.display().to_string(),
                source_scope: "workspace".into(),
                event: SkillUsageEventKind::Created,
                author_kind: SkillAuthorKind::Agent,
                created_by: Some("skill-package-recovery".into()),
                pinned: None,
                evidence_id: Some(candidate.proposal_ref.clone()),
                metadata,
            },
        })
        .await;

        tracing::info!(
            skill_id = %skill_id,
            skill_name = %candidate.name,
            description_chars = candidate.description.chars().count(),
            package_dir = %candidate.package_dir.display(),
            proposal_ref = %candidate.proposal_ref,
            "skill governance package recovery restored active record"
        );
        true
    }
}

async fn discover_recovery_candidates(root: &Path) -> Vec<MaterializedPackageRecoveryCandidate> {
    let Ok(root) = fs::canonicalize(root).await else {
        tracing::debug!(
            root = %root.display(),
            "skill governance package recovery skipped missing root"
        );
        return Vec::new();
    };
    let mut candidates = Vec::new();
    if let Some(candidate) = load_recovery_candidate(&root.join("SKILL.md")).await {
        candidates.push(candidate);
        return candidates;
    }

    let Ok(mut children) = fs::read_dir(&root).await else {
        tracing::warn!(
            root = %root.display(),
            "skill governance package recovery could not read root"
        );
        return candidates;
    };
    let mut scanned = 0usize;
    while let Ok(Some(child)) = children.next_entry().await {
        if scanned >= MAX_RECOVERY_CANDIDATES_PER_ROOT {
            break;
        }
        let path = child.path();
        if !path.is_dir() || is_hidden_path(&path) {
            continue;
        }
        scanned = scanned.saturating_add(1);
        if let Some(candidate) = load_recovery_candidate(&path.join("SKILL.md")).await {
            candidates.push(candidate);
        }
    }
    candidates
}

async fn load_recovery_candidate(skill_md: &Path) -> Option<MaterializedPackageRecoveryCandidate> {
    let metadata = fs::metadata(skill_md).await.ok()?;
    if !metadata.is_file() || metadata.len() > MAX_RECOVERY_SKILL_MD_BYTES {
        tracing::warn!(
            path = %skill_md.display(),
            bytes = metadata.len(),
            "skill governance package recovery skipped invalid SKILL.md size"
        );
        return None;
    }
    let raw = fs::read_to_string(skill_md).await.ok()?;
    let frontmatter = parse_frontmatter(&raw)?;
    let proposal_ref = extract_provenance_ref(&raw, "Proposal ref")?;
    let task_ref = extract_provenance_ref(&raw, "Task ref")?;
    let trace_ref = extract_provenance_ref(&raw, "Trace ref")?;
    Some(MaterializedPackageRecoveryCandidate {
        name: bounded_identity(frontmatter.name),
        description: bounded_description(frontmatter.description),
        package_dir: skill_md.parent().unwrap_or(skill_md).to_path_buf(),
        proposal_ref: bounded_evidence_ref(proposal_ref),
        task_ref: bounded_evidence_ref(task_ref),
        trace_ref: bounded_evidence_ref(trace_ref),
    })
}

fn parse_frontmatter(raw: &str) -> Option<SkillFrontmatter> {
    let mut parts = raw.splitn(3, "---");
    let _prefix = parts.next()?;
    let yaml = parts.next()?;
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml).ok()?;
    if frontmatter.name.trim().is_empty() || frontmatter.description.trim().is_empty() {
        return None;
    }
    Some(frontmatter)
}

fn extract_provenance_ref(raw: &str, label: &str) -> Option<String> {
    let prefix = format!("- {label}: `");
    raw.lines().find_map(|line| {
        let value = line.trim().strip_prefix(&prefix)?;
        Some(value.trim_end_matches('`').trim().to_string())
    })
}

fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn bounded_identity(value: String) -> String {
    value.trim().chars().take(80).collect()
}

fn bounded_description(value: String) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn bounded_evidence_ref(value: String) -> String {
    value.trim().chars().take(256).collect()
}
