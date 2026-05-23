//! Provider-neutral contracts for safe Skill package mutation.
//!
//! Skill content changes are privileged OS service operations.  This module
//! defines the typed command boundary and deterministic validation rules, while
//! runtime-host providers own concrete package write strategies.  Keeping the
//! rules here prevents Web, CLI, frontend, Context, Task, or application code
//! from learning filesystem mutation semantics.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::{
    SkillOwnershipOperation, SkillOwnershipPolicy, SkillOwnershipPolicyContext,
    SkillServicePolicyHints, SkillServiceScope,
};

/// Maximum payload accepted by the built-in mutation contract.
///
/// The initial limit is intentionally conservative.  Skill packages should
/// evolve through small support files or bounded patches; large binary assets
/// need a package/store provider with explicit retention and quota policy.
pub const MAX_SKILL_MUTATION_BYTES: usize = 64 * 1024;

/// Service command for all governed Skill content mutations.
pub const SKILL_CONTENT_MUTATE_COMMAND: &str = "skill.content.mutate";

/// Mutation operations owned by the Skill service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillContentMutationKind {
    CreateSkill,
    PatchSkill,
    WriteSupportFile,
    RemoveSupportFile,
    ArchiveMaterialization,
    RestoreMaterialization,
}

/// Package ownership classes used by mutation policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillPackageOwnershipClass {
    AgentPrivate,
    CentralUser,
    Tenant,
    Bundled,
    Marketplace,
    ApplicationOwned,
    Paid,
    Encrypted,
    PluginProvided,
}

impl SkillPackageOwnershipClass {
    /// Parse a bounded metadata label into an ownership class.
    ///
    /// Metadata labels are lower-case service policy facts, not package-manager
    /// provider names. Unknown values stay absent so callers fail closed
    /// through their normal policy defaults instead of inventing ownership.
    pub fn from_policy_label(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "agent_private" | "agent-private" => Some(Self::AgentPrivate),
            "central_user" | "central-user" => Some(Self::CentralUser),
            "tenant" => Some(Self::Tenant),
            "bundled" => Some(Self::Bundled),
            "marketplace" => Some(Self::Marketplace),
            "application_owned" | "application-owned" => Some(Self::ApplicationOwned),
            "paid" => Some(Self::Paid),
            "encrypted" => Some(Self::Encrypted),
            "plugin_provided" | "plugin-provided" => Some(Self::PluginProvided),
            _ => None,
        }
    }

    /// Return true when generic agent flows must not mutate active package
    /// bytes without a stronger future override contract.
    pub fn is_protected(&self) -> bool {
        matches!(
            self,
            Self::Bundled
                | Self::Marketplace
                | Self::ApplicationOwned
                | Self::Paid
                | Self::Encrypted
                | Self::PluginProvided
        )
    }
}

/// Structured validation denial for mutation requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillContentMutationDenied {
    pub reason: String,
    pub retryable_with_approval: bool,
}

impl SkillContentMutationDenied {
    /// Build a deterministic denial with bounded operator-facing text.
    pub fn new(reason: impl Into<String>, retryable_with_approval: bool) -> Self {
        Self {
            reason: reason.into(),
            retryable_with_approval,
        }
    }
}

/// Command that requests one policy-gated Skill package mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillContentMutationCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_id: String,
    pub package_root: PathBuf,
    pub relative_path: PathBuf,
    pub kind: SkillContentMutationKind,
    pub ownership: SkillPackageOwnershipClass,
    pub content: Option<Vec<u8>>,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub allow_executable_script_mutation: bool,
    pub policy: SkillServicePolicyHints,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl SkillContentMutationCommand {
    /// Validate command envelope, policy refs, ownership, path, size, encoding,
    /// executable policy, and content-sensitivity before a provider may write.
    pub fn validate(&self) -> Result<(), SkillContentMutationDenied> {
        if self.trace.trace_id.trim().is_empty() {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation requires trace context",
                false,
            ));
        }
        if self.skill_id.trim().is_empty() {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation requires skill id",
                false,
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation requires rationale",
                false,
            ));
        }
        if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation requires evidence references",
                false,
            ));
        }
        if self
            .policy_decision_refs
            .iter()
            .all(|id| id.trim().is_empty())
        {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation requires policy decision references",
                true,
            ));
        }
        if self.policy.package_ready != Some(true) || self.policy.entitlement_ready != Some(true) {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation requires package guard and entitlement approval",
                true,
            ));
        }
        let ownership_context = self.ownership_policy_context();
        let ownership_decision = SkillOwnershipPolicy.evaluate(&ownership_context);
        if let Some(reason) = ownership_decision.denied_reason() {
            return Err(SkillContentMutationDenied::new(reason, true));
        }
        validate_allowed_relative_path(&self.relative_path)?;
        validate_content_shape(self)?;
        Ok(())
    }

    /// Return sanitized policy refs for audit and result DTOs.
    pub fn sanitized_policy_decision_refs(&self) -> Vec<String> {
        sanitized_refs(&self.policy_decision_refs)
    }

    /// Return sanitized evidence refs for audit and result DTOs.
    pub fn sanitized_evidence_ids(&self) -> Vec<String> {
        sanitized_refs(&self.evidence_ids)
    }

    /// Convert mutation command hints into the shared ownership-policy context.
    ///
    /// The command does not trust shell-local approval semantics.  It accepts
    /// only bounded boolean hints that must already have come from policy,
    /// package guard, entitlement, or application-scope services.
    fn ownership_policy_context(&self) -> SkillOwnershipPolicyContext {
        SkillOwnershipPolicyContext {
            ownership: self.ownership.clone(),
            operation: SkillOwnershipOperation::PatchActiveContent,
            automatic_agent_flow: metadata_bool(
                &self.policy.metadata,
                "skill.automatic_agent_flow",
            )
            .unwrap_or(true),
            application_scope_approved: metadata_bool(
                &self.policy.metadata,
                "skill.application_scope_approved",
            )
            .unwrap_or(false),
            mutation_entitlement_granted: metadata_bool(
                &self.policy.metadata,
                "skill.mutation_entitlement_granted",
            )
            .unwrap_or(false),
            explicit_approval_granted: metadata_bool(
                &self.policy.metadata,
                "skill.explicit_approval_granted",
            )
            .unwrap_or(false),
        }
    }
}

/// Result status for content mutation commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillContentMutationStatus {
    Applied,
    Denied,
    Unavailable,
}

/// Result returned by a Skill content mutation provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillContentMutationResult {
    pub status: SkillContentMutationStatus,
    pub skill_id: String,
    pub relative_path: PathBuf,
    pub mutation_kind: SkillContentMutationKind,
    pub mutated: bool,
    pub rollback_memento_ref: Option<String>,
    pub bytes_written: u64,
    pub denied_reason: Option<String>,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub trace_id: String,
    pub captured_at: DateTime<Utc>,
}

impl SkillContentMutationResult {
    /// Build a structured denial without exposing raw content.
    pub fn denied(
        command: &SkillContentMutationCommand,
        denial: SkillContentMutationDenied,
    ) -> Self {
        Self {
            status: SkillContentMutationStatus::Denied,
            skill_id: command.skill_id.clone(),
            relative_path: command.relative_path.clone(),
            mutation_kind: command.kind.clone(),
            mutated: false,
            rollback_memento_ref: None,
            bytes_written: 0,
            denied_reason: Some(denial.reason),
            evidence_ids: command.sanitized_evidence_ids(),
            policy_decision_refs: command.sanitized_policy_decision_refs(),
            trace_id: command.trace.trace_id.clone(),
            captured_at: Utc::now(),
        }
    }
}

fn validate_allowed_relative_path(path: &Path) -> Result<(), SkillContentMutationDenied> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(SkillContentMutationDenied::new(
            "skill content mutation path must be relative",
            false,
        ));
    }
    let components = path.components().collect::<Vec<_>>();
    if components.iter().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(SkillContentMutationDenied::new(
            "skill content mutation path must not traverse outside package root",
            false,
        ));
    }
    match components.first() {
        Some(Component::Normal(name))
            if name.to_str() == Some("SKILL.md") && components.len() == 1 =>
        {
            Ok(())
        }
        Some(Component::Normal(name))
            if matches!(
                name.to_str(),
                Some("references" | "templates" | "scripts" | "assets")
            ) =>
        {
            Ok(())
        }
        _ => Err(SkillContentMutationDenied::new(
            "skill content mutation path must target SKILL.md or an allowed support directory",
            false,
        )),
    }
}

fn validate_content_shape(
    command: &SkillContentMutationCommand,
) -> Result<(), SkillContentMutationDenied> {
    let requires_content = matches!(
        command.kind,
        SkillContentMutationKind::CreateSkill
            | SkillContentMutationKind::PatchSkill
            | SkillContentMutationKind::WriteSupportFile
            | SkillContentMutationKind::RestoreMaterialization
    );
    if requires_content && command.content.is_none() {
        return Err(SkillContentMutationDenied::new(
            "skill content mutation requires bounded content",
            false,
        ));
    }
    if matches!(command.kind, SkillContentMutationKind::RemoveSupportFile)
        && command.relative_path == Path::new("SKILL.md")
    {
        return Err(SkillContentMutationDenied::new(
            "SKILL.md removal requires archive materialization instead of support-file removal",
            true,
        ));
    }
    if is_script_path(&command.relative_path) && !command.allow_executable_script_mutation {
        return Err(SkillContentMutationDenied::new(
            "executable script mutation requires explicit policy approval",
            true,
        ));
    }
    if let Some(content) = command.content.as_ref() {
        if content.len() > MAX_SKILL_MUTATION_BYTES {
            return Err(SkillContentMutationDenied::new(
                "skill content mutation exceeds bounded payload size",
                true,
            ));
        }
        if !is_asset_path(&command.relative_path) {
            let text = std::str::from_utf8(content).map_err(|_| {
                SkillContentMutationDenied::new(
                    "text skill content mutation requires utf-8 encoding",
                    false,
                )
            })?;
            reject_sensitive_text(text)?;
        }
    }
    Ok(())
}

fn reject_sensitive_text(text: &str) -> Result<(), SkillContentMutationDenied> {
    let lower = text.to_ascii_lowercase();
    let sensitive_markers = [
        "private_key",
        "api_key",
        "secret_key",
        "password=",
        "authorization:",
        "-----begin",
        "credential",
        "raw_signature",
    ];
    if sensitive_markers
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Err(SkillContentMutationDenied::new(
            "skill content mutation rejected sensitive-looking content",
            false,
        ));
    }
    Ok(())
}

fn is_script_path(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Normal(name)) if name.to_str() == Some("scripts")
    )
}

fn is_asset_path(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Normal(name)) if name.to_str() == Some("assets")
    )
}

fn sanitized_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect()
}

fn metadata_bool(metadata: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    metadata
        .get(key)
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
}
